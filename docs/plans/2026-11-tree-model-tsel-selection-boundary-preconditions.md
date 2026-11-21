# Tree model `tsel` — Selection operations check the spec's tree preconditions (per-PR plan-memo)

Rev 1.1: PM 891 scope (addRange step 1 confirmed; containsNode root clause and getRangeAt document-tree clause in;
four neighbours → slots) + widened umbrella erratum.
Rev 1.1 also marks every Selection API quote taken from webref's raw cache (PM 896; §3 lookups).
Rev 1.2: PM 897 (Selection attributes' document-tree clause in; P-13 cells spec-valued; P-14 → widened slot).
Rev 1.3: plan-review R1 fixes K1–K6 (0/7/21/0); decision record
`/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/tree-model/tsel-r1-decisions.md` (the "R1 decision record"
below); PM 901 ratifies P-15.
Rev 1.4: Step 4.5 re-check fixes M1–M11 (A2 0/0/3, A4 0/1/4, A5 0/1/2); full citation audit §3.2 (PM); decision
record `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/tree-model/tsel-r13-recheck-decisions.md` (the "re-check
decision record" below); PM 903 (slots for P-17 / P-18), PM 905 (P-20 folded).

Slice **`tsel`** of the umbrella
[2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(order tp → **tsel** → tq → ta, PM 884; scope PM 885, widened by PM 890 and PM 891). Slot
`#11-selection-boundary-setter-skips-root-check` (registered 2026-11-21, PM 884; owner L3 DOM/form); the older slot
`#11-selection-document-membership` (registered 2026-11-05) is folded into it and closes with it (PM 890). Base:
`origin/main` `3cbd5b88`; every code fact below was measured there on 2026-11-21. Line numbers move, the commands
do not. `origin/main` has since moved to `6cbe39e3` (#655, comment-only in the files this plan touches:
`git diff --stat 3cbd5b88 6cbe39e3 -- <each file listed in §5.2>` changes wording only, no line count;
`vm/host_data/mod.rs` 709 → 708 lines, so its cites below `:303` are one lower there); the code PR re-measures at
its own base. Owner lane: L3 DOM/form. Security-relevant: closes the silent production paths by which the document's
Selection mutates shadow-tree content through `deleteFromDocument` (probes 1e, 4a).

Stated in the umbrella and only referred to here: the tree model (§1), the census (§6), the `tq` slice and its walk
(§1.6, §5 `tq`). This memo adds no rule to the umbrella; its umbrella edits are the erratum parts of §0.4.

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
   this, abort these steps"; webref 外 (raw cache, selection-api 2026-06-11)) and its sibling on the same seam, `addRange()` step 1 ("If the root of the range's
   boundary points are not the document associated with this, abort these steps"; webref 外 (raw cache, selection-api 2026-06-11)). Today `select_all_children`
   (`:424`) checks the node document only and `add_range` (`:455`) compares `Range::owner_document` only, so a node
   or a Range inside a shadow tree, or detached, is accepted (probes 2a-2f). `addRange` is not in PM 885's list; it
   is added here because it is a boundary setter with the same precondition (§0.5), confirmed PM 891 (§11 P-10).
3. **The shadow-including document check, with its step order and its abort.** `collapse()` step 4: "If document
   associated with this is not a shadow-including inclusive ancestor of node, abort these steps."; `extend()` step
   1: "If the document associated with this is not a shadow-including inclusive ancestor of node, abort these
   steps."; `setBaseAndExtent()` step 2: "If document associated with this is not a shadow-including inclusive
   ancestor of anchorNode or focusNode, abort these steps." (each: webref 外 (raw cache, selection-api 2026-06-11);
   cache lines 475, 537, 598). Today `same_document` (`:613-618`) compares the node document, so a
   detached node is accepted (probes 3a, 3c, 3e), and a node of another document is rejected with
   `SelectionError::WrongDocument` (`:125`), which the native throws as "WrongDocumentError" (probe 3h). The spec
   aborts: no exception, no state change, no `selectionchange`. The check also moves to its spec position in each
   method's step order (§4.1), which changes which exception wins (probes 3f, 3h, D1, D3, D5). This facet is the
   defect of the folded slot `#11-selection-document-membership` (§8 L-5).
4. **`deleteFromDocument()`'s document-tree precondition** (PM 890): "The method must invoke deleteContents() on
   this's range if this is not empty and both focus and anchor are in the document tree. Otherwise the method must
   do nothing." (webref 外 (raw cache, selection-api 2026-06-11)) Today `SelectionState::delete_from_document` (`:513-546`) checks emptiness only (`:518`) and its
   native (`delete_selection_contents`, `dom_selection_mutation.rs:121-149`) checks nothing and always schedules
   `selectionchange` (`:147`), so a selection lying wholly inside a shadow tree, which facet (3) keeps legal,
   deletes shadow-tree content (probe 4a: `st` "SHADOW" → "DOW"; spec: nothing).
5. **`containsNode()`'s root clause** (PM 891; the same predicate as facet (2), on a read path): "The method must
   return `false` if this is empty or if node's root is not the document associated with this." (webref 外 (raw cache, selection-api 2026-06-11)) Today
   `contains_node` (`:552`) gates on `same_document` (`:564`), the node document, so with `L` selected
   `containsNode(st)` is `true` for a Text inside a closed shadow tree under `L` (probe N2; spec `false`), and a
   detached node of the document passes the gate too. It takes `root_is_document`; `same_document` has no caller
   left and is deleted (§4.1).
6. **`getRangeAt()`'s document-tree clause** (PM 891; the same predicate as facet (4), on a read path): "The method
   must throw an IndexSizeError exception if index is not 0, or if this is empty or either focus or anchor is not
   in the document tree. Otherwise, it must return a reference to (not a copy of) this's range." (webref 外 (raw cache, selection-api 2026-06-11)) Today the native
   `native_selection_get_range_at` (`dom_selection_proto.rs:600`) checks the index and emptiness itself, in the VM
   host, and nothing else, so `getRangeAt(0)` returns a range lying in a shadow tree (probe N5; spec
   "IndexSizeError"). The three conditions move into one engine-independent method, `SelectionState::range_at`
   (NEW, §4.1; Layering mandate), which the native marshals.
7. **The attributes' document-tree clause** (PM 897; the same predicate as facets (4) and (6), on six read paths;
   decisive reason: `tsel` must not leave `rangeCount` 1 while `getRangeAt(0)` throws). Each clause, quoted from
   webref's cache (webref 外 (raw cache, selection-api 2026-06-11)):
   - `anchorNode`: "The attribute must return the anchor node of this, or `null` if the anchor is null or anchor is
     not in the document tree." (webref 外 (raw cache, selection-api 2026-06-11))
   - `anchorOffset`: "The attribute must return the anchor offset of this, or `0` if the anchor is null or anchor is
     not in the document tree." (webref 外 (raw cache, selection-api 2026-06-11))
   - `focusNode`: "The attribute must return the focus node of this, or `null` if the focus is null or focus is not
     in the document tree." (webref 外 (raw cache, selection-api 2026-06-11))
   - `focusOffset`: "The attribute must return the focus offset of this, or `0` if the focus is null or focus is not
     in the document tree." (webref 外 (raw cache, selection-api 2026-06-11))
   - `rangeCount`: "The attribute must return `0` if this is empty or either focus or anchor is not in the document
     tree, and must return `1` otherwise." (webref 外 (raw cache, selection-api 2026-06-11))
   - `type`: "The attribute must return `"None"` if this is empty or either focus or anchor is not in the document
     tree, `"Caret"` if this's range is collapsed, and `"Range"` otherwise." (webref 外 (raw cache, selection-api
     2026-06-11))

   Today none of them checks it: `SelectionState::range_count` (`selection.rs:165`) reads `active_range_id` only,
   `selection_type` (`:188`) the range's collapsed flag only, and the four node / offset natives
   (`dom_selection_proto.rs:489-537`) return the concepts `anchor` / `focus` (`:223`, `:241`) unfiltered (probes
   A1-A6: a selection inside the shadow tree reads `st`, its offsets, 1 and "Range"). The clause moves into
   `SelectionState` (§4.1): two point accessors (NEW) that return the anchor / focus only when it is in the
   document tree, and `range_count` / `selection_type` reading one helper (NEW) shared with facets (4) and (6);
   the natives marshal only. `anchor` / `focus` stay as the spec's concepts (§2 Definition: they "need not to be
   in the document tree"), read by `extend` and the new helpers.

   One consequence on a write path, by spec text: `addRange()` step 2 is "If `rangeCount` is not `0`, abort these
   steps." (webref 外 (raw cache, selection-api 2026-06-11)), so a selection inside a shadow tree (`rangeCount` 0
   by this facet) no longer blocks `addRange()`: step 3 replaces it (probe A8; §0.5, §11 P-15, ratified PM 901).
   `add_range` reads `range_count` for step 2, and its native goes through `mutate_selection`, so the displaced
   range is freed by the same code as every other replacement, and only when no wrapper holds it (§4.2; proven by
   J16 and mutant M23, not asserted).

8. **The third write source (R1 decision record K1).** Besides the Selection's own setters and `addRange`, a script
   writes the selection's range through Range methods on the wrapper it holds: `getRangeAt(0)` "must return a
   reference to (not a copy of) this's range", `addRange(r)` stores the script's `r` (step 3, "by a strong
   reference"), and §2's note says "if … a script modifies the boundary points of the range, the same range object
   must continue to be associated with the selection" (all three: webref 外 (raw cache, selection-api 2026-06-11);
   cache lines 319-323, 348, 135-142). The Range natives write through `Range::set_start_to_boundary` /
   `set_end_to_boundary` (`range_proto.rs:552`, `:567`, under `write_range`, `:347`), so DOM *set the start or end*
   step 4 keeps the range one-rooted, but the root can be any tree: `r = getRangeAt(0); r.setStart(st, 0)` leaves
   the selection's range collapsed at (`st`, 0) inside the shadow tree, and a detached node or a node of another
   document does the same in its own tree. `tsel` adds no check on this path (DOM puts none there); facets (4)-(7)
   read the result: anchor and focus are not in the document tree, so the attributes read `null` / `0` / `"None"`,
   `rangeCount` 0, `getRangeAt(0)` throws, `deleteFromDocument()` does nothing, and `addRange()` replaces the range
   (facet 7) while the script's wrapper keeps the displaced range alive (§4.2). Cells E13, J15, J16.

### §0.2 Out of scope (and whose it is)

- **The Range walk.** `next_in_preorder_global` (`range/mod.rs`), `Range::to_string` and `Range::delete_contents`
  (`range/mutation.rs`) are not touched: they are `tq`'s (umbrella §5 `tq`). `tq`'s cross-root Selection
  `toString` cell expects `""` on `tsel`'s base (§2 TQ × SEL).
- **Selection direction and `selectAllChildren()`'s child count** (slot `#11-selection-select-all-children-steps`,
  widened by PM 897 to "Selection direction steps (selectAllChildren / extend / setBaseAndExtent set forwards on a
  collapsed range; current_direction is the single site) + selectAllChildren child count"; §11 P-14). Step 7 of
  `selectAllChildren()` sets *forwards*; `select_all_children` sets `Directionless` (probe N4: `"directionless"`,
  spec `"forward"`), and `current_direction` (`:204`) returns `Directionless` for every collapsed range, where
  `extend()` and `setBaseAndExtent()` set *forwards*. Step 3 takes "the number of children of node"
  (webref 外 (raw cache, selection-api 2026-06-11)); `select_all_children` takes `node_length` (`:437`), which is a
  Text node's data length (read from the code, not probed). Not preconditions; `tsel` changes no direction. The
  tests that pin today's `"none"` / `Directionless` are the slot's and are left as they are (§7.2): `tests_selection.rs`
  `extend_collapsed_back_to_anchor_is_directionless` (`:311`) and `set_base_and_extent_collapsed_is_directionless`
  (`:379`); `selection_tests.rs` `set_base_and_extent_collapsed_is_directionless` (`:184`) and
  `select_all_children_makes_range` (`:110`). No cell of this plan asserts a direction (§7).
- **`collapse(null)`** (slot `#11-selection-collapse-null-removes-all-ranges`). Step 1 behaves as
  `removeAllRanges()`; the native's `arg_node_required` (`dom_selection_proto.rs:385`) throws "TypeError" (probe
  N1). WebIDL nullable conversion in the VM, not a precondition of `SelectionState`.
- **`selectionchange` for an operation that is a spec no-op, and for a Range method on the selection's range**
  (slot `#11-selection-change-on-noop-range-mutation`, widened by PM 903 and PM 905). Facet (a), a spec no-op must
  not schedule it: a `deleteFromDocument` that runs `deleteContents()` on a collapsed range (DOM `deleteContents()`
  step 1 returns, so no boundary point is mutated), and `removeAllRanges()` / `empty()` on an empty selection
  (nothing is dissociated). Today the native schedules the event in both; `tsel` keeps that, and drops the event
  only for its own aborts and for `deleteFromDocument()`'s "do nothing" branch (§4.2). Facet (b), a content-script
  mutation of the associated range's boundary point must schedule it: the Range natives (through `write_range`)
  schedule none today and still none after `tsel` (§11 P-18).
- **The reads without the clause.** `isCollapsed`: "The attribute must return true if and only if the anchor and
  focus are the same (including if both are null). Otherwise it must return false."; the stringifier: "The
  stringification must return the string, which is the concatenation of the rendered text if there is a range
  associated with this."; `direction` (empty or directionless → `"none"`) (all three:
  webref 外 (raw cache, selection-api 2026-06-11)). None of them names the document tree, so a selection inside
  a shadow tree keeps its `isCollapsed` and `toString()` (probe A1: `false`, `"SHA"`), and `tsel` changes neither;
  the cells use them as the spec-fixed reads beside the attributes of facet (7).
- **Cross-root order.** `compare_points` (`range/mod.rs:508`) answers a cross-root query in shadow-including order
  (it climbs through `is_ancestor_or_self` and `tree_order_cmp`); its relation is `tr`'s and `tc`'s (umbrella §5).
  `tsel` adds no cross-root order query; the two it inherits are Q-1.
- **`insertNode`'s transient Range** (`range_proto_mutation.rs:158-160`, raw writes on one node): one point, one
  root, not a Selection write.

### §0.3 The declared behaviour change

One: **Selection operations check the spec's tree preconditions** (PM 890; PM 885's "Selection's boundary setters
apply the spec's preconditions" plus `deleteFromDocument()`'s; PM 891 adds `addRange()` step 1 (confirmed),
`containsNode()`'s root clause and `getRangeAt()`'s document-tree clause; PM 897 adds the attributes'
document-tree clause). Observable results, all from the Selection API (§2, §3, §6.2) and DOM text quoted in §3:

1. A cross-root `setBaseAndExtent` or `extend` leaves a collapsed selection: `toString()` is `""`, `isCollapsed`
   is `true`, and `deleteFromDocument()` removes nothing. After `extend` the caret is at the new focus (step 5).
2. `selectAllChildren(n)` and `addRange(r)` with `n` / `r` inside a shadow tree or detached do nothing.
3. `collapse` / `setPosition`, `extend` and `setBaseAndExtent` with a node that is not in the document,
   shadow-including (detached, or another document's), do nothing and throw nothing. The exceptions that remain
   come in the spec's step order (§4.1).
4. `deleteFromDocument()` does nothing unless both anchor and focus are in the document tree, read as the tree
   whose root is the document associated with the selection (§0.5, rung 2; not "any document"): a selection
   inside a shadow tree, or moved by a Range method into a detached tree or another document (§0.1 item 8), is
   kept and nothing is deleted.
5. A setter or `addRange()` that aborts, and a `deleteFromDocument()` that does nothing, schedule no
   `selectionchange` (Selection API §6.2: the event is scheduled when "the selection is dissociated with its range,
   associated with a new range, or the associated range's boundary point is mutated" (webref 外 (raw cache,
   selection-api 2026-06-11); cache lines 1043-1047); an abort does none of these). Not narrowed further: the five
   methods without an abort (`removeRange`, `removeAllRanges`, `empty`, `collapseToStart`, `collapseToEnd`) and a
   `deleteFromDocument()` that runs `deleteContents()` on a collapsed range keep today's event, the empty-selection
   `removeAllRanges()` / `empty()` and the collapsed `deleteContents()` included (§0.2; slot
   `#11-selection-change-on-noop-range-mutation` facet (a), widened by PM 905); and a Range method on the
   selection's range schedules none today and still none (the same slot's facet (b), PM 903; §11 P-18).
6. `containsNode(n)` is `false` when `n`'s root is not the document (a node in a shadow tree, or detached).
7. `getRangeAt(0)` throws "IndexSizeError" when anchor or focus is not in the document tree (a selection inside a
   shadow tree).
8. For such a selection `anchorNode` / `focusNode` are `null`, `anchorOffset` / `focusOffset` are `0`, `rangeCount`
   is `0` and `type` is `"None"`; `isCollapsed` and `toString()` are unchanged (§0.2). So `rangeCount`, `type`,
   `getRangeAt(0)` and `deleteFromDocument()` agree on one condition.
9. `addRange(r)` on such a selection, with `r` rooted at the document, replaces it (step 2 reads `rangeCount`, now
   `0`) and schedules `selectionchange` (Selection API §6.2: the selection is "associated with a new range" (webref 外 (raw
   cache, selection-api 2026-06-11); cache line 1044)). PM 901 ratifies this (§11 P-15). A displaced range that a script's wrapper holds
   stays registered and live (§0.1 item 8, J16).
10. A selection whose range a script moved out of the document tree through a Range method (§0.1 item 8) reads as
    in items 4, 7 and 8 (attributes `null` / `0` / `"None"`, `getRangeAt(0)` throws, `deleteFromDocument()` does
    nothing), whether the range now lies in a shadow tree, a detached tree or another document.

### §0.4 Ratified-surface items (reviewed in this plan's plan-review)

The plan PR carries one erratum to the umbrella (PM 884, widened by PM 891), in the five parts below and nowhere
else. Each part is reviewed here as a ratified-surface edit; each quote is the umbrella's text after the edit.
Deciding record per part: (a) PM 891 (5a) (§11 P-9); (b) and (c) PM 891 (the widening; the umbrella's in-text tag
reads "PM 884", the order decision they follow); (d) PM 884; (e) the PM's instruction to record the erratum in a
Changelog line (§11 P-6), in the form of #647 (`29edf6fc`: `*title* (date; no rev bump; carried by …, branch …)`),
reformatted in Rev 1.3 (R1 decision record K6). How `tq`'s own Changelog entry joins this one is a landing item
(§8 L-13), not part of this erratum.

- **(a)** The Status block's first sentence, a minimal insertion (the rest of the Status text is unchanged; P-9):

  > Status: **umbrella, draft 3.5** (pre-implementation; ratified (PM 848, #648 `4918a6c7`)). Draft 1 was plan-reviewed (R1: CRIT 1 / IMP 21 / MIN 24) and

- **(b)** §5.1's "`tq` first" bullet (was "- `tq` first among the behaviour slices: two accessor swaps that close
  the closed-root exposure of §0.1 item 6"; the bullet's second line is unchanged):

  > - `tsel` then `tq` first among the behaviour slices (erratum, PM 884): `tq`'s two accessor swaps close the closed-root exposure of §0.1 item 6
  > about a week before `ta` (PM 790 (3): `ta` ~12-01; PM 820).

- **(c)** The §5 table's `tq` row, Deps column (was "— (it lands before `ta`, PM 820; no code dependency on
  `tp`)"):

  > tsel (erratum, PM 884) (it lands before `ta`, PM 820; no code dependency on `tp`)

- **(d)** §5.1, after the `tq → ta` bullet; the order line, unchanged since the tsel plan PR's first revision:

  > - tsel → tq (erratum, tsel plan PR, PM 884): the order is tp → tsel → tq → ta; `tsel` ([2026-11-tree-model-tsel-selection-boundary-preconditions.md](2026-11-tree-model-tsel-selection-boundary-preconditions.md), slot `#11-selection-boundary-setter-skips-root-check`) makes Selection operations check the spec's tree preconditions (PM 890), and `tq`'s cross-root Selection `toString` cell expects `""` on that base.

- **(e)** The `**Changelog.**` line directly after the Status block (the umbrella had no changelog line, its
  revision record lives in the Status block, §11 P-6), naming every part:

  > **Changelog.** *Erratum: tsel before tq (PM 884, PM 891)* (2026-11-21; no rev bump; carried by the tsel plan PR, branch `tree-model-tsel-plan`). Status ratified (PM 848, #648 `4918a6c7`); §5.1 slice order tsel before tq and the "`tq` first" sentence; §5 `tq` row Deps.

`tsel` has no row in the §5 table; part (d) is its record there. Not edited here: umbrella §7.4 "At `ta`" item
(5) restates the slice list without `tsel` ("Slices tp → tq → ta → …"); that text is a landing item, applied from
the landing list `elidex-pm-trial/handoff/pending-at-landing/tree-model-umbrella.md`, so `tsel` is added there at
application (§8 L-7), not by a sixth erratum part.

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

Every row selects "include", so it is included and reported; PM 891 confirmed it (§11 P-10). `containsNode` and
`getRangeAt` are reads outside PM 890's list, so the one-declared-change row split for them; PM 891 brought both in
(§0.1 items 5 and 6; Q-2 and Q-3 resolved). PM 897 brought in the attributes' clause (§0.1 item 7; Q-4 resolved).

The choice PM 897 leaves to this plan: how `addRange()` step 2 reads "rangeCount" once the attribute carries the
clause.

| Principle | Step 2 reads `range_count` (the attribute's value) | Step 2 reads `is_empty` (today) |
|---|---|---|
| Spec | **selects**: the step text is "If `rangeCount` is not `0`" | rejects: a second reading of the same name |
| One issue, one way | **selects**: `rangeCount` has one definition, used by the getter and by step 2 | rejects |
| PM 897's decisive reason (no internal inconsistency) | **selects**: `rangeCount` 0 and an `addRange` that aborts as if a range were there would be the same kind of disagreement as `rangeCount` 1 with a throwing `getRangeAt(0)` | rejects |
| Security by structure | **selects**: the replaced range lies outside the document tree: in a shadow tree (set there by a setter), or, moved there by a Range method on the wrapper the script holds (§0.1 item 8), in a shadow tree, a detached tree or another document; the new one is rooted at the document (step 1). A displaced range that a wrapper holds stays registered (`mutate_selection`'s one rule, §4.2), so the script's handle keeps working | — |

Every row selects reading `range_count`, so it was adopted and reported (§11 P-15), and **PM 901 ratified it**:
`addRange()` replaces a selection that lies outside the document tree; its native moves onto `mutate_selection`
(one site for "a Selection write replaced a range"); an evicted `RangeId` is freed only if no wrapper holds it.
Ground (PM 901): Selection API `addRange()` steps 1-2 under the tree clause, and One issue, one way. PM 901's
condition, met in §7: the held-wrapper displacement and the moved-out ranges have cells (E13, J15, J16) and the
unconditional-unregister mutant (M23) is in the table, so "no wrapper holds it" is proven by a cell, not asserted.

The reading of "the document tree" (R1 decision record K2). Facets (4), (6) and (7) say "in the document tree";
the text does not fix which document. Rung (1), spec text, both ways: DOM §4.2.1 defines "A document tree is a node
tree whose root is a document." and "A node is in a document tree if its root is a document." (`body dom
concept-document-tree`, its first and fourth paragraphs; `dfn dom "in a document tree"` → §4.2.1
`#in-a-document-tree`), which says *a* document; the Selection API writes the definite "in the [=document tree=]"
(webref 外 (raw cache, selection-api 2026-06-11); cache line 239, and likewise 249, 259, 269, 289, 298-299, 322,
764), the ReSpec link to the dfn *document tree* (`#concept-document-tree`), not to `#in-a-document-tree`, and
names "the document associated with this" (webref 外 (raw cache, selection-api 2026-06-11); cache lines 634-635,
342, 780-781, where the source marks up "the <a>document</a> associated with [=this=]") where it means that
document (`selectAllChildren()` step 2, `addRange()` step 1, `containsNode()`) but not in these clauses. For the
associated document: the definite article; §2's note "It could be in a shadow tree of the same document."; and
§2's "Every document with a browsing context has a unique selection associated with it." (webref 外 (raw cache,
selection-api 2026-06-11); cache lines 99-100, 179-181). Rung (1) does not decide it. Population checked for
rung (1): the cache holds 8 clauses that name the document tree (`anchorNode`, `anchorOffset`, `focusNode`,
`focusOffset`, `rangeCount`, `type`, `getRangeAt()`, `deleteFromDocument()`) and the §2 note (line 180), 9
occurrences (`/usr/bin/grep -n 'document tree'` over the cache file: 8 lines, plus `type`'s wrapped "[=document
/ tree=]" at 298-299), and 0 links to `#in-a-document-tree` (`/usr/bin/grep -c 'in-a-document-tree'` and
`/usr/bin/grep -c -i 'in a document tree'`: 0 each; control: `/usr/bin/grep -c '\[=document tree=\]'` over the
same file gives 8). Rung (2), elidex principle: **the associated document's tree**, on one deciding ground, the
internal-inconsistency argument: under the any-document reading a selection whose range a Range method moved
into another document (§0.1 item 8; `addRange()` step 1 refuses such a range) would read `rangeCount` 1 while
`containsNode()` is `false` for every node in it (its root is not the associated document), the kind of
disagreement PM 897 ruled out. That `root_is_document` then serves every site is a consequence of the reading,
not a second ground for it. Rung (3) is not reached; no browser was measured.

## §1. Shape and verdict

One PR. `elidex-dom-api` `selection.rs`: four setters reordered and rewritten onto the spec's setters, one
predicate (NEW, private) and the existing shadow-including membership test `focus::is_in_document` (§4.1), the
five methods without an abort return their own "did something" `bool`, `add_range` reads the range's root, `delete_from_document` checks the document tree and
says whether it ran, `contains_node` reads the node's root, `range_at` (NEW) holds `getRangeAt()`'s three
conditions, `range_count` / `selection_type` and two point accessors (NEW) carry the attributes' clause through
one helper (NEW) shared with `delete_from_document` and `range_at`, `add_range` step 2 reads `range_count`, the
`WrongDocument` variant and `same_document` are deleted, and the rewritten doc comments cite the flat Selection
API §3 (§4.1). `elidex-js`: `mutate_selection` and
`delete_selection_contents` schedule `selectionchange` only when the operation did something (each closure passes
the `SelectionState` method's own `bool`; no literal stays in the host), `addRange`'s native
goes through `mutate_selection` instead of reading `owner_document`, `getRangeAt`'s native calls `range_at` in
place of its own checks, the six attribute natives call the new accessors, `map_selection_error` loses its
`WrongDocument` arm. No
algorithm moves into `vm/host/`. Tests: new cells in `selection_tests.rs` and a new VM test file; the VM selection
fixture attaches its tree to the document (§7.2). The slice meets the Edge-dense trigger (≥3 intersecting axes,
§2), so it has this per-PR plan and a full `/elidex-plan-review` before code (§9).

## §2. Coupled invariants (edge-matrix)

Axes: **SEL** (Selection state: `active_range_id`, `direction`), **LRR** (`LiveRangeRegistry`: registration, the
displaced-id unregister in `mutate_selection`, live-range adjustment), **ROOT** (tree root: `find_tree_root`,
`walkers.rs:408`, stops at the `ShadowRoot` marker), **DOC** (document association: the shadow-including
membership test `focus::is_in_document`, `focus/sot.rs:69`; the root is the associated document,
`root_is_document`; node document, `owner_document`, `dom/mod.rs:881`), **ERR** (exception vs abort, the step order
that decides which wins, the native's mapping), **event** (`selectionchange` scheduling), **MUT**
(`deleteFromDocument` → `Range::delete_contents` and its `MutationRecord`s), **ORD** (boundary-point order across
roots), **read** (the read paths of facets 5-7: `containsNode`, `getRangeAt`, the six attributes), **RW** (the
third write source, §0.1 item 8: Range methods on the wrapper a script holds), **TQ** (`tq`'s walk). Rows named
after a slice (`ta × tsel`, `tr × tsel`) are hand-offs to that slice, not axes.

| Pair | Invariant after `tsel` | Cells |
|---|---|---|
| SEL × ROOT | every range the four setters and `addRange` give the Selection has one root (start's root = end's root), and so does every write through a Range method (RW): the Range natives run DOM *set the start or end* step 4, which collapses a cross-root write onto the new point (`range_proto.rs:552`, `:567`) | E1, E2, E13, J1, J2, J15 |
| SEL × DOC | after `collapse` / `extend` / `setBaseAndExtent` the range's shadow-including root is the associated document (a shadow tree of the document is allowed: the spec's note, "anchor and focus … could be in a shadow tree of the same document" (webref 外 (raw cache, selection-api 2026-06-11))); after `selectAllChildren` / `addRange` its root is the document itself. This holds at the Selection's own writes only: a Range method on the wrapper (RW) can move the same range into a shadow tree, a detached tree or another document, and the Selection stays associated with it (§2's note: "the same range object must continue to be associated with the selection"; webref 外 (raw cache, selection-api 2026-06-11)) | E3, E4, E5, E6, E13, J4, J5, J6, J9, J15 |
| DOC × MUT | `deleteFromDocument` reaches `delete_contents` only when the anchor's and the focus's root is the document; so a Selection never deletes inside a shadow tree, a detached tree or another document, whichever setter or Range method made it. With SEL × ROOT, "both" reduces to the start container's root, but the check is written on both points as the spec says | E8, J10 |
| DOC × read (facets 5, 6) | `containsNode(n)` is `false` whenever `n`'s root is not the document, before any boundary comparison; `getRangeAt(0)` throws "IndexSizeError" unless anchor and focus are in the document tree. Both read the same `root_is_document` as facets (2), (4) and (7), so every site shares one predicate | E9, E10, J11, J12 |
| read × read (facet 7) | for every selection, `rangeCount`, `type`, `getRangeAt(0)` and `deleteFromDocument()` read one condition (paraphrased: this is not empty and both anchor and focus are in the document tree; one helper, §4.1): `rangeCount` 1 ⇔ `type` ≠ "None" ⇔ `getRangeAt(0)` returns ⇔ `deleteFromDocument()` may delete; and `anchorNode` is `null` exactly when the anchor is null or outside the document tree (likewise `focusNode`). `isCollapsed` and `toString()` read the concepts and carry no clause (§0.2) | E11, J1, J2, J9, J10, J13 |
| SEL × ERR | an abort returns `Ok(false)` and leaves `active_range_id` and `direction` bit-identical; an exception does too (unchanged rule, `collapse`'s doc comment) | E5, E7, J6 |
| DOC × ERR | which outcome wins when a call is both outside the document and otherwise wrong is the spec's step order, per method (§4.1 table): `collapse` offset before document; `extend` document before emptiness; `setBaseAndExtent` offsets, then document, then DocumentType (DOM step 1 inside step 5). Mutants M9-M11 | E7, J8 |
| ERR × event | a setter or `addRange` that aborts, and a `deleteFromDocument()` that does nothing, schedule no `selectionchange`; one that sets a range, or runs `deleteContents()`, schedules one, as today; the five methods without an abort keep today's unconditional event (§0.3 item 5) | J7, J10, J15 (d) |
| LRR × ERR | an abort registers no Range and unregisters none (`prev_id == new_id` in `mutate_selection`) | E5 (registry untouched), J6 |
| LRR × SEL (`addRange` replace, facet 7; PM 901) | `addRange()` on a selection outside the document tree replaces its range (step 2 reads `rangeCount`, `0`); the displaced id is unregistered only when no wrapper holds it, by `mutate_selection`'s one displaced-id rule (`addRange`'s native now goes through it). The displaced range can be one a setter put in a shadow tree (no wrapper: `getRangeAt(0)` threw, so it is freed) or one a Range method moved out (RW: the script holds its wrapper, so it stays registered and live). A selection in the document tree still blocks `addRange()` (step 2 aborts), as today | E12, J14, J16 |
| LRR × ROOT | live-range adjustment cannot make a one-root range cross-root: DOM's *live range pre-remove steps* (§5.6, run by *remove* step 3, §4.2.3) move a boundary inside the removed node to (parent, index), in the same tree, and *insert* (§4.2.3) moves offsets within the same parent. So the one-root invariant set at write time holds until the next write. Reasoning, not a cell; `extend` reads "this's range's root" from the start container on that basis | — |
| LRR × DOC (facets 4, 6, 7) | live-range adjustment cannot take a selection out of the document tree: removing the subtree that holds a boundary moves the boundary to the removed node's parent, which stays in the document. A selection leaves it by two writes only: a setter that puts it in a shadow tree, or a Range method on the wrapper the script holds (RW: `getRangeAt(0)` hands out "a reference to (not a copy of) this's range" (webref 外 (raw cache, selection-api 2026-06-11); cache lines 322-323), `addRange(r)` keeps the script's `r`), which can move it into a shadow tree, a detached tree or another document. Facet (4)'s "do nothing", facet (6)'s throw and facet (7)'s `null` / `0` / "None" arise from those two, not from a removal | E8, E10, E11, E13, J15 |
| RW × read (§0.1 item 8, K2) | a range moved by a Range method into another document reads as outside the document tree (the associated document's tree, §0.5 rung 2), the same as one moved into a shadow tree or a detached tree; so `rangeCount` 0 and `containsNode()` `false` for its nodes agree (the any-document reading would give `rangeCount` 1 there) | E13, J15 (M24) |
| DOC × ROOT | the two predicates are storage-neutral: `is_in_document` climbs the stored parent, and at base a shadow root's stored parent is its host, so it reaches the document through the host (census R-8: shadow-including, "S, equal" at base); at `ta` its step takes `shadow_including_parent` (census, the R-5 / R-8 row: "K"), so the relation is the same after `ta` and `ts`. `find_tree_root` stops at the `ShadowRoot` marker at every head | E6, J9 (they hold unchanged through `ts`) |
| ROOT × ORD | DOM's boundary-point position asserts one root (§3); `setBaseAndExtent()` step 5 and `extend()` step 9 ask "before" of a pair that can be cross-root. `tsel` asks no new cross-root order question: it keeps `direction_between` (`:624`) as the branch selector, so a cross-root `setBaseAndExtent` collapses at whichever point the branch writes last. Not asserted by any cell (Q-1) | — |
| TQ × SEL | `tsel` touches no file of `tq`'s. After `tsel`, a cross-root Selection is collapsed, so its `toString` is `""` with or without `tq`'s walk; that is the value `tq`'s cell expects (umbrella §5.1 erratum). `tq`'s other Selection cells use `addRange` of a Range built in `body` (the `tq` plan §6), whose root is the document, so `addRange`'s new check and facet (4) accept them | J1 (on both bases) |
| ta × tsel | `tsel` adds callers of two relations `ta` handles: `find_tree_root` (kept, census C-6 "K") and `focus::is_in_document` (census R-8, "K"; its step takes `shadow_including_parent` at `ta`). It adds no caller of `find_tree_root_composed` (Rev 1.3: the shadow-including check reuses `is_in_document`, R1 decision record K6). `ta`'s edits are compiler-driven, so the new callers follow; the census counts change (§8 L-3) | — |
| tr × tsel | `tsel` keeps two cross-root order queries, `direction_between` → `compare_points` in `set_base_and_extent` (step 5's branch) and in `extend`'s direction step (step 9), Q-1; and it edits `contains_node`, which umbrella §2 UP-T × RANGE and its §3 row map to `tr` ("`Selection.containsNode` reads the same boundary comparison", `selection.rs` (tr)). After `tsel`, `contains_node`'s comparison can also be cross-root: a selection whose range a Range method moved into another tree (RW) has its root elsewhere while `node`'s root is the document. If `tr` turns DOM *position* step 1 ("Assert: nodeA and nodeB have the same root.") into code, the cross-root `setBaseAndExtent` branch, `extend`'s step 9 and that `containsNode` call reach it from production; `tr`'s plan must take them (§8 L-6). `tsel`'s cells assert relations, not the collapse point, and no cell asserts the cross-root `containsNode` answer | — |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| SELECTION API §3 Selection interface | `collapse()` steps 2-6: "If node is a DocumentType, throw an InvalidNodeTypeError exception" / "throw an IndexSizeError exception if offset is longer than node's length" / "If document associated with this is not a shadow-including inclusive ancestor of node, abort these steps." / "Set the start the start and the end of newRange to (node, offset)." (webref 外 (raw cache, selection-api 2026-06-11)) | DocumentType → throw; offset > length → throw; not in the document (shadow-including) → abort; else set | `SelectionState::collapse` (`selection.rs:260`) | ✗ (step 1, node null → `removeAllRanges()`: VM nullable conversion, §0.2) | yes |
| SELECTION API §3 Selection interface | `extend()` steps 1, 2, 5-7: "If the document associated with this is not a shadow-including inclusive ancestor of node, abort these steps." / "If this is empty, throw an InvalidStateError exception" / "If node's root is not the same as the this's range's root, set the start newRange's start and end to newFocus." / "Otherwise, if oldAnchor is before or equal to newFocus, set the start newRange's start to oldAnchor, then set its end to newFocus." / "Otherwise, set the start newRange's start to newFocus, then set its end to oldAnchor." (webref 外 (raw cache, selection-api 2026-06-11)) | not in the document → abort; empty → throw; cross-root → collapse at newFocus; same root × forward / backward | `SelectionState::extend` (`selection.rs:329`) | ✗ (step 9, direction: §0.2 and Q-1) | yes |
| SELECTION API §3 Selection interface | `setBaseAndExtent()` steps 1, 2, 5: "If anchorOffset is longer than anchorNode's length or if focusOffset is longer than focusNode's length, throw an IndexSizeError exception" / "If document associated with this is not a shadow-including inclusive ancestor of anchorNode or focusNode, abort these steps." / "If anchor is before focus, set the start the newRange's start to anchor and its end to focus. Otherwise, set the start them to focus and anchor respectively." (webref 外 (raw cache, selection-api 2026-06-11)) | offset → throw; either node not in the document → abort; branch × set the start / end (DocumentType throws there, DOM step 1) | `SelectionState::set_base_and_extent` (`selection.rs:381`) | ✗ (step 7, direction: §0.2; step 5's "before" across roots: Q-1) | yes |
| SELECTION API §3 Selection interface | `selectAllChildren()` steps 1, 2, 4, 5: "If node is a DocumentType, throw an InvalidNodeTypeError exception" / "If node's root is not the document associated with this, abort these steps." / "Set newRange's start to (node, 0)." / "Set newRange's end to (node, childCount)." (webref 外 (raw cache, selection-api 2026-06-11)) | DocumentType → throw; root not the document → abort; else set (plain set, not "set the start") | `SelectionState::select_all_children` (`selection.rs:424`) | ✗ (step 3's childCount and step 7's direction: §0.2) | yes |
| SELECTION API §3 Selection interface | `addRange()` steps 1-3: "If the root of the range's boundary points are not the document associated with this, abort these steps." / "If rangeCount is not 0, abort these steps." / "Set this's range to range by a strong reference" (webref 外 (raw cache, selection-api 2026-06-11)) | root not the document → abort; rangeCount ≠ 0 → abort (`rangeCount` the attribute's value, facet 7: a selection outside the document tree reads 0, so step 3 replaces it); else set | `SelectionState::add_range` (`selection.rs:455`), step 2 through `range_count`; native `native_selection_add_range` (`dom_selection_mutation.rs:155`), through `mutate_selection` | ✓ | yes |
| SELECTION API §3 Selection interface | `deleteFromDocument()`: "The method must invoke deleteContents() on this's range if this is not empty and both focus and anchor are in the document tree. Otherwise the method must do nothing." (webref 外 (raw cache, selection-api 2026-06-11)) | empty → nothing; anchor or focus not in the document tree → nothing; else `deleteContents()` | `SelectionState::delete_from_document` (`selection.rs:513`); native `delete_selection_contents` (`dom_selection_mutation.rs:121`) | ✓ | yes |
| SELECTION API §3 Selection interface | `containsNode()`: "The method must return `false` if this is empty or if node's root is not the document associated with this." / "Otherwise, if allowPartialContainment is `false`, the method must return `true` if and only if start of its range is before or visually equivalent to the first boundary point in the node and end of its range is after or visually equivalent to the last boundary point in the node." (and the `true` branch) (webref 外 (raw cache, selection-api 2026-06-11)) | empty → `false`; root not the document → `false`; else full / partial containment (unchanged) | `SelectionState::contains_node` (`selection.rs:552`); native `native_selection_contains_node` (`dom_selection_proto.rs:629`, unchanged) | ✓ | yes |
| SELECTION API §3 Selection interface | `getRangeAt()`: "The method must throw an IndexSizeError exception if index is not 0, or if this is empty or either focus or anchor is not in the document tree. Otherwise, it must return a reference to (not a copy of) this's range." (webref 外 (raw cache, selection-api 2026-06-11)) | index ≠ 0 → throw; empty → throw; anchor or focus not in the document tree → throw; else this's range (the same wrapper) | `SelectionState::range_at` (NEW); native `native_selection_get_range_at` (`dom_selection_proto.rs:600`) | ✓ | yes |
| SELECTION API §3 Selection interface | `anchorNode` / `anchorOffset` / `focusNode` / `focusOffset`: "The attribute must return the anchor node of this, or `null` if the anchor is null or anchor is not in the document tree." / "… the anchor offset of this, or `0` if the anchor is null or anchor is not in the document tree." / "… the focus node of this, or `null` if the focus is null or focus is not in the document tree." / "… the focus offset of this, or `0` if the focus is null or focus is not in the document tree." (webref 外 (raw cache, selection-api 2026-06-11)) | point null → `null` / `0`; point not in the document tree → `null` / `0`; else the point's node / offset | `SelectionState::anchor_in_document_tree`, `focus_in_document_tree` (NEW); natives `native_selection_get_anchor_node` (`dom_selection_proto.rs:489`), `native_selection_get_anchor_offset` (`:502`), `native_selection_get_focus_node` (`:514`), `native_selection_get_focus_offset` (`:527`) | ✓ | no |
| SELECTION API §3 Selection interface | `rangeCount` / `type`: "The attribute must return `0` if this is empty or either focus or anchor is not in the document tree, and must return `1` otherwise." / "The attribute must return `"None"` if this is empty or either focus or anchor is not in the document tree, `"Caret"` if this's range is collapsed, and `"Range"` otherwise." (webref 外 (raw cache, selection-api 2026-06-11)) | empty → 0 / "None"; anchor or focus not in the document tree → 0 / "None"; else 1 / "Caret" or "Range" | `SelectionState::range_count` (`selection.rs:165`), `selection_type` (`:188`), both through `anchor_and_focus_in_document_tree` (NEW); natives `native_selection_get_range_count` (`dom_selection_proto.rs:551`), `native_selection_get_type` (`:561`) (with the four above, every attribute native of facet (7)) | ✓ | no |
| SELECTION API §3 Selection interface | `isCollapsed` / stringifier (no clause): "The attribute must return true if and only if the anchor and focus are the same (including if both are null). Otherwise it must return false." / "The stringification must return the string, which is the concatenation of the rendered text if there is a range associated with this." (webref 外 (raw cache, selection-api 2026-06-11)) | the concepts, in or out of the document tree | `SelectionState::is_collapsed` (`selection.rs:178`), unchanged; `native_selection_to_string` (`dom_selection_proto.rs:643`), unchanged | ✗ (the stringifier's `textarea` / `input` branch: not this slice; slot `#11-selection-text-control-branches`, PM 903, §11 P-17) | no |
| SELECTION API §2 Definition | *empty*: "When there is no range associated with the selection, the selection is empty"; *anchor* / *focus* note: "anchor and focus of selection need not to be in the document tree. It could be in a shadow tree of the same document."; the selection keeps its range object (Rev 1.3, R1 decision record K1): "Once a selection is associated with a given range, it must continue to be associated with that same range until this specification requires otherwise." and the note "if the DOM changes in a way that changes the range's boundary points, or a script modifies the boundary points of the range, the same range object must continue to be associated with the selection." (webref 外 (raw cache, selection-api 2026-06-11); cache lines 95-181) | empty / not empty; shadow tree of the document allowed; a Range method on the wrapper the script holds (`getRangeAt(0)`'s reference, `addRange`'s `r`) moves the selection's range into the document tree, a shadow tree, a detached tree or another document (one root, DOM *set the start* step 4) | `SelectionState::is_empty`, `anchor`, `focus` (`selection.rs:171`, `:223`, `:241`); the Range natives `native_range_set_start` / `_set_end` (`range_proto.rs:541`, `:557`, writing through `set_start_to_boundary` / `set_end_to_boundary` at `:552`, `:567`), unchanged, their result read by the facet-7 helpers | ✓ | yes |
| SELECTION API §6.2 selectionchange event | `selectionchange` event: "When the selection is dissociated with its range, associated with a new range, or the associated range's boundary point is mutated either by the user or the content script, the user agent must schedule a selectionchange event on document." (webref 外 (raw cache, selection-api 2026-06-11); cache lines 1043-1047) | nothing done → no event; set / deleted → event | `mutate_selection` (`dom_selection_mutation.rs:55`, the pending flag at `:111`); `delete_selection_contents` (`:147`) | ✗ (input / textarea selection: not this slice; slot `#11-selection-text-control-branches`, PM 903, §11 P-17. The spec no-ops that keep today's event, a collapsed `deleteContents()` and `removeAllRanges()` / `empty()` on an empty selection: §0.2, slot `#11-selection-change-on-noop-range-mutation` facet (a), widened by PM 905. A Range method mutating the selection's range: no event today or after; the same slot's facet (b), PM 903, §11 P-18) | yes |
| WHATWG DOM §5.6 Interface Range | *set the start or end of a range to a boundary point* steps 1, 2, 4: "If node is a doctype, then throw an "InvalidNodeTypeError" DOMException." / "If offset is greater than node's length, then throw an "IndexSizeError" DOMException." / set the start: "If range's root is not node's root or boundaryPoint is after range's end, then set range's end to boundaryPoint." / set the end: "If range's root is not node's root or boundaryPoint is before range's start, then set range's start to boundaryPoint." | doctype / offset → throw; cross-root or crossing → collapse onto the new point | `Range::set_start_to_boundary`, `Range::set_end_to_boundary` (`range/boundary.rs:52`, `:67`; steps 1-2 are run by the caller, as their doc comment says) | ✓ | yes |
| WHATWG DOM §5.2 Boundary points | *position* step 1: "Assert: nodeA and nodeB have the same root." | same root → before / equal / after; cross-root → not defined | `compare_points` (`range/mod.rs:508`) through `direction_between` (`selection.rs:624`) | ✗ (the cross-root branch is a spec gap: Q-1) | yes |
| WHATWG DOM §5.6 Interface Range | `deleteContents()` step 1: "If this is collapsed, then return." | collapsed → nothing | `Range::delete_contents` (`range/mutation.rs`, reached from `delete_from_document`, `selection.rs:533`) | ✗ (only the collapsed branch is a `tsel` cell; the rest is `tq`'s) | yes |
| WHATWG DOM §4.2.1 Document tree | *document tree*: "A document tree is a node tree whose root is a document."; *in a document tree*: "A node is in a document tree if its root is a document." Which document the Selection API's "in the document tree" means is not fixed by the text; read as the associated document at rung (2) (§0.5; R1 decision record K2) | a boundary node's root is the associated document → in the document tree; a shadow root, a detached root or another document → not | `root_is_document` (NEW; the associated document) in the facet-7 helpers, `delete_from_document` and `range_at` (NEW); not `element/document_base.rs`'s `in_document_light_tree` (`:226`, root is *a* Document: the any-document reading, §4.1) | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including inclusive ancestor* (with *shadow-including root*): an object that is the node or one of its shadow-including ancestors (parent, or a shadow root's host) | the document is one ⇔ the node's shadow-including root is the document (a document has no parent and no host) | `focus::is_in_document` (`focus/sot.rs:69`, existing; census R-8: the same question, shadow-including), reused in `collapse`, `extend`, `set_base_and_extent` | ✓ | yes |
| WHATWG DOM §1.1 Trees | *root*: "The root of an object is itself, if its parent is null, or else it is the root of its parent." | the root of a shadow-tree node is its shadow root, never the document | `root_is_document` (NEW) over `EcsDom::find_tree_root` (`walkers.rs:408`), in `select_all_children`, `add_range` and `contains_node` | ✓ | yes |

Lookups (2026-11-21, `.claude/tools/webref`): `dfn selection-api "collapse()"` / `"extend()"` /
`"setBaseAndExtent()"` / `"selectAllChildren()"` / `"addRange()"` / `"containsNode()"` / `"getRangeAt()"` /
`"deleteFromDocument()"` / `rangeCount` / `anchorNode` / `anchorOffset` / `focusNode` / `focusOffset` / `type` /
`isCollapsed` / `stringifier` → §3 Selection interface (`#dom-selection-collapse`, `#dom-selection-extend`,
`#dom-selection-setbaseandextent`, `#dom-selection-selectallchildren`, `#dom-selection-addrange`,
`#dom-selection-containsnode`, `#dom-selection-getrangeat`, `#dom-selection-deletefromdocument`,
`#dom-selection-rangecount`, `#dom-selection-anchornode`, `#dom-selection-anchoroffset`, `#dom-selection-focusnode`,
`#dom-selection-focusoffset`, `#dom-selection-type`, `#dom-selection-iscollapsed`, `#dom-selection-stringifier`;
the eight re-run 2026-11-21 for Rev 1.2); `dfn selection-api empty` → §2 Definition `#dfn-empty`; `dfn selection-api
direction` → §2 `#dfn-direction` and §3 `#dom-selection-direction`; `heading selection-api 3` → §3 Selection
interface; `dfn selection-api selectionchange` → §6.2 selectionchange event `#dfn-selectionchange` and `heading
selection-api 6.2` → §6.2 selectionchange event `#selectionchange-event` (Rev 1.4, M1: the event's clause is §6.2,
not §3; the flat-§3 rule of §4.1 is for §3's Selection interface members only). `dfn dom "set the start"` → §5.6 Interface Range `#concept-range-bp-set`, prose from `body dom
concept-range-bp-set`; `dfn dom before` (for boundary point) → §5.2 `#concept-range-bp-before`, prose from `body
dom concept-range-bp-position`; `body dom dom-range-deletecontents` → §5.6; `dfn dom "document tree"` → §4.2.1
`#concept-document-tree`, prose from `body dom concept-document-tree` (Rev 1.3: also `dfn dom "in a document
tree"` → §4.2.1 `#in-a-document-tree`, the fourth paragraph of the same `body dom concept-document-tree`
prose); `dfn dom "shadow-including inclusive
ancestor"` → §4.8 `#concept-shadow-including-inclusive-ancestor`; `dfn dom "shadow-including root"` → §4.8; `dfn
dom root` (for tree) → §1.1 `#concept-tree-root`. `coverage-map dom 5.6 dom 4.8 dom 5.2 selection-api 3
selection-api 2` produced the starter rows. Every § and dfn cite of the memo, these included, is audited one row
each in §3.2 (Rev 1.4). **The Selection API quotes are not from `webref body`**, and each
carries the marker "webref 外 (raw cache, selection-api 2026-06-11)" at its quote site (PM 896; per row in this
table): `webref body selection-api` cannot render the ReSpec source, so they are quoted from webref's cache file
`~/.cache/elidex-webref/c0d97c3dfdf1cb8a59fa73f678d007b67d8cef81` (Last-Modified 2026-06-11 per its `.meta`; the
ReSpec HTML; `<dfn>collapse()</dfn>` at its
line 458, `extend()` 530, `setBaseAndExtent()` 585, `selectAllChildren()` 624, `addRange()` 334,
`deleteFromDocument()` 758, `containsNode()` 775, `getRangeAt()` 315, the attributes `anchorNode` 233,
`anchorOffset` 243, `focusNode` 253, `focusOffset` 263, `isCollapsed` 273, `rangeCount` 283, `type` 293, `direction`
304, the stringifier 804, the definitions 95-180, `selectionchange` 1040). No WebFetch. The memo makes no claim from `webref idl`
(in particular no "not present" answer for an IDL block, which `webref idl` cannot be trusted to give where the
block has multi-line extended attributes, PM 896).

**Breadth**: K=2 specs (DOM, Selection API), M=19 rows (Rev 1.3 widens the §2 Definition row with the range association, K1; no new row). The Selection API rows are "unmapped-label" for preflight
(its label table has no Selection API entry); their citations are the `dfn selection-api` lookups above.
**Split decision**: single PR (a terminal slice of the umbrella).

### §3.1 User-input touch audit

All inputs are author-controlled: the node and offset arguments of the four setters, the Range given to
`addRange` (built with `setStart` / `setEnd` / `selectNode`, any tree a script can make, closed shadow trees
included), and the Range methods a script calls on the selection's own range through the wrapper it holds (§0.1
item 8). Exposure decreases: after `tsel` no Selection range spans two trees, `deleteFromDocument` deletes only
when both anchor and focus are in the document tree, so the document's Selection can no longer reach shadow-tree
content (probes 1e and 4a: today `st` "SHADOW" → "DOW"), and a detached or foreign node can no longer become the
document's selection through a Selection method. It still can through a Range method on the held wrapper (DOM
puts no tree check on `setStart`): such a selection reads as outside the document tree (facets 4-7), so its
content is not deleted and its nodes are not handed out by the attributes. `getRangeAt(0)` no longer hands out a
range lying outside the document tree (facet 6): the Selection gives no *new* Range handle into a shadow tree,
though a handle taken while the range was in the document tree stays the script's, and the script that moved it
already had the nodes. `containsNode` no longer answers `true` for a node inside a closed shadow tree (facet 5). `anchorNode` / `focusNode` no longer return a node inside a shadow tree, closed ones
included (facet 7). What stays exposed is what the spec gives without the clause (§0.2): `toString()` renders the
text of a selection set inside a shadow tree, and `isCollapsed` reflects it; a script can set such a selection
only with the shadow tree's nodes already in hand. The VM natives keep their argument conversions (`arg_node_required`,
`arg_offset_*`, `arg_range`) unchanged; no new input path.

### §3.2 Citation audit (Rev 1.4, re-check decision record M2)

Every spec § number and every spec dfn the memo cites, one row each: the cite as written, where it stands, the
`.claude/tools/webref` lookup (run 2026-11-21 from this worktree; never WebFetch) with the § and title or anchor it
returned, and the verdict. Selection API headings and dfns come from `webref heading` / `dfn selection-api` (only
`body` cannot render it); its step numbers and prose come from the raw cache and carry the marker. Cites of this
memo's own sections, of the umbrella, of the census and of other plans are not spec cites and are not rows. Cites
that quote the code's comments (`§3.1`, `WebIDL §3.7.6`, …) are rows with the verdict on the code (§8 L-8 holds
their population). 51 rows (S 33, D 14, W 3, H 1). Mismatches in the memo's own cites: 4, each fixed in Rev 1.4
(S2, D8, D14, W1). Mismatches in the code's cites: S4, W2, W3, H1; of them `tsel`'s code PR fixes the ones in the
doc comments it rewrites (the S4 sites of §8 L-8's removed list, and W2's `selection.rs:376`), and the rest stay
with the sweep.

| # | Cite as written | Where | Lookup → result | Verdict |
|---|---|---|---|---|
| S1 | "SELECTION API §3 Selection interface", "Selection API §3" | §3 rows (Selection interface members), §0.3, §4.1 doc comments and text, §8 L-8 | `heading selection-api 3` → §3 Selection interface `#selection-interface` | ok |
| S2 | "SELECTION API §3 Selection interface" on the `selectionchange` row | §3 | `heading selection-api 6.2` → §6.2 selectionchange event `#selectionchange-event`; `dfn selection-api selectionchange` → §6.2 `#dfn-selectionchange` | fixed (M1): "SELECTION API §6.2 selectionchange event"; the other quotes of the clause (§0.3 items 5 and 9, §11 P-18) and §4.1's rule now name §6.2 |
| S3 | "SELECTION API §2 Definition", "§2's note", "§2's" | §3, §0.1 item 8, §0.5, §2 SEL × DOC | `heading selection-api 2` → §2 Definition `#definition` | ok |
| S4 | "Selection API §3.1" / "§3.2" / "§3.3" / "§3.4" (the code's) | §4.1, §8 L-8 | `heading selection-api 3.1`, `3.2`, `3.3`, `3.4` → "no headings under §3.x in Selection API" each (control: `heading selection-api 3` → §3) | the code's cites are wrong; §8 L-8 |
| S5 | `collapse()` | §0.1, §3, §4.1 | `dfn selection-api "collapse()"` → §3 `#dom-selection-collapse` | ok |
| S6 | `setPosition()` | §0.3, §5.1 | `dfn selection-api "setPosition()"` → §3 `#dom-selection-setposition` | ok |
| S7 | `extend()` | §0.1, §3, §4.1 | `dfn selection-api "extend()"` → §3 `#dom-selection-extend` | ok |
| S8 | `setBaseAndExtent()` | §0.1, §3, §4.1 | `dfn selection-api "setBaseAndExtent()"` → §3 `#dom-selection-setbaseandextent` | ok |
| S9 | `selectAllChildren()` | §0.1, §3, §4.1 | `dfn selection-api "selectAllChildren()"` → §3 `#dom-selection-selectallchildren` | ok |
| S10 | `addRange()` | §0.1, §0.5, §3, §4.1 | `dfn selection-api "addRange()"` → §3 `#dom-selection-addrange` | ok |
| S11 | `removeRange()` | §0.3, §4.1 | `dfn selection-api "removeRange()"` → §3 `#dom-selection-removerange` | ok |
| S12 | `removeAllRanges()` | §0.2, §0.3, §4.1 | `dfn selection-api "removeAllRanges()"` → §3 `#dom-selection-removeallranges` | ok |
| S13 | `empty()` | §0.2, §0.3, §4.1 | `dfn selection-api "empty()"` → §3 `#dom-selection-empty` | ok |
| S14 | `collapseToStart()` | §0.3, §7 (the note before J1) | `dfn selection-api "collapseToStart()"` → §3 `#dom-selection-collapsetostart` | ok |
| S15 | `collapseToEnd()` | §0.3, §4.1 | `dfn selection-api "collapseToEnd()"` → §3 `#dom-selection-collapsetoend` | ok |
| S16 | `deleteFromDocument()` | §0.1 item 4, §3, §4.1 | `dfn selection-api "deleteFromDocument()"` → §3 `#dom-selection-deletefromdocument` | ok |
| S17 | `containsNode()` | §0.1 item 5, §3, §4.1 | `dfn selection-api "containsNode()"` → §3 `#dom-selection-containsnode` | ok |
| S18 | `getRangeAt()` | §0.1 item 6, §3, §4.1 | `dfn selection-api "getRangeAt()"` → §3 `#dom-selection-getrangeat` | ok |
| S19 | `anchorNode` | §0.1 item 7, §3 | `dfn selection-api anchorNode` → §3 `#dom-selection-anchornode` | ok |
| S20 | `anchorOffset` | §0.1 item 7, §3 | `dfn selection-api anchorOffset` → §3 `#dom-selection-anchoroffset` | ok |
| S21 | `focusNode` | §0.1 item 7, §3 | `dfn selection-api focusNode` → §3 `#dom-selection-focusnode` | ok |
| S22 | `focusOffset` | §0.1 item 7, §3 | `dfn selection-api focusOffset` → §3 `#dom-selection-focusoffset` | ok |
| S23 | `rangeCount` | §0.1 item 7, §0.5, §3 | `dfn selection-api rangeCount` → §3 `#dom-selection-rangecount` | ok |
| S24 | `type` | §0.1 item 7, §3 | `dfn selection-api type` → §3 `#dom-selection-type` | ok |
| S25 | `isCollapsed` | §0.2, §3 | `dfn selection-api isCollapsed` → §3 `#dom-selection-iscollapsed` | ok |
| S26 | the stringifier | §0.2, §3 | `dfn selection-api stringifier` → `stringifier()`, §3 `#dom-selection-stringifier` | ok |
| S27 | *direction* / `direction` | §0.2, §3 Lookups | `dfn selection-api direction` → §2 `#dfn-direction` and §3 `#dom-selection-direction` | ok |
| S28 | *empty* | §3 §2 row, §4.1 | `dfn selection-api empty` → §2 `#dfn-empty` | ok |
| S29 | *anchor* | §0.1 item 7, §3 §2 row | `dfn selection-api anchor` → §2 `#dfn-anchor` | ok |
| S30 | *focus* | §0.1 item 7, §3 §2 row | `dfn selection-api focus` → §2 `#dfn-focus` | ok |
| S31 | *selection* ("Every document … has a unique selection") | §0.5 | `dfn selection-api selection` → §2 `#dfn-selection` | ok |
| S32 | step numbers: `collapse()` 1-6, `extend()` 1, 2, 5-7, 9, `setBaseAndExtent()` 1, 2, 5, 7, `selectAllChildren()` 1-5, 7, `addRange()` 1-3 | §0.1, §3, §4.1, §0.2 | webref 外 (raw cache, selection-api 2026-06-11): the `<li>` order under each `<dfn>` (cache lines 458-489, 530-583, 585-622, 624-656, 334-350) | ok |
| S33 | raw-cache line cites (§3 Lookups; §0.1 items 3, 8; §0.5; §0.3 item 5; §2; §4.2; §11 P-12, P-19) | as listed | webref 外 (raw cache, selection-api 2026-06-11): each `<dfn>` line of §3 Lookups, 475 / 537 / 598, 319-323 / 348 / 135-142, 99-100 / 179-181, 1043-1047, 239 … 764, 322-323, 233-299 read at the cited line | ok |
| D1 | "WHATWG DOM §5.6 Interface Range" | §3 | `heading dom 5.6` → §5.6 Interface Range `#interface-range` | ok |
| D2 | *set the start or end of a range to a boundary point*, "set the start" steps 1, 2, 4, 4.1 | §0.1 item 1, §3, §4.1, §7 E13 | `dfn dom "set the start"` → §5.6 `#concept-range-bp-set`; steps from `body dom concept-range-bp-set` | ok |
| D3 | `deleteContents()` step 1, steps 4-8 | §0.2, §3, §4.1 | `dfn dom "deleteContents()"` → §5.6 `#dom-range-deletecontents`; steps from `body dom dom-range-deletecontents` | ok |
| D4 | "WHATWG DOM §5.2 Boundary points", "DOM §5.2" | §3, §12 Q-1 | `heading dom 5.2` → §5.2 Boundary points `#boundary-points` | ok |
| D5 | *position* step 1, *before* | §2 ROOT × ORD, `tr × tsel`, §3, §8 L-6 | `dfn dom position` → §5.2 `#concept-range-bp-position`; `dfn dom before` → §5.2 `#concept-range-bp-before`; step 1 from `body dom concept-range-bp-position` | ok |
| D6 | "WHATWG DOM §4.2.1 Document tree", "DOM §4.2.1", "DOM §1.1 / §4.2.1" | §0.5, §3, §4.1 code, §11 P-19 | `heading dom 4.2.1` → §4.2.1 Document tree `#document-trees` | ok |
| D7 | *document tree* | §0.5, §3 | `dfn dom "document tree"` → §4.2.1 `#concept-document-tree` | ok |
| D8 | *in a document tree*, "the second sentence of the same prose" | §3 Lookups | `dfn dom "in a document tree"` → §4.2.1 `#in-a-document-tree`; `body dom concept-document-tree`: its fourth paragraph | fixed (M3): "the fourth paragraph" |
| D9 | "WHATWG DOM §4.8 Interface ShadowRoot", "DOM §4.8" | §3, §4.1 | `heading dom 4.8` → §4.8 Interface ShadowRoot `#interface-shadowroot` | ok |
| D10 | *shadow-including inclusive ancestor* | §0.1 item 3, §3 | `dfn dom "shadow-including inclusive ancestor"` → §4.8 `#concept-shadow-including-inclusive-ancestor` | ok |
| D11 | *shadow-including root* | §3, §4.1 | `dfn dom "shadow-including root"` → §4.8 `#concept-shadow-including-root` | ok |
| D12 | "WHATWG DOM §1.1 Trees", "DOM §1.1" | §3, §4.1 code | `heading dom 1.1` → §1.1 Trees `#trees` | ok |
| D13 | *root* (of a tree) | §3, §4.1 | `dfn dom root` → §1.1 `#concept-tree-root` (for tree) | ok |
| D14 | "DOM's removing steps … and inserting steps" | §2 LRR × ROOT | `dfn dom "removing steps"` → §4.2.3 `#concept-node-remove-ext` (the hook other specs define, not the live-range adjustment); `dfn dom "live range pre-remove steps"` → §5.6 `#live-range-pre-remove-steps`, run by *remove* step 3 (`dfn dom remove` → §4.2.3 `#concept-node-remove`; `body dom concept-node-remove`); `dfn dom insert` → §4.2.3 `#concept-node-insert` | fixed: "*live range pre-remove steps* (§5.6, run by *remove* step 3, §4.2.3) … *insert* (§4.2.3)" |
| W1 | "`[SameObject]` holds" (for `getRangeAt()`) | §4.2 | `dfn webidl SameObject` → §3.3.12 [SameObject] `#SameObject`; `body webidl SameObject`: "appears on a read only attribute" | fixed: `getRangeAt()` is an operation; the rule is the Selection API's "a reference to (not a copy of) this's range" |
| W2 | "WebIDL §3.7.6" (the code's, for argument conversion) | §8 L-8 (`selection.rs:376`, `dom_selection_mutation.rs:234`) | `heading webidl 3.7.6` → §3.7.6 Attributes `#js-attributes`; `heading webidl 3.7.7` → §3.7.7 Operations `#js-operations`, whose steps pass the arguments to the overload resolution algorithm (`body webidl js-operations` step 5; `dfn webidl "overload resolution algorithm"` → §3.6, which converts left to right) | the code's cite is wrong: `selection.rs:376` sits in `set_base_and_extent`'s rewritten doc comment and is fixed by the code PR to "WebIDL §3.7.7 Operations" (§4.1); `:234` left to the sweep |
| W3 | "WebIDL §3.10" (the code's, for ToUint32) | §8 L-8 (`dom_selection_proto.rs:412`) | `heading webidl 3.10` → §3.10 Observable array exotic objects `#js-observable-arrays` | the code's cite is wrong; left to the sweep (`tsel` does not edit that comment) |
| H1 | "HTML §8.1.7.1 ("selection task source")" (the code's) | §8 L-8 (`dom_selection_proto.rs:661`) | `heading html 8.1.7.1` → §8.1.7.1 Definitions `#definitions-3`; `dfn html "task source"` → §8.1.7.1 `#task-source`; `dfn html "selection task source"` → no dfn; the Selection API queues on the "user interaction task source" (webref 外 (raw cache, selection-api 2026-06-11); cache line 1071) | the heading is right, the named task source is not HTML's; outside the §3.x property of L-8; left to the sweep (`tsel` does not edit that comment) |

## §4. Mechanism

### §4.1 `elidex-dom-api` — `selection.rs`

Two predicates, one existing and one private (NEW), each against the existing relations of the same name
(R1 decision record K6, One issue, one way):

| Spec question | `tsel` uses | Existing relation it was checked against | Same relation? |
|---|---|---|---|
| the associated document is a shadow-including inclusive ancestor of the node (facet 3; paraphrase of the three step texts quoted in §0.1 item 3) | `crate::focus::is_in_document(dom, node, owner_document)` (`focus/sot.rs:69`), **reused** | itself; and `EcsDom::find_tree_root_composed(node) == owner_document` (`walkers.rs:432`, DOM §4.8 *shadow-including root*), which Rev 1.2 wrapped in a NEW `in_document_shadow_including` | yes: `is_in_document` climbs the stored parent to `document`, and at base a shadow root's stored parent is its host, so it is shadow-including (census §3 row R-8: the same question as *connected*, for one given document, classed shadow-including, and at base S, equal); at `ta` its step takes `shadow_including_parent` (census, "K"), so it stays the same relation through `ta` and `ts`. A second predicate for it would be a second home; Rev 1.2's NEW wrapper is dropped. Its doc comment's "light-tree ancestor chain" wording is not the relation it computes; `ta`'s census row R-8 restates it (§8 L-3), `tsel` does not edit `focus/sot.rs` |
| node's root is the document associated with this / the node is in the document tree (facets 2, 4-7; a paraphrase, not a quote: the step texts quoted in §0.1 items 2 and 4-7 state it negated ("is not the document", "not in the document tree"), all but `deleteFromDocument()`; row 1 is marked the same way) | `root_is_document(node, owner_document, dom)` (NEW, private): `dom.find_tree_root(node) == owner_document` | `element/document_base.rs:226` `in_document_light_tree(dom, entity)`: `find_tree_root(entity)` is *a* `Document` node | no: `in_document_light_tree` is the any-document reading, `root_is_document` the associated document (§0.5, rung 2); they differ exactly for a node of another document (E4, E13, J15; mutant M24). Not merged; neither is `ta`'s or `ts`'s to unify, since the difference is the reading, not the storage |

```rust
/// DOM §1.1 / §4.2.1: `node`'s root is the associated document (`node` is in its document tree;
/// the associated document, not any document: tree-model tsel plan §0.5).
fn root_is_document(node: Entity, owner_document: Entity, dom: &EcsDom) -> bool {
    dom.find_tree_root(node) == owner_document
}
```

One helper (NEW, private) carries the condition that `deleteFromDocument()`, `getRangeAt()`, `rangeCount` and
`type` each state (a paraphrase of their four clauses, not a quote: this is not empty and both anchor and focus
are in the document tree, which is the associated document's tree, §0.5), and two accessors
(NEW, public) carry the per-point clause of `anchorNode` / `anchorOffset` and `focusNode` / `focusOffset`:

```rust
/// Selection API §3 (`rangeCount`, `type`, `getRangeAt()`, `deleteFromDocument()`; paraphrased): this
/// is not empty and both anchor and focus are in the associated document's tree (rung 2, tsel plan §0.5).
fn anchor_and_focus_in_document_tree(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom, owner_document: Entity) -> bool;

/// Selection API §3 (`anchorNode`, `anchorOffset`): the anchor, or `None` if it is null or not in the document tree.
pub fn anchor_in_document_tree(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom, owner_document: Entity) -> Option<(Entity, usize)>;

/// Selection API §3 (`focusNode`, `focusOffset`): the focus, or `None` if it is null or not in the document tree.
pub fn focus_in_document_tree(&self, registry: &mut LiveRangeRegistry, dom: &EcsDom, owner_document: Entity) -> Option<(Entity, usize)>;
```

`range_count` and `selection_type` gain `registry`, `dom` and `owner_document` and read the helper first
(`range_count` today takes `&self` only). `anchor` / `focus` keep their signatures as the spec's concepts; their
doc comments drop "`None` when `rangeCount == 0`" for "`None` when this is empty" (the two now differ).

The four setters return `Result<bool, SelectionError>`: `Ok(true)` set a range, `Ok(false)` aborted (the
`add_range` convention, which returns `bool` for the same reason: the native decides on `selectionchange`).

The five methods without an abort carry their own "did something" fact too (R1 decision record K6 A1-2; the
one-site form, chosen because it is cheap: five signatures and their test call shapes). `remove_range` and
`collapse_to_start` / `collapse_to_end` return `Result<bool, SelectionError>`, `remove_all_ranges` and `empty`
return `bool`; each returns `true` on every non-error path, so behaviour is unchanged, and each doc comment says
that the value is unconditional today. The no-op event on an empty selection (`removeAllRanges()` / `empty()`:
nothing is dissociated, yet the event is scheduled) is unchanged by `tsel`; its slot placement was asked of the PM
(§11 P-20) and PM 905 folded it into facet (a) of `#11-selection-change-on-noop-range-mutation` ("any Selection
operation that is a spec no-op must not schedule selectionchange"). So the doc comments of `remove_all_ranges` and
`empty` cite that slot for the empty-selection case; those of `remove_range`, `collapse_to_start` and
`collapse_to_end` cite none (on a non-error path each dissociates or replaces the range). Then every
`mutate_selection` closure passes the `SelectionState` method's own value and no literal `Ok(true)` stays in the
host; that slot's fix is one edit inside `SelectionState`, not in the natives.

Step order, each check before any state change:

| Method | New order (spec) | Base order |
|---|---|---|
| `collapse` | DocumentType → throw; offset → throw; `is_in_document` → abort; `set_start_to_boundary` + `set_end_to_boundary` (step 6) | DocumentType; `same_document` → `WrongDocument`; offset; raw writes |
| `extend` | `is_in_document` → abort; empty → `InvalidState`; DocumentType → throw; offset → throw (DOM set-the-start steps 1-2, run before step 5's writes); if `find_tree_root(node) != find_tree_root(range start)` → both points to newFocus (step 5); else the forward / backward branch through `set_*_to_boundary` | empty; DocumentType; `same_document`; offset; raw writes |
| `set_base_and_extent` | offsets → `IndexSize` (step 1); `is_in_document` for both → abort (step 2); DocumentType for either → throw (DOM step 1, inside step 5); branch on `direction_between` through `set_*_to_boundary` (step 5) | DocumentType; `same_document`; offsets; raw writes |
| `select_all_children` | DocumentType → throw; `root_is_document` → abort; plain writes (steps 4-5 say "Set" (webref 外 (raw cache, selection-api 2026-06-11)), not "set the start") | DocumentType; `same_document`; raw writes |
| `add_range` | `root_is_document(start container of the registered range)` → abort; `range_count(registry, dom, owner_document) != 0` → abort (step 2, facet 7); set. Signature `fn add_range(&mut self, registry: &mut LiveRangeRegistry, dom: &EcsDom, owner_document: Entity, range_id: RangeId) -> bool` | `active_range_id.is_some()`; `owner_document` equality |
| `delete_from_document` | `anchor_and_focus_in_document_tree` → else nothing (it covers empty); `delete_contents` and the write-back as today. Returns `Option<Vec<MutationRecord>>`: `None` = the "do nothing" branch, `Some(records)` = ran `deleteContents()` (a collapsed range included: `Some(vec![])`) | empty → `Vec::new()`; else delete |
| `contains_node` (read) | empty → `false`; `root_is_document(node)` → else `false`; containment as today | empty; `same_document` → `false`; containment |
| `range_at` (NEW, read) | `index != 0` → `OutOfRange`; `anchor_and_focus_in_document_tree` → else `OutOfRange` (it covers empty); `Ok(active_range_id)`. Signature `fn range_at(&self, index: usize, registry: &mut LiveRangeRegistry, dom: &EcsDom, owner_document: Entity) -> Result<RangeId, SelectionError>` (`usize`: what `arg_offset_required` returns) | in the native: empty → `OutOfRange`; `index > 0` → `OutOfRange` |
| `range_count` (read) | `anchor_and_focus_in_document_tree` → 1, else 0 | `active_range_id.is_some()` |
| `selection_type` (read) | `anchor_and_focus_in_document_tree` → else `None`; collapsed → `Caret`; else `Range` | empty → `None`; collapsed → `Caret`; else `Range` |
| `anchor_in_document_tree` / `focus_in_document_tree` (NEW, read) | `self.anchor` / `self.focus`, kept only if `root_is_document` of its node | the natives read `anchor` / `focus` unfiltered |

`delete_from_document` gains the owner document as a parameter (it has none today). Why `extend` needs its own
step 5 branch: `set_*_to_boundary` collapse onto the point written last. In the forward branch that is newFocus,
which is step 5's answer; in the backward branch it is oldAnchor (probe 1d's shape, `collapse(post, 2); extend(st,
3)`, is backward because `compare_points` orders `st` before `post`). So the setters alone are not step 5; the
explicit root comparison is. `set_base_and_extent` has no such step: its cross-root branch is a spec gap (Q-1).

Why `delete_from_document` returns `Option` and not a `bool` beside the records (R1 decision record K4): the
value is the event decision, and its contract is "`Some` = this's range was given to `deleteContents()`". Slot
`#11-selection-change-on-noop-range-mutation` (Trigger: `tsel`'s landing, PM 891) stays out of `tsel` (PM 891,
decided; not reopened); if it rules that a collapsed run schedules no event, its fix is one branch inside
`delete_from_document`: test `is_collapsed` before invoking `delete_contents` and return `None` for a collapsed
range (DOM `deleteContents()` step 1 returns at once, so nothing observable is lost). The type, the native and
the native's `None` / `Some` rule stay as `tsel` leaves them. The records are not the criterion: an empty
`records` does not mean "no boundary mutated" (a non-collapsed range between two empty elements, `(a, 0)` to `(b,
0)` under one parent, has no node to remove and no CharacterData to splice, so no record, yet DOM step 8 moves both
boundary points to `(parent, index of a + 1)`; `body dom dom-range-deletecontents` steps 4-8), so the slot reads
the collapsed check, not the records.

`SelectionError::WrongDocument` (`:125`) has no producer left and is deleted (CLAUDE.md: dead code is deleted).
`same_document` (`:613`) loses its last caller when `contains_node` (`:564`) takes `root_is_document` (facet 5,
PM 891), so it is deleted too. `range_at` reuses `SelectionError::OutOfRange` (`:132`), which `map_selection_error`
already maps to "IndexSizeError". The module doc comment's "validity gates" paragraph and its single-range-disposition bullet (`selection.rs:21-23` at `3cbd5b88`: `add_range` "is a spec-correct no-op when a range is already set", false after facet 7 for a selection outside the document tree), and the doc comments of
`collapse`, `collapse_to_start`, `collapse_to_end`, `extend`, `set_base_and_extent`, `select_all_children`,
`add_range`, `remove_range`, `remove_all_ranges`, `empty`, `delete_from_document`, `contains_node`, `range_count`,
`selection_type` and `anchor` are rewritten to the new order, the new return value and the new clause; the
`getRangeAt` native's check comment (`dom_selection_proto.rs:607`) goes with its checks. Each rewritten comment
cites the flat "Selection API §3" for a Selection interface member (`webref heading selection-api 3` → §3
Selection interface; `heading selection-api 3.1` … `3.4` find no heading, §3.2 row S4), not the wrong subsection
numbers they carry today (R1 decision record K5; sweep `project_selection-api-citation-sweep`, PM 895's part "fold
into the next Selection/Range work touching these files"). The flat-§3 rule is for §3's members only: a comment
that cites the `selectionchange` clause cites "Selection API §6.2" (re-check decision record M1; §3.2 row S2).
`set_base_and_extent`'s rewritten doc comment also carries "Per WebIDL §3.7.6" (`selection.rs:376`, §3.7.6 is
Attributes); the rewrite cites "WebIDL §3.7.7 Operations" (`heading webidl 3.7.7`; its step 5 passes the arguments
to the overload resolution algorithm, which converts them left to right; §3.2 row W2). The sites this removes from
the sweep's population are listed in §8 L-8. `direction_between` and `node_fully_contained` keep their raw writes: they build temporary comparison
ranges, not Selection ranges.

### §4.2 `elidex-js` — marshalling only

- `mutate_selection` (`dom_selection_mutation.rs:55`): the closure returns `Result<bool, SelectionError>`; the
  function returns `Result<(), VmError>`; `selectionchange_pending` is set only on `Ok(true)` (`:111`). Its nine
  callers (`git grep -n "mutate_selection(ctx" crates/script/elidex-js/src/vm/host/dom_selection_mutation.rs`:
  `:196` removeRange, `:208` removeAllRanges, `:221` empty, `:237` collapse, `:249` collapseToStart, `:261`
  collapseToEnd, `:275` extend, `:293` setBaseAndExtent, `:306` selectAllChildren) each pass the
  `SelectionState` method's own value (`Ok(s.remove_all_ranges())` / `Ok(s.empty())` for the two that cannot
  throw); for the five without an abort it is `true` on every non-error path (§4.1), so their behaviour is
  unchanged. `addRange` becomes the tenth (below). The displaced-id rule (`:98-110`) is unchanged: unregister
  the old id only when `range_instances` has no wrapper for it.
- `delete_selection_contents` (`:121`): passes the document to `delete_from_document`; on `None` it neither commits
  records nor sets `selectionchange_pending` (`:146-147`); on `Some(records)` it does both, as today.
- `native_selection_add_range` (`:155`): stops reading `r.owner_document` (`:171-172`); it keeps its registration
  check first (an unregistered id is the same "TypeError"), then calls `mutate_selection` with
  `|s, reg, dom, doc| Ok(s.add_range(reg, dom, doc, range_id))`. Today it writes the slot itself and sets
  `selectionchange_pending` on `true` (`:178-186`); it never displaced a range, since step 2 aborted on any range.
  After facet (7) step 3 can replace a selection outside the document tree, so the displaced id must be freed when
  no wrapper holds it, which is `mutate_selection`'s rule (`:98-110`): one site for "a Selection write replaced its
  range" (one issue, one way), not a second copy in the native. The event is unchanged: `Ok(true)` → pending.
  PM 901 ratified this move and its rule ("an evicted `RangeId` is freed only if no wrapper holds it"), on the
  condition that a cell proves the held-wrapper half: J16 (a displaced range whose wrapper the script holds stays
  registered and usable) and J14 (one without a wrapper is freed), with mutant M23 (the `range_instances` guard
  removed) red on J16.
- `native_selection_get_range_at` (`dom_selection_proto.rs:600`): its own emptiness and index checks (`:608-624`)
  are replaced by one `read_selection` call to `range_at(index, reg, dom, doc)`; `Err` goes through
  `map_selection_error` as today, `Ok(rid)` to `wrap_range_id` as today (the same wrapper, so the method returns "a reference to (not a copy of)
  this's range" (webref 外 (raw cache, selection-api 2026-06-11); cache lines 322-323); WebIDL's `[SameObject]`,
  §3.3.12, is for read only attributes and is not the rule here, §3.2 row W1).
- `native_selection_contains_node` (`dom_selection_proto.rs:629`): unchanged (it already passes the document).
- The Range natives (`range_proto.rs`: `write_range`, `:347`; `setStart` / `setEnd` at `:541`, `:557`) are
  unchanged: DOM puts no tree check on them, and they already write through `set_*_to_boundary` (§0.1 item 8).
- The six attribute natives (`dom_selection_proto.rs:489-537`, `:551`, `:561`): `anchorNode` / `anchorOffset` call
  `anchor_in_document_tree(reg, dom, doc)`, `focusNode` / `focusOffset` `focus_in_document_tree(reg, dom, doc)`,
  `rangeCount` `range_count(reg, dom, doc)`, `type` `selection_type(reg, dom, doc)`; `read_selection` already hands
  them `doc` (today bound as `_doc`). Their `None` → `null` / `0` mapping is unchanged. `isCollapsed`, `direction`
  and the stringifier are unchanged (§0.2).
- `map_selection_error` (`dom_selection_proto.rs:304`): the `WrongDocument` arm (`:314`) is deleted with the
  variant. No other arm changes.

### §4.3 ECS-native / layering check

The predicates and step orders live in `elidex-dom-api` (Layering mandate); the VM changes are the "did something"
flags, parameter lists (the six attribute natives pass `doc`), and `addRange`'s native moving onto
`mutate_selection`. `SelectionState` stays a `RangeId` plus a direction (no new component, no
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
for m in range_count anchor focus selection_type; do git grep -n -P "\.$m\(" -- crates; done   # Rev 1.2, facet 7
git grep -n -w selection_state -- crates
git grep -l -P '\bgetSelection\b' -- crates ':!crates/script/elidex-js' ':!crates/dom/elidex-dom-api'   # 0; control: without the excludes, elidex-js files hit
```

| Method | Production caller (one each) | Test callers |
|---|---|---|
| `collapse` | `native_selection_collapse` (`dom_selection_mutation.rs:238`; `setPosition` is the same native) | `selection_tests.rs` (11 calls) |
| `extend` | `native_selection_extend` (`:276`) | `selection_tests.rs` (3) |
| `set_base_and_extent` | `native_selection_set_base_and_extent` (`:294`) | `selection_tests.rs` (2) |
| `select_all_children` | `native_selection_select_all_children` (`:307`) | `selection_tests.rs` (2) |
| `add_range` | `native_selection_add_range` (`:182`) | `selection_tests.rs` (3) |
| `delete_from_document` | `delete_selection_contents` (`:140`), from `native_selection_delete_from_document` (`:312`) | none in `selection_tests.rs`; VM: `tests_mutation_observer/range_ops.rs` (2) |
| `contains_node` | `dom_selection_proto.rs:638` | `selection_tests.rs` (4) |
| `range_at` (NEW; today the native's own checks) | `native_selection_get_range_at` (`dom_selection_proto.rs:600`) | VM: `tests_selection.rs` (`getRangeAt` in 7 tests, `git grep -n getRangeAt -- crates`; six are §7.2's fixture tests, `get_range_at_out_of_range_throws` runs on an empty selection) |
| `collapse_to_start`, `collapse_to_end` (return `Result<bool, _>`, Rev 1.3) | `dom_selection_mutation.rs:250`, `:262` | — |
| `remove_range` (`Result<bool, _>`), `remove_all_ranges`, `empty` (`bool`; Rev 1.3) | `dom_selection_mutation.rs:197`, `:209`, `:222`; `empty` calls `remove_all_ranges` (`selection.rs:496`) | `selection_tests.rs:279` (`.unwrap()`), `:287` (compares an `Err`), `:299`, `:310` (value unused): the call shapes compile unchanged (no `#[must_use]` on a `bool` return); measured with `for m in remove_range remove_all_ranges empty collapse_to_start collapse_to_end; do git grep -n -P "\.$m\(" 3cbd5b88 -- crates; done` (12 hits at `3cbd5b88`: the callers listed here, `selection.rs:493` (a doc line), `:496` and one VM test script string, `tests_selection.rs:518`) |
| `range_count` (signature gains `registry`, `dom`, `owner_document`) | `native_selection_get_range_count` (`dom_selection_proto.rs:557`); `add_range` step 2 (NEW internal caller) | `selection_tests.rs:34`, `:57`, `:120` |
| `selection_type` (same) | `native_selection_get_type` (`dom_selection_proto.rs:567`) | `selection_tests.rs:44`, `:58`, `:121` |
| `anchor` (concept, unchanged) | `anchorNode` / `anchorOffset` natives (`dom_selection_proto.rs:495`, `:508`) → move to `anchor_in_document_tree`; `extend`'s oldAnchor (`selection.rs:352`) stays | `selection_tests.rs:46`, `:63`, `:127`, `:147`, `:167`, `:215` (unchanged) |
| `focus` (concept, unchanged) | `focusNode` / `focusOffset` natives (`dom_selection_proto.rs:520`, `:533`) → move to `focus_in_document_tree` | `selection_tests.rs:47`, `:64`, `:128`, `:148`, `:168`, `:216` (unchanged) |

**Non-script callers: none.** The only `SelectionState` users outside `selection.rs` are the VM's Selection natives,
its HostData slot (`host_data/mod.rs:425`, `bind.rs` borrow splits, `lifecycle.rs:62` reset) and the GC root walk
(`object_roots.rs:156`, `current_range_id` only). The fourth command finds only `lib.rs:120` (the re-export) and
`elidex-form-core`'s own `SelectionDirection` (the input / textarea selection, a different type). No editing, shell
or a11y code reads or writes the document Selection, so nothing depends on today's lax behaviour. Re-run for facet (7) (Rev 1.2, at `3cbd5b88`): the `.range_count(` / `.anchor(` /
`.focus(` / `.selection_type(` hits are the rows above and nothing else (the `.focus(` command's other hits are
`HTMLElement.focus()` / `window.focus()` prose and tests, a different method); `selection_state` (the HostData
field, `pub(crate)`) is read only in `dom_selection_*`, `bind.rs`, `object_roots.rs:154` (`current_range_id`),
`lifecycle.rs:62` (reset) and `host_data/mod.rs`; no crate outside `elidex-js` / `elidex-dom-api` names
`getSelection` (0 hits; the control without the excludes lists `elidex-js` files). No non-script reader depends on
the attributes' values today.

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
| N2 | `selectAllChildren(L); containsNode(st)` | `true` | `false` (facet 5; base row of E9, J11) |
| N4 | `selectAllChildren(L); direction` | `"directionless"` | `"forward"` (§0.2) |
| N5 | `setBaseAndExtent(st,0, st,3); getRangeAt(0)` | returns | "IndexSizeError" (facet 6; base row of E10, J12) |
| A1 | `setBaseAndExtent(st,0, st,3); anchorNode` | `st` | `null` (facet 7; base row of E11, J13). Same run: `isCollapsed` `false`, `toString()` `"SHA"` on base and in the spec (no clause, §0.2) |
| A2 | `collapse(st,1); anchorOffset` | `1` | `0` (with `setBaseAndExtent(st,0, st,3)` base already reads `0`, so this row uses the caret) |
| A3 | `setBaseAndExtent(st,0, st,3); focusNode` | `st` | `null` |
| A4 | `setBaseAndExtent(st,0, st,3); focusOffset` | `3` | `0` |
| A5 | `setBaseAndExtent(st,0, st,3); rangeCount` | `1` | `0` (with facet 6 alone, `getRangeAt(0)` would throw while this reads 1: §2 read × read) |
| A6 | `setBaseAndExtent(st,0, st,3); type`; `collapse(st,1); type` | `"Range"`; `"Caret"` | `"None"`; `"None"` |
| A7 | `setBaseAndExtent(pre,0, pre,2)` (control): the six attributes | `pre`, 0, `pre`, 2, 1, `"Range"` | the same |
| A8 | `setBaseAndExtent(st,0, st,3); addRange(r)`, `r` = (`pre`,0)-(`pre`,2) | aborts: `getRangeAt(0) !== r`, the selection stays (`st`,0)-(`st`,3) | step 2 reads `rangeCount` 0 → step 3 sets `r`: `getRangeAt(0) === r` (§0.1 item 7, P-15) |

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

Rows A1-A8 are round 3 (Rev 1.2, 2026-11-21, same base and command, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, one
cargo job; the temporary file removed and `mod.rs` restored afterwards, `git status --short` empty). The probe
sources are kept outside the repository for the record (VM round 3:
`/private/tmp/claude-501/-Users-kazuaki-orca-workspaces-elidex-L3/dd0852cc-51ac-4464-8e63-719a384a6780/scratchpad/tests_tsel_probe_round3.rs`; VM round 2:
`/private/tmp/claude-501/-Users-kazuaki-orca-workspaces-elidex-L3/dd0852cc-51ac-4464-8e63-719a384a6780/scratchpad/tests_tsel_probe_round2.rs`;
dom-api: `.../scratchpad/tsel_probe_domapi.diff`); the round-1 cases (1a-4a, N1-N5) are the rows above, on the same
fixture.

Rev 1.3 (R1 decision record K1) ran no probe. The third write source's cells are **predicted from code**, per
cell: E13 (a), (b), (c) and J15 (a), (b), (c), from `native_range_set_start` (`range_proto.rs:541`), which
validates only DocumentType and offset (`validate_boundary_node_and_offset`, `:488`) and writes through
`set_start_to_boundary` under `write_range` (`:347`, `registry.with_range_mut`), so a cross-root destination
collapses the range onto it; the selection's reads then follow facets (4)-(7) as specified (their base values are
irrelevant: the cells assert the spec's values). J16, from `mutate_selection`'s displaced-id rule
(`dom_selection_mutation.rs:98-110`, `range_instances.contains_key`) and `getRangeAt`'s wrapper cache: `r0` is in
`range_instances`, so the displaced id is kept. On base J15 / J16 cannot be shaped (base `addRange` aborts on any
range, and the attributes carry no clause), so a base probe would show only today's values.

Rev 1.4 (re-check decision record; no cargo run, budget) adds two predicted-from-code items. J15 (d):
`r.setStart(st, 0)` is cross-root, so DOM set-the-start step 4.1 collapses the range onto (`st`, 0)
(`native_range_set_start`, `range_proto.rs:541`, through `set_start_to_boundary` at `:552` under `write_range`,
`:347`); `r.setEnd(st, 3)` is then same-root and not before the start, so only the end moves
(`native_range_set_end`, `:557`, `set_end_to_boundary` at `:567`), leaving (`st`, 0)-(`st`, 3) not collapsed;
`deleteFromDocument()` reaches `SelectionState::delete_from_document` (`selection.rs:513`), whose new
`anchor_and_focus_in_document_tree` check fails (`st`'s root is `sr`), so it returns `None` and
`delete_selection_contents` neither commits records nor sets the pending flag (today unconditional at
`dom_selection_mutation.rs:146-147`); the Range natives set no pending flag (§11 P-18: the 3 writes of
`selectionchange_pending` are all in `dom_selection_mutation.rs`). M25: the trace is under §7's mutant table.

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
- E9 `contains_node_root_not_document_is_false` (NEW; base row N2): `select_all_children(host's parent in doc)` →
  `contains_node(shadow Text, false)` and `(…, true)` → `false`; `contains_node(detached element, true)` → `false`;
  control: `contains_node(the host, false)` → `true`.
- E10 `range_at_outside_document_tree_is_index_size` (NEW; base row N5): `set_base_and_extent(shadow Text, 0,
  shadow Text, 3)` → `range_at(0)` is `Err(OutOfRange)`; control: a selection over a light Text in `doc` →
  `range_at(0)` is `Ok(current_range_id)`, `range_at(1)` is `Err(OutOfRange)`; empty → `Err(OutOfRange)`.
- E11 `attributes_outside_document_tree` (NEW; base rows A1-A6): `set_base_and_extent(shadow Text, 0, shadow Text,
  3)` → `anchor_in_document_tree` and `focus_in_document_tree` are `None`, `range_count` is 0, `selection_type` is
  `None`, while the concepts `anchor` / `focus` are `Some((shadow Text, 0))` / `Some((shadow Text, 3))` (the spec's
  note, §3 row §2) and `is_collapsed` is `false`; `collapse(shadow Text, 1)` → `range_count` 0, `selection_type`
  `None`; control: `set_base_and_extent(light Text in doc, 0, …, 2)` → `Some((Text, 0))`, `Some((Text, 2))`, 1,
  `Range`; empty → `None`, `None`, 0, `None`.
- E12 `add_range_over_selection_outside_document_tree_sets` (NEW; base row A8): a selection (shadow Text, 0)-(shadow
  Text, 3), then `add_range` of a registered Range in `doc` → `true`, `current_range_id` is that Range's id;
  control: a selection over a light Text in `doc`, then `add_range` of another Range in `doc` → `false`,
  `current_range_id` unchanged.
- E13 `selection_range_moved_out_by_range_write` (NEW, Rev 1.3; §0.1 item 8, R1 decision record K1): a selection
  over a light Text in `doc`; then the selection's own registered Range is written as a Range native writes it,
  `registry.with_range_mut(current_range_id, dom, |r, dom| r.set_start_to_boundary(x, 0, dom))`, for `x` = (a)
  the shadow Text, (b) a detached element, (c) an element of the second document, each on a fresh fixture → the
  range is collapsed at (`x`, 0) (DOM set-the-start step 4.1), `current_range_id` unchanged (the same range stays
  associated), `range_count` 0, `selection_type` `None`, `anchor_in_document_tree` / `focus_in_document_tree`
  `None`, `range_at(0)` `Err(OutOfRange)`, `delete_from_document` `None`; then `add_range` of a registered Range
  in `doc` → `true` (step 2 reads 0). Expectations from the associated-document reading (§0.5, rung 2) for (c).

**VM end-to-end, `crates/script/elidex-js/src/vm/tests/tests_selection_preconditions.rs` (NEW)** (§6's tree):

After facet (7) a selection outside the document tree reads `rangeCount` 0 like an empty one, so a VM cell that
must tell "aborted" from "set outside the document tree" reads through what has no clause: `isCollapsed`,
`toString()`, and `collapseToStart()`, which throws "InvalidStateError" exactly when this is empty ("The method must
throw InvalidStateError exception if the this is empty."; webref 外 (raw cache, selection-api 2026-06-11)). The
cells call it last, since on a non-empty selection it replaces the range (§11 P-16).

- J1 `cross_root_set_base_and_extent_is_collapsed_and_empty` (NEW): 1a and 1b → `isCollapsed`, `toString()` `""`.
  Green on `tsel`'s base and after `tq` (the value `tq`'s cell expects). Where the range collapses is Q-1, so the
  point is not asserted; what the spec fixes for either point is asserted: `(rangeCount === 0) === (anchorNode ===
  null)`, `(anchorNode === null) === (focusNode === null)` (one caret), `type === (rangeCount === 0 ? "None" :
  "Caret")`, and `getRangeAt(0)` throws "IndexSizeError" iff `rangeCount` is 0 (facets 6 and 7 read one condition).
- J2 `cross_root_extend_collapses_at_focus` (NEW): 1c and 1d → `isCollapsed`, `toString()` `""`; the caret is at
  (`st`, 3) (step 5), inside the shadow tree, so `anchorNode` and `focusNode` are `null`, `anchorOffset` and
  `focusOffset` are `0`, `rangeCount` is `0`, `type` is `"None"` (facet 7) and `getRangeAt(0)` throws
  "IndexSizeError" (facet 6). A caret left at (`pre`, 1) or (`post`, 2) (M2's shape) is in the document tree and
  reads `pre` / `post`, 1 and `"Caret"`. The point itself is also E2's.
- J3 `cross_root_delete_from_document_removes_nothing` (NEW): 1e and 1f → `pre.data === "pre"`, `st.data ===
  "SHADOW"`, `se.isConnected`.
- J4 `select_all_children_outside_document_tree_is_noop` (NEW): 2a, 2b, 2c, each on an empty selection → no throw,
  `toString()` `""` (a set selection would read `"SHADOW"` / `"detached"`), `collapseToStart()` throws
  "InvalidStateError".
- J5 `add_range_outside_document_tree_is_noop` (NEW): 2e, 2f on an empty selection → no throw, `isCollapsed`
  `true`, `toString()` `""` (a set one reads `"SH"` / `"de"`), `collapseToStart()` throws "InvalidStateError".
- J6 `setters_outside_document_abort_without_throw` (NEW): 3a, 3c, 3d and 3h's four abort cases on an empty
  selection → no throw, `toString()` `""`, `collapseToStart()` throws "InvalidStateError"; 3e (caret at (`pre`,
  1) first) → no throw, `anchorNode === pre`, `anchorOffset` 1, `focusNode === pre`, `type` `"Caret"`.
- J7 `aborted_setter_schedules_no_selectionchange` (NEW): one aborted call per method (3a, 3c, 3e, 2a, 2e) → the
  event count stays 0 across the drain; control: `collapse(pre, 1)` → 1.
- J8 `setter_step_order_exceptions` (NEW): `collapse(ft, 99)` → "IndexSizeError" (base: "WrongDocumentError");
  3f, 3g → "IndexSizeError"; `extend(pre, 1)` on empty → "InvalidStateError".
- J9 `shadow_tree_selection_is_accepted` (NEW): 3b → one `selectionchange` across the drain (an abort would give
  0), and the caret is there but outside the document tree: `anchorNode` `null`, `anchorOffset` `0`, `rangeCount`
  `0`, `type` `"None"` (facet 7), `collapseToStart()` does not throw (not empty); `setBaseAndExtent(st,0, st,3)` →
  `isCollapsed` `false`, `toString()` `"SHA"`, `anchorNode` and `focusNode` `null`, `focusOffset` `0`,
  `rangeCount` `0`, `type` `"None"`.
- J10 `delete_from_document_inside_shadow_tree_does_nothing` (NEW): 4a → `st.data === "SHADOW"`, the selection
  still in place (`isCollapsed` `false`, `toString()` `"SHA"`) and still outside the document tree (`rangeCount`
  `0`, `type` `"None"`, `anchorNode` `null`), no `MutationRecord` for a `MutationObserver` on
  `sr` with `{characterData: true, subtree: true}`, and no `selectionchange` from the `deleteFromDocument()` call
  (counted across a drain that follows the setter's own event).
- J11 `contains_node_outside_document_tree_is_false` (NEW; base row N2): `selectAllChildren(L)` →
  `containsNode(st)` and `containsNode(st, true)` → `false`; `containsNode(dt, true)` → `false`; control:
  `containsNode(pre)` → `true`.
- J12 `get_range_at_outside_document_tree_throws` (NEW; base row N5): `setBaseAndExtent(st,0, st,3);
  getRangeAt(0)` → "IndexSizeError"; control: `setBaseAndExtent(pre,0, pre,2)` → `getRangeAt(0) ===
  getRangeAt(0)` and `getRangeAt(1)` throws "IndexSizeError".
- J13 `attributes_outside_document_tree_are_null_zero_none` (NEW; base rows A1-A7): `setBaseAndExtent(st,0, st,3)`
  → `anchorNode` `null`, `anchorOffset` `0`, `focusNode` `null`, `focusOffset` `0`, `rangeCount` `0`, `type`
  `"None"`, and (no clause) `isCollapsed` `false`, `toString()` `"SHA"`; `collapse(st,1)` → `anchorOffset` `0`,
  `focusOffset` `0`, `type` `"None"`; control (A7): `setBaseAndExtent(pre,0, pre,2)` → `pre`, `0`, `pre`, `2`, `1`,
  `"Range"`.
- J14 `add_range_replaces_selection_outside_document_tree` (NEW; base row A8): `setBaseAndExtent(st,0, st,3)`; `r`
  = (`pre`,0)-(`pre`,2); after a drain, `addRange(r)` → `getRangeAt(0) === r`, `rangeCount` `1`, `anchorNode ===
  pre`, one `selectionchange` across the next drain; the live-range registry's `len()` drops by one across the
  `addRange` call (the displaced selection range has no wrapper, since `getRangeAt(0)` threw; read as
  `vm.inner.host_data.as_ref().unwrap().live_range_registry.len()`, the field `pub(crate)` at `host_data/mod.rs:367`,
  as `tests_abort.rs:555` reads `listener_store`); control: with a selection over `post` first, `addRange(r)` →
  `getRangeAt(0) !== r`, no event, `len()` unchanged.
- J15 `range_method_moves_selection_out_of_document_tree` (NEW, Rev 1.3; §0.1 item 8, K1): for each destination,
  on a fresh tree: `setBaseAndExtent(pre,0, pre,2); r = getRangeAt(0)`, then (a) `r.setStart(st, 0)`, (b)
  `r.setStart(dt, 0)`, (c) `r.setStart(ft, 0)` (`ft` the `DOMParser` document's Text) → `r.collapsed` `true`,
  `r.startContainer` the destination (the script's handle still writes the selection's range), and the selection
  reads `anchorNode` `null`, `anchorOffset` `0`, `focusNode` `null`, `focusOffset` `0`, `rangeCount` `0`, `type`
  `"None"`, `getRangeAt(0)` throws "IndexSizeError", `isCollapsed` `true`; `deleteFromDocument()` then leaves
  `st.data === "SHADOW"` / `dt.data === "detached"` / `ft.data === "foreign"`; for (c) also `containsNode(ft)` →
  `false` (its root is not the associated document). (d) (Rev 1.4, re-check decision record M6) a destination
  built non-collapsed, so that `deleteContents()` would remove text if it ran: `r.setStart(st, 0); r.setEnd(st,
  3)` (same root, so the range is (`st`, 0)-(`st`, 3)) → `r.collapsed` `false`, `isCollapsed` `false`,
  `toString()` `"SHA"`, `rangeCount` `0`, `type` `"None"`; then, after a drain, the event count is read, and
  `deleteFromDocument()` → `st.data === "SHADOW"` (unchanged) and no `selectionchange` across the next drain (the
  "do nothing" branch). (a)-(c) leave a collapsed range, on which `deleteContents()` removes nothing whether or not
  it runs, so only (d) tells a skipped call from a run one on this path. No `selectionchange` count is asserted for
  the `setStart` / `setEnd` calls (slot `#11-selection-change-on-noop-range-mutation` facet (b), PM 903; §11
  P-18). Expectations: the associated-document reading (§0.5) for (c); predicted from code, not probed (§6; (d)'s
  trace is there).
- J16 `add_range_displacing_held_wrapper_keeps_it_live` (NEW, Rev 1.3; K1 (d), the PM 901 condition): for each of
  J15's three destinations, on a fresh tree: `setBaseAndExtent(pre,0, pre,2); r0 = getRangeAt(0);
  r0.setStart(<destination>, 0)`; `r` = (`post`,0)-(`post`,2) from `document.createRange()`; read `len()` as in
  J14; after a drain, `addRange(r)` → `getRangeAt(0) === r`, `rangeCount` `1`, one `selectionchange` across the
  next drain, `len()` **unchanged** across the `addRange` call (the displaced range's wrapper `r0` is held, so
  `mutate_selection` keeps it registered), and `r0` still works: `r0.startContainer` is the destination and
  `r0.collapsed` is `true` (no "InvalidStateError" from a dropped registry entry). With J14 (no wrapper → `len()`
  drops by one) it fixes both halves of the displaced-id rule. Predicted from code, not probed (§6).

**Mutants** (each run once against the final head, ≤2 concurrent cargo jobs; each must turn at least the listed
cells red):

| # | Mutant | Red |
|---|---|---|
| M1 | `extend`: raw `set_start` / `set_end` back, step 5 branch removed | E2, J2 (`isCollapsed`) |
| M2 | `extend`: step 5 branch removed, `set_*_to_boundary` kept | E2 (backward), J2 (1d, `getRangeAt`) |
| M3 | `set_base_and_extent`: raw writes back | E1, J1, J3 |
| M4 | the shadow-including check (`is_in_document`) → node-document equality (base `same_document`) | E5 (detached), J6 (3a, 3c, 3e) |
| M5 | the shadow-including check (`is_in_document`) → `find_tree_root` equality | E6, J9 |
| M6 | `root_is_document` → `find_tree_root_composed` equality | E3 (shadow), E4 (shadow), E8, E9 (shadow), E10, E11, E13 (a), J4 (2a, 2b), J5 (2e), J10, J11 (`st`), J12, J13, J15 (a) |
| M7 | the abort returns `Err(InvalidState)` instead of `Ok(false)` | E5, J6 |
| M8 | `mutate_selection` sets `selectionchange_pending` on `Ok(false)` | J7 |
| M9 | `collapse`: the document check before the offset check | E7, J8 (`collapse(ft, 99)`) |
| M10 | `extend`: the empty check before the document check | E7 (D5), J6 (3d) |
| M11 | `set_base_and_extent`: DocumentType check before the offset check | E7 (D3) |
| M12 | `delete_from_document`: the document-tree check removed | E8, J10, J15 (d) (`st.data` "DOW") |
| M13 | `delete_selection_contents` sets `selectionchange_pending` on `None` | J10 (event), J15 (d) (event) |
| M14 | `contains_node`: `root_is_document` → node-document equality (base `same_document`) | E9, J11 |
| M15 | `range_at`: the document-tree check removed | E10, J2, J12 |
| M16 | `anchor_in_document_tree` returns `anchor` unfiltered | E11, J2, J9, J13 (`anchorNode`, `anchorOffset`) |
| M17 | `focus_in_document_tree` returns `focus` unfiltered | E11, J2, J9, J13 (`focusNode`, `focusOffset`) |
| M18 | `range_count`: back to `active_range_id.is_some()` | E11, J2, J9, J10, J13 (and J1's relation where the kept `direction_between` collapses 1a inside the shadow tree, Q-1; not claimed) |
| M19 | `selection_type`: the document-tree check removed | E11, J2, J9, J10, J13 |
| M20 | `add_range` step 2 on `is_empty` (base) instead of `range_count` | E12, J14 |
| M21 | `native_selection_add_range` writes the slot itself again (no `mutate_selection`, so no displaced-id unregister) | J14 (`len()`) |
| M22 | the shared helper `anchor_and_focus_in_document_tree` → `!is_empty` | E8, E10, E11, E13, J2, J10, J12, J13, J14, J15 |
| M23 | `mutate_selection` unregisters the displaced id unconditionally (the `range_instances.contains_key` guard removed; on the `addRange` path, which now goes through it) | J16 (`len()` drops, `r0.startContainer` throws); also, predicted, the existing `previous_range_survives_collapse_when_wrapper_held` on the `collapse` path |
| M24 | `root_is_document` → "the root is a `Document`" (the any-document reading, `in_document_light_tree`'s relation) | E4 (second document), E13 (c), J15 (c) (`rangeCount` 1) |
| M25 (Rev 1.4, re-check decision record M5) | confined to the `addRange` path: `native_selection_add_range`, after `mutate_selection` returns, unregisters the displaced id itself whenever it differs from the new one, with no `range_instances` guard (`mutate_selection`'s shared guard is kept, unlike M23) | J16 only (`len()` drops, `r0.startContainer` throws) |

Why M25 is red on J16 only (predicted from code, not run; re-check decision record M5): J14's displaced range
has no wrapper, so `mutate_selection` already unregistered it (`dom_selection_mutation.rs:103-110`, the guard
`range_instances.contains_key` at `:106`), and the native's second `unregister` is a `HashMap::remove` of an absent
key, which returns `None` and changes nothing (`range/live.rs:408-413`): `len()` drops by one either way. J16's
displaced range has its wrapper `r0`, so the shared guard keeps it and M25's unguarded `unregister` removes it:
`len()` drops (J16 expects it unchanged), and `r0.startContainer` goes through `read_range` (`range_proto.rs:319`),
which returns the detached-range "InvalidStateError" for an unregistered id (`:333`, `:339`; `detached_range_error`,
`:303`). No E cell runs the native. The existing `addRange` VM cells (`add_range_sets_when_empty`: no previous id;
`add_range_noop_when_already_set`: no change of id) take no unregister. M23 stays as it is: it removes the shared
guard, so it is red on J16 and, predicted, on the `collapse` path too.

`add_range` with its two aborts swapped is an equivalent mutant (both abort, nothing observable differs); no cell
is claimed for it. A mutant that checks only the anchor (not the focus) in `delete_from_document` or in `range_at`
is equivalent under SEL × ROOT (both points share one root after `tsel`); no cell is claimed for it. For the same
reason the helper checking one point only, and `anchor_in_document_tree` testing the focus's node (or the reverse),
are equivalent; no cell is claimed.

### §7.1 1000-line re-count at `3cbd5b88`

Re-counted for Rev 1.2 (`wc -l` at `3cbd5b88`, unchanged since Rev 1): `selection_tests.rs` 367 (E1-E10 +≈350,
E11-E12 +≈90, Rev 1.3's E13 +≈50, the §7.2 call-shape edits ±0 → ≈860); `tests_selection.rs` 787: the VM cells
(J1-J12 ≈390, J13-J14 ≈110, Rev 1.3's J15-J16 ≈110, Rev 1.4's J15 (d) ≈15) would take it past 1000, so they go in the new file
`tests_selection_preconditions.rs` (≈625; CLAUDE.md
*1000-line debt*); `tests_selection.rs` gets only the fixture edit (+1 line). `selection.rs` 677 (+≈70, the
attribute helpers +≈45, `same_document` −≈10 → ≈780); `dom_selection_mutation.rs` 320 (`addRange` onto
`mutate_selection`, ≈−5); `dom_selection_proto.rs` 748 (six closures, ±0); `vm/tests/mod.rs` 215. No file crosses
1000; no prerequisite split.

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
  `Directionless`, which is the widened slot's (§0.2, N4) and is left as it is, as is
  `set_base_and_extent_collapsed_is_directionless`. The `range_count()` / `selection_type()` calls
  (`:34`, `:44`, `:57-58`, `:120-121`) gain `registry`, `dom` and `doc`; every one reads an empty selection or one
  built under `doc` (appended to it), so its value is the spec's unchanged (`new_state_is_empty_directionless`
  builds no DOM today and takes `fresh()` for the new arguments; 0 holds). The `anchor` / `focus` asserts are the
  concepts and are unchanged. `contains_node_full_contain_basic` builds
  under `doc` (`true` holds); `contains_node_cross_document_false` passes a node of a second document, whose root is
  that document, so `false` holds under `root_is_document`.
- **The attributes in the VM.** Every `tests_selection.rs` read of `anchorNode` / `anchorOffset` / `focusNode` /
  `focusOffset` / `rangeCount` / `type` runs on `TREE_SETUP` (attached: in the document tree, values unchanged) or
  on an empty selection (`null` / `0` / `"None"` before and after). `range_ops.rs`'s two Selection cells build under
  `body`. The direction tests (`extend_collapsed_back_to_anchor_is_directionless`,
  `set_base_and_extent_collapsed_is_directionless`) are the widened slot's and are left as they are.
- **`getRangeAt` / `containsNode` in the VM.** Every `getRangeAt` / `containsNode` test in `tests_selection.rs`
  runs on `TREE_SETUP` (now attached: every point is in the document tree, so `getRangeAt` returns and
  `containsNode` keeps its value) or on an empty selection (`get_range_at_out_of_range_throws`: "IndexSizeError"
  both before and after).
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
  shadow-including inclusive ancestor (the existing `focus::is_in_document`), in the spec's step order;
  `SelectionState::delete_from_document` does nothing unless anchor and focus are in the document tree (PM 890);
  `SelectionState::contains_node` answers `false` unless the node's root is the document, and
  `SelectionState::range_at` (NEW) carries `getRangeAt()`'s three conditions (PM 891);
  `SelectionState::range_count` / `selection_type` and `anchor_in_document_tree` / `focus_in_document_tree` (NEW)
  carry the attributes' document-tree clause, and `add_range` step 2 reads `range_count` (PM 897);
  the five methods without an abort return their own "did something" value; `SelectionError::WrongDocument` and
  `same_document` deleted. In `crates/script/elidex-js/src/vm/host/`: `mutate_selection` and
  `delete_selection_contents` (`dom_selection_mutation.rs`) schedule `selectionchange` only when the operation did
  something, each closure passing the `SelectionState` method's own value; `native_selection_add_range` goes
  through `mutate_selection` (PM 901), so a displaced range is freed only when no wrapper holds it;
  `native_selection_get_range_at` (`dom_selection_proto.rs`) calls `range_at` in place of its own checks; the six
  attribute natives (`anchorNode`, `anchorOffset`, `focusNode`, `focusOffset`, `rangeCount`, `type`) call
  `anchor_in_document_tree` / `focus_in_document_tree` / `range_count` / `selection_type` with the document;
  `map_selection_error` loses its `WrongDocument` arm." The row's "no VM change" is corrected to the marshalling edits (§11 P-2), and its
  "scope widened by PM 885" gains "and PM 890 (`deleteFromDocument`), PM 891 (`addRange` step 1,
  `containsNode`, `getRangeAt`) and PM 897 (the attributes' document-tree clause)".
- **L-2** the umbrella erratum (§0.4 (a)-(e), including the `tq` row's Deps) is in the plan PR: nothing to apply
  at landing.
- **L-3** to `ta`'s plan (hand-off, not a ledger edit): census C-6 row `find_tree_root_composed` "2 callers
  (verified 2026-11-18)": unchanged (Rev 1.3 reuses `is_in_document`, so `tsel` adds no caller); row
  `find_tree_root` "24 … lines" → 25 (`root_is_document`, plus `extend`'s step 5 if written as a direct call);
  census R-8 `is_in_document` (`focus/sot.rs:69`) gains three callers in `selection.rs` (`collapse`, `extend`,
  `set_base_and_extent`) with its relation unchanged, and its doc comment ("its light-tree ancestor chain reaches
  `document`") does not say the shadow-including relation it computes, which `ta`'s step change
  (`shadow_including_parent`) is the place to restate. `ta` re-measures at its base.
- **L-4** withdrawn (Rev 1.1): PM 891 brought `containsNode` into the slice; its change, cells and mutant are
  §0.1 item 5, §4.1, E9 / J11 and M14.
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
  plan PR touches neither memo, so their wording is left for the next PR that does. Accepted as is (R1 decision
  record K6, A3 MIN-2): the closed row's pointer keeps every cite valid, and PM 890 required no memo edit. The
  close text names the PR: "fixed by `tsel` (PR #<n>, `<sha>`, <date>)".
- **L-6** to `tr`'s plan (hand-off, not a ledger edit; R1 decision record K3; §2 `tr × tsel`): `tsel` keeps the
  cross-root order queries `direction_between` → `compare_points` in `set_base_and_extent` (step 5) and in
  `extend`'s direction step (step 9) (Q-1), and edits `contains_node`, which the umbrella maps to `tr`; after
  `tsel`, `contains_node`'s boundary comparison is also cross-root for a selection whose range a Range method
  moved out of the document (§0.1 item 8). If `tr` turns DOM *position* step 1 ("Assert: nodeA and nodeB have
  the same root.") into code, those three call sites reach it from production, so `tr`'s plan takes them (its
  cells assert relations, not `tsel`'s collapse point). Next to L-3.
- **L-7** the umbrella landing list (`elidex-pm-trial/handoff/pending-at-landing/tree-model-umbrella.md`, the
  application of umbrella §7.4 "At `ta`" item (5)): the new slice list "Slices tp → tq → ta → …" gains `tsel`
  ("tp → tsel → tq → ta → …", umbrella §5.1 as amended by §0.4 (d)) when the PM applies it; the umbrella text is
  not edited by this plan PR (§0.4).
- **L-8** the citation sweep (`project_selection-api-citation-sweep`, PM 895's part; R1 decision record K5;
  population re-measured in Rev 1.4 by the property, re-check decision record M4). The property is "a §3.x cite"
  (a wrong section in every case found), not a list of known labels. Command at `3cbd5b88`:
  `git grep -n -P '§\s*3\.[0-9]' 3cbd5b88 -- crates/dom/elidex-dom-api/src/selection.rs
  crates/dom/elidex-dom-api/src/selection_tests.rs crates/script/elidex-js/src/vm/host/dom_selection_mutation.rs
  crates/script/elidex-js/src/vm/host/dom_selection_proto.rs` → **37 hits** (`selection.rs` 29,
  `dom_selection_mutation.rs` 1, `dom_selection_proto.rs` 7, `selection_tests.rs` 0). Control: the hit
  `selection.rs:218` ("Per Selection API §3.1:", `anchor`'s doc comment) is in the output, and `git show
  3cbd5b88:crates/dom/elidex-dom-api/src/selection.rs | sed -n 218p` shows that line. (Rev 1.3's `'§3\.[123]\b'`
  gave 33: it missed `§3.4`, `§3.7.6` and `§3.10`.) Classified with webref (§3.2 rows S4, W2, W3):
  - **Selection API, wrong section (34)**: the Selection API has no §3.1-§3.4 (`heading selection-api 3.1` …
    `3.4`: no heading each; control `heading selection-api 3` → §3 Selection interface). `selection.rs:25`, `:52`,
    `:55`, `:84`, `:87`, `:120`, `:124`, `:128`, `:162`, `:175`, `:187`, `:202`, `:218`, `:252`, `:286`, `:304`,
    `:322`, `:372`, `:420`, `:447`, `:450`, `:472`, `:485`, `:493`, `:499`, `:549`, `:578`, `:604` (28);
    `dom_selection_proto.rs:123`, `:201`, `:301` (two cites, §3.2 / §3.3, one line), `:411`, `:607`, `:661`
    ("Selection API §3.4"; the `selectionchange` clause is §6.2) (6).
  - **WebIDL, wrong section (3)**: `selection.rs:376` and `dom_selection_mutation.rs:234` ("WebIDL §3.7.6" for
    argument conversion; §3.7.6 is Attributes, the conversion is §3.7.7 Operations through the overload
    resolution algorithm, §3.6); `dom_selection_proto.rs:412` ("WebIDL §3.10" for ToUint32; §3.10 is Observable
    array exotic objects).
  - **Correct: 0.**

  **Removed by `tsel` (20)**, each in a doc comment it rewrites or deletes: `selection.rs:124` (the
  `WrongDocument` variant, deleted), `:162` (`range_count`), `:187` (`selection_type`), `:218` (`anchor`), `:252`
  (`collapse`), `:286` (`collapse_to_start`), `:304` (`collapse_to_end`), `:322` (`extend`), `:372` and `:376`
  (`set_base_and_extent`; `:376` is rewritten to "WebIDL §3.7.7 Operations", §4.1), `:420`
  (`select_all_children`), `:447`, `:450` (`add_range`), `:472` (`remove_range`), `:485` (`remove_all_ranges`),
  `:493` (`empty`), `:499` (`delete_from_document`), `:549` (`contains_node`), `:604` (`same_document`, deleted);
  `dom_selection_proto.rs:607` (the `getRangeAt` native's check comment, replaced). **Left to the sweep (17)**:
  `selection.rs:25`, `:52`, `:55`, `:84`, `:87`, `:120`, `:128`, `:175`, `:202`, `:578`;
  `dom_selection_proto.rs:123`, `:201`, `:301`, `:411`, `:412`, `:661`; `dom_selection_mutation.rs:234`.
  20 + 17 = 37. Outside the property and left too: `dom_selection_proto.rs:661`'s "HTML §8.1.7.1 ("selection task
  source")" (the heading is §8.1.7.1 Definitions; HTML has no "selection task source" dfn; §3.2 row H1). The
  sweep's members in other files are untouched.
- **L-9** `#11-selection-change-on-noop-range-mutation` (Trigger: `tsel`'s landing, PM 891; R1 decision record
  K4; widened by PM 903 and PM 905): stays out of `tsel` (PM 891, decided). Hand the slot the shape `tsel` leaves,
  for both facets:
  - facet (a), a spec no-op must not schedule the event (PM 905: "any Selection operation that is a spec no-op").
    `delete_from_document` returns `Option<Vec<MutationRecord>>`, `Some` for every run of `deleteContents()` (a
    collapsed one included, `Some(vec![])`), and the native schedules on `Some`; the fix there is one branch
    inside `delete_from_document` (collapsed → `None`, §4.1), with no type or native change, and it must not read
    "empty records" as "no boundary mutated" (§4.1's counter-example). `remove_all_ranges` / `empty` return their
    own value (§4.1), `true` today also on an empty selection; the fix there is one edit in `SelectionState`
    (`false` when nothing was dissociated), and their doc comments cite the slot (§4.1).
  - facet (b), a content-script mutation of the associated range's boundary point must schedule the event (PM
    903). Fix site: the Range natives through `write_range` (`range_proto.rs:347`; `setStart` / `setEnd` and the
    other boundary natives the ledger row lists), which `tsel` leaves unchanged (§4.2); `tsel`'s cells assert no
    event count for a Range method call (J15, J16).
- **L-10** Source-sha re-point. (i) The three neighbour slots registered by PM 891 from this plan:
  `#11-selection-select-all-children-steps`, `#11-selection-collapse-null-removes-all-ranges`,
  `#11-selection-change-on-noop-range-mutation`: "at `6a4912de` (branch `tree-model-tsel-plan`, not on main)" →
  "at `<the plan PR's merge commit>` (PR #<n>)". (ii) The PM 903 texts that cite this plan "at `4f1a7d4d` (branch
  `tree-model-tsel-plan`, not on main)": the widening paragraph of `#11-selection-change-on-noop-range-mutation`'s
  row and the row of `#11-selection-text-control-branches`: the same re-point.
- **L-11** concurrent branch `layout-nodekind-inference-shim-removal` (head `712b32d5`, slot
  `#11-nodekind-inference-shim`, not on main at writing) edits `selection.rs` too: it replaces the private
  `is_doctype` with `dom.is_document_type` at the four DocumentType checks and deletes the helper. Rule: whichever
  of the two code PRs lands second merges `main` and re-applies `tsel`'s DocumentType-check reorder (§4.1 table)
  on the other's form of the check; the semantics do not interact.
- **L-12** the ECS-mission ratchet allowlist: at `origin/main` `6cbe39e3`, `.claude/tools/ecs-mission-allowlist.tsv`
  rows 39-40 (class T, slot `#11-core-browser-parity-deviations`). Rows are keyed by text (columns class, slot,
  path, normalised text; no line number), not by line. Row 39 is the line ``/// - `Directionless`: spec doesn't pin
  a side; …`` (`selection.rs:221`, in `anchor`'s doc comment; full text in the row, `sed -n 39p` of the allowlist
  at `6cbe39e3`): `tsel`'s rewrite of that doc
  comment keeps this line **byte-identical** (direction is outside `tsel`, §0.2; the row's slot keeps its member).
  Row 40 is `node_fully_contained`'s doc-comment line (`selection.rs:650`), which `tsel` does not edit. If the code
  PR finds it must change either line, it stops and reports to the PM with the slot's population before and after.
  The code PR re-runs the ratchet (at `3cbd5b88` the file had five `selection.rs` rows, 58-62; #655 reclassified
  them).
- **L-13** (re-check decision record M9; to L3, the `tq` lane) `tq`'s Changelog entry in the umbrella. Per PM 898
  `tq`'s plan PR lands after `tsel`'s and merges `main`; on that merge, `tq`'s Changelog entry joins `tsel`'s
  `**Changelog.**` paragraph (§0.4 (e)) in the #647 form, newest first. A mechanical edit on the `tq` branch, done
  by L3; not an erratum part of this plan.

## §9. Gate

The slice intersects SEL, ROOT, DOC, ERR, MUT, LRR and RW (§2: ≥3 axes), so the *Edge-dense* rule applies: this per-PR
plan passes `/elidex-plan-review` in full, including the erratum parts (a)-(e) of §0.4, before any code. Then
one PR, through `/pre-push`, cut after this plan's landing. It lands before `tq` (umbrella §5.1 erratum); `tq`'s
P-5 is a precondition on this landing.

## §10. Definition of done

1. §4.1 and §4.2 are implemented; `SelectionError::WrongDocument`, its mapping arm and `same_document` are gone;
   `getRangeAt`'s checks are in `range_at`; the attributes' clause is in `SelectionState`; no algorithm in
   `vm/host/`.
2. E1-E13 and J1-J16 are green; the §7.2 re-derivations are applied; every mutant of §7 turns its cells red (run
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
  so step 5's "If anchor is before focus" (webref 外 (raw cache, selection-api 2026-06-11)) has no spec answer for a cross-root pair (Q-1). `""` and "nothing
  removed" hold on either branch, so `tq`'s cells are not affected.
- **P-6** "find where the umbrella keeps its changelog" (the driver's addition): the umbrella has no changelog
  line; its revision record is inside the Status block. The line was added as a new `**Changelog.**` line after
  the Status block, which is unchanged (§0.4).
- **P-7** `select_all_children`'s raw writes (`:439-440`, ledger row *Site*) are not a defect: steps 4-5 say "Set
  newRange's start" (webref 外 (raw cache, selection-api 2026-06-11)), not "set the start". The facet there is step 2 only.
- **P-8** The VM selection fixture (`TREE_SETUP`) is detached, and its comment's reason is stale (§7.2): 28 cells
  depend on today's lax check. Not in the brief or the slot draft.
- **P-9** (resolved by PM 891 (5a)) The brief calls the umbrella "ratified"; its Status block read "umbrella, draft
  3.5 (pre-implementation)" and recorded no ratification. The erratum's part (a) writes it in: "ratified (PM 848,
  #648 `4918a6c7`)" (§0.4).
- **P-10** PM 885's list of methods omits `addRange()` step 1, the same "root is the document" precondition on a
  boundary setter (probes 2e, 2f). Included by the lens of §0.5; confirmed PM 891.
- **P-11** PM 890 (verified): `SelectionState::delete_from_document` (`:513-546`) has no document-tree check, only
  the emptiness check at `:518`; its native has none either and schedules `selectionchange` unconditionally
  (`dom_selection_mutation.rs:147`). The function also takes no document today, so it gains one (§4.1).
- **P-12** (resolved by PM 897; Rev 1.1, found while quoting `getRangeAt()`) The document-tree clause of facet (6) is not specific to
  `getRangeAt()`: `anchorNode`, `anchorOffset`, `focusNode`, `focusOffset`, `rangeCount` and `type` carry it too
  (webref cache lines 233-299; `rangeCount` quoted in §0.1 item 7), and none of them checks it today. PM 891 brings in
  `getRangeAt()` only. So after `tsel`, a selection inside a shadow tree has `rangeCount` 1 while `getRangeAt(0)`
  throws (§2 read × read); before `tsel` the two agreed (both wrong), and the spec has them agree. A loop `for (i <
  rangeCount) getRangeAt(i)` throws on such a selection after `tsel`. **Resolved by PM 897**: the clause is in
  (§0.1 item 7, E11, J13, M16-M19), on the decisive reason that `tsel` must not create that inconsistency.
- **P-13** (resolved by PM 897; Rev 1.1) Four VM cells of Rev 1 asserted values through those attributes for a point inside the shadow
  tree, where the spec's value differs: J1's `type` "Caret" (spec "None" if the collapse lands in the shadow tree,
  Q-1), J2's `anchorNode === st` / `focusNode === st` (spec `null`), J9's `anchorNode === st` (spec `null`), J10's
  "the selection still (`st`, 0)-(`st`, 3)" (observable only through those attributes or `getRangeAt`, which now
  throws). Each is rewritten to assert only spec-fixed values (`isCollapsed`, `toString()`, `getRangeAt`'s throw,
  the `selectionchange` count); the points stay asserted at the dom-api level (E2, E6, E8), where `anchor` /
  `focus` are the concepts, not the attributes. The cells' coverage of M1, M2, M5 and M12 is kept (§7 table).
  **Resolved by PM 897** (Rev 1.2): with the clause in, the spec decides those values, so J2, J9 and J10 assert
  them (`null`, `0`, `"None"` for a selection inside the shadow tree), and J1 asserts the relations that hold
  wherever its collapse lands (Q-1). Kills kept: M1 and M2 by J2 (`isCollapsed`; M2's caret at (`post`, 2) reads
  `post`), M5 by J9 (the event count, `collapseToStart()`), M12 by J10.
- **P-14** (resolved by PM 897; Rev 1.1, for the PM) Q-3's "Selection direction" item had two halves: `selectAllChildren()` step 7
  (N4) and `current_direction` returning `Directionless` for every collapsed range. The second is not specific to
  `selectAllChildren`: `extend()` ("If newFocus is before oldAnchor, set this's direction to backwards. Otherwise,
  set it to forwards."; webref 外 (raw cache, selection-api 2026-06-11)) and `setBaseAndExtent()` ("If focus is before anchor, set this's direction to backwards.
  Otherwise, set it to forwards"; webref 外 (raw cache, selection-api 2026-06-11)) set *forwards* for a collapsed result, so `direction` is `"forward"` there, and
  the existing VM cells `extend_collapsed_back_to_anchor_is_directionless` and
  `set_base_and_extent_collapsed_is_directionless` pin `"none"`. By its name, slot
  `#11-selection-select-all-children-steps` covers the first half; whether it also covers the second, or the second
  needs its own slot, is the PM's (the draft's text was not read here). **Resolved by PM 897**: the slot is widened
  to "Selection direction steps (selectAllChildren / extend / setBaseAndExtent set forwards on a collapsed range;
  current_direction is the single site) + selectAllChildren child count", and the four tests that pin `"none"` /
  `Directionless` are its (§0.2). `tsel` changes no direction and no cell asserts one (§7).
- **P-15** (Rev 1.2, found while placing facet 7; **ratified, PM 901**) `addRange()` step 2 is "If `rangeCount` is not `0`" (§3), so the
  attributes' clause reaches a write path: a selection outside the document tree no longer blocks `addRange()`,
  which replaces it (probe A8: base aborts). Decided by the lens of §0.5 (every row selects reading `range_count`);
  reported, not asked. It brings a second change the brief did not list: `native_selection_add_range` writes the
  slot itself and never displaced a range, so it moves onto `mutate_selection` to free the displaced id (§4.2,
  J14, M21). Before this, `addRange` was the one Selection write outside `mutate_selection`. **Ratified by PM
  901**: `addRange()` replaces a selection that lies outside the document tree; the native moves onto
  `mutate_selection` (one site for "a Selection write replaced a range"); an evicted `RangeId` is freed only if
  no wrapper holds it; ground: Selection API `addRange()` steps 1-2 under the tree clause, and One issue, one
  way. PM 901's condition is met in Rev 1.3: the wrapper held during the displacement and the ranges moved to a
  shadow tree / a detached tree / another document have cells (E13, J15, J16), and the unconditional-unregister
  mutant (M23) is in §7's table, so "no wrapper holds it" is proven by a cell. Rev 1.3 also widens the lens
  row's ground (§0.5): the replaced range can be one a Range method moved out, and wrapper-held (§0.1 item 8).
- **P-16** (Rev 1.2, found while rewriting the cells) Rev 1.1's J4, J5 and J6 asserted an abort as "`rangeCount`
  0" / "selection unchanged". After facet (7) a selection wrongly set outside the document tree (a detached node, a
  shadow-tree node, a foreign node) also reads `rangeCount` 0, so those reads no longer tell an abort from a set.
  They now read `toString()`, `isCollapsed` and `collapseToStart()`'s "InvalidStateError" (empty only), which carry
  no clause (§7, the note before J1). The dom-api cells read `current_range_id` and were not affected.

- **P-17** (Rev 1.3, R1 decision record K6 A3 MIN-3) The two ✗ cells of §3 for text controls (the stringifier's
  `textarea` / `input` branch, and `selectionchange` for an `input` / `textarea` selection) are held by no ledger
  slot: the ledger's Selection slots (`/usr/bin/grep -o -i '#11-[a-z0-9-]*selection[a-z0-9-]*'` over
  `project_open-defer-slots.md`, 2026-11-21: `#11-selection-boundary-setter-skips-root-check`,
  `-document-membership`, `-select-all-children-steps`, `-collapse-null-removes-all-ranges`,
  `-change-on-noop-range-mutation`, `-error-fold`, `-offset-utf16-units`, `-mid-surrogate-fidelity`,
  `#11-readonly-selection-edit-bypass`, `#11-text-control-set-selection-range-caret`, the image-source one, and
  the closed `#11-traversal-and-range-pr-b-selection`; 12 ids) name neither. Not tracked; reported for the PM, not in `tsel`.
  **Resolved by PM 903**: both are held by the new slot `#11-selection-text-control-branches` (registered
  2026-11-21), which makes 13 selection slot ids in the ledger; §3's two ✗ cells name it.
- **P-18** (Rev 1.3, found while placing K1) A Range method on the selection's range (`getRangeAt(0).setStart(…)`)
  schedules no `selectionchange`, though the spec (Selection API §6.2) schedules one when "the associated range's boundary point is
  mutated either by the user or the content script" (webref 外 (raw cache, selection-api 2026-06-11); cache lines
  1044-1046): the only
  three sites that set `selectionchange_pending` are in `dom_selection_mutation.rs` (`:111`, `:147`, `:184`;
  `git grep -n 'selectionchange_pending = true' -- crates`, 3 hits at `3cbd5b88`), and `write_range`
  (`range_proto.rs:347`) sets none. Read from the code, not probed. Not a tree precondition, so not `tsel`'s; no
  ledger slot names it (P-17's search); reported for the PM. `tsel`'s cells assert no event count for a Range
  method call.
  **Resolved by PM 903**: `#11-selection-change-on-noop-range-mutation` is widened (id kept) to both directions,
  facet (a) a no-op must not fire and facet (b) a content-script boundary mutation of the associated range must
  fire; this is facet (b) (§0.2, §3's `selectionchange` ✗ cell, §8 L-9).
- **P-19** (Rev 1.3, while recording K2) The R1 decision record quotes DOM §4.2.1 as "a node tree whose root is a
  document" only; the same section also defines "A node is in a document tree if its root is a document." (`dfn
  dom "in a document tree"` → `#in-a-document-tree`), which says *a* document and so leans to the any-document
  reading at rung (1). It does not decide the question: the Selection API writes "in the [=document tree=]"
  (webref 外 (raw cache, selection-api 2026-06-11); cache line 239, and likewise in the other seven clauses, §0.5)
  with the definite article and links `#concept-document-tree`, not `#in-a-document-tree`. K2's rung-(2) decision is
  applied as recorded and the sentence is in §0.5's ladder; reported so the PM sees the full rung-(1) text.
- **P-20** (Rev 1.4, re-check decision record M10; **resolved by PM 905**) `removeAllRanges()` / `empty()` on an
  empty selection schedule `selectionchange` today, though nothing is dissociated, and `tsel` leaves that unchanged
  (both return `true` as before, §4.1). Rev 1.3's §4.1 had their doc comments cite
  `#11-selection-change-on-noop-range-mutation`, whose facets then were (a) `deleteFromDocument()` on a collapsed
  range and (b) Range-method mutations (PM 903), neither of which is this case; the slot placement was asked of
  the PM. **PM 905**: folded into facet (a), widened to "any Selection operation that is a spec no-op must not
  schedule selectionchange"; facet (b) stays. So the two doc comments cite the slot (§4.1, §8 L-9).

## §12. Open questions

- **Q-1** (spec gap; not asserted by any cell) `setBaseAndExtent()` step 5 and steps 7 / 9 of
  `setBaseAndExtent()` / `extend()` ask "before" of a pair that can be cross-root, and DOM §5.2 asserts one root.
  The plan keeps `direction_between` as the branch selector (no new order rule), so a cross-root
  `setBaseAndExtent` collapses at the point the chosen branch writes last (with base `compare_points`: the later
  of the two in shadow-including order). The cells assert only what the spec fixes (collapsed, `""`, nothing
  removed). An issue against the Selection API is the PM's call. Basis, rung (2) (R1 decision record K6 A2
  MIN-3): no new order rule in `tsel`; base's `compare_points` answer (shadow-including order) is kept as base,
  because the cross-root order relation is `tr`'s (umbrella §5, §2 UP-T × RANGE) and a second rule here would be a
  second home for it (One issue, one way); `tr` owns it (§2 `tr × tsel`, §8 L-6).
- **Q-2** (resolved, PM 891) Does `containsNode()`'s root clause join this slice? For: one predicate, `same_document`
  deleted (one issue, one way), and N2 is a wrong `true` today. Against: it is a read outside PM 890's list.
  Answer: in (§0.1 item 5, E9, J11, M14).
- **Q-3** (resolved, PM 891) Candidate slots for §0.2's defects: `getRangeAt()`'s document-tree clause (N5) is in
  (§0.1 item 6, E10, J12, M15); Selection direction (N4, and `current_direction` on a collapsed range) and
  `selectAllChildren`'s child count → `#11-selection-select-all-children-steps` (widened by PM 897, §11 P-14);
  `collapse(null)` (N1) → `#11-selection-collapse-null-removes-all-ranges`; `selectionchange` after a collapsed
  `deleteContents()` → `#11-selection-change-on-noop-range-mutation`.
- **Q-4** (resolved, PM 897: in; §0.1 item 7) The attributes' document-tree clause (`anchorNode`,
  `anchorOffset`, `focusNode`, `focusOffset`, `rangeCount`, `type`): does it join this slice (the same
  `root_is_document` on six read paths; it removes §2's read × read disagreement that facet (6) creates), or go to
  a slot (then `tsel` ships `rangeCount` 1 with a throwing `getRangeAt(0)` for a shadow-tree selection until that
  slot lands)? Answer: in (§0.1 item 7, E11, E12, J13, J14, M16-M22); the cells are spec-valued (§11 P-13).
