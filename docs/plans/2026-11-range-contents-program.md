# Range contents program (umbrella plan-memo)

Slots: `#11-range-full-impl` (cloneContents / surroundContents / createContextualFragment), `#11-range-extract-partial-ancestor-clone`,
`#11-range-comment-extract-clone`, `#11-range-tostring-boundary-text`, plus the defects this memo finds (§0.4).
Base: `origin/main` `ee6c7a1a`. Owner lane: L3 DOM/form. Decision: PM decision 202 (program = umbrella; §0.1).

**Rev 1** (2026-11-01) — first draft. Every fact below is re-runnable at `ee6c7a1a`: file:line, or the command that
produced it. Line numbers move; the commands do not.

**Rev 2** (2026-11-01) — plan-review R1 (0 CRIT / 8 IMP / 27 MIN). New defect D13 (shadow-host raw child index, RC-1);
D4 gets its own proposed ID; D10/D11 are fixed in RC-1 (was RC-3); D8 is measured on a non-singleton document and,
with C3 and P4, reads the node document through C0's `EcsDom::node_document`; RC-2's node-document work depends on
C0 (Layout); RC-4 consumes IBP-sandbox N9's Fragment-mode producer instead of deriving it; the created set for the
post-clone passes is the existing `ClonedFrom` marker (driver decision; conditions in §4.3; the cloner's
recording variant is dropped); the engine→host error
contract is stated (§4.3); the PI boundary is rejected up front (E1); E2 names seven methods; green-branch rules for
D8 / D12; pairs B × E and X × C; §6 gains the §4.10 / §4.5 / Selection API § drifts and the 4-pattern sweep; PR 0a /
0b renamed RC-0a / RC-0b; §9 states suspension points and cross-lane order; Open questions are now one list for the
PM. Code facts re-verified at `ee6c7a1a`.

## §0. Frame, premises, decisions

### §0.1 The decision and how this memo refines the cut

PM decision 202 fixed an umbrella with four slices: (1) clone/extract canonical, (2) surroundContents, (3) toString,
(4) createContextualFragment. The memo keeps all four and the one-algorithm rule for clone/extract. It changes the
order and adds one mechanism, both forced by the code:

| PM 202 slice | Here | Why it moved |
|---|---|---|
| 3 toString | **RC-1** (first), with the shared containment primitives and `deleteContents` | toString's defect and a `deleteContents` / `extractContents` defect have one root: both walk pre-order and `break` at `end_container` (`crates/dom/elidex-dom-api/src/range/mutation.rs:50-52` and `:93-95`). The fix is one containment primitive. If it landed with clone/extract and toString kept its own walk until slice 3, two enumerations of "nodes in the range" would coexist (strangler). So the primitive lands once and every consumer moves in that PR. |
| 1 clone/extract | **RC-2** | Unchanged in substance: one algorithm parameterised by clone vs move. It consumes RC-1. Its node-document part waits for C0 (§9). |
| 2 surroundContents | **RC-3** | Unchanged. It consumes RC-1's fixed `insertNode` (§0.4 D10/D11 are fixed in RC-1, which already rewrites `insertNode`). |
| 4 createContextualFragment | **RC-4** | Unchanged. Independent of RC-1..3 (parser path); it consumes IBP-sandbox N9's Fragment-mode producer (§5.4, §9). |
| — | **RC-0a / RC-0b** | Prereq pure-move splits (§7). |

The added mechanism is the **live-range reference** (§4.2, RC-1): the mutating algorithms read and write the
*registered* live range at the spec's own steps, instead of snapshot-then-commit. The surround algorithm (extract,
then replace-all, then insert, then select, all on the same live range) cannot be written faithfully on a snapshot,
and `insertNode` already shows a silent-wrong caused by the snapshot (§0.4 D12).

The renumbering, the reorder, the added mechanism and the D10 / D11 move are ratification items for the PM (Open
questions, R1-R4).

### §0.2 Premises re-measured at `ee6c7a1a`

- Stubs: `clone_contents` returns `None` (`range/mutation.rs:601-603`), `surround_contents` returns `None`
  (`:610-612`), `create_contextual_fragment` returns `None` (`:637-639`). The VM natives brand-check and throw
  `NotSupportedError` (`crates/script/elidex-js/src/vm/host/range_proto_mutation.rs:213-262`). Visible, not silent.
- toString: `range/mutation.rs:16-68`. Different containers: the walk starts with `next_in_preorder_global(start_container)`
  (`:49`), i.e. the first child of an Element start container, whatever `start_offset` is; it stops when it reaches
  `end_container` (`:50-52`), so descendants of an Element end container before `end_offset` are never visited; it
  takes only `NodeKind::Text` (`:53`), not CDATASection.
- `contained_top_level_nodes` (`range/mutation.rs:89-130`): the same walk, stopping at `end_container` (`:93-95`), then
  filtered by the §5.5 "contained" test (`:104-112`). The filter removes over-collection; nothing restores what the
  walk never reached. Consumers: `delete_contents` (`:216`), `extract_contents` (`:362`).
- Final boundary: `delete_contents` and `extract_contents` collapse to `(start_container, start_offset)` (`:235-236`,
  `:399-400`). DOM `dom-range-deletecontents` steps 5-8 and `concept-range-extract` steps 12-15 set
  `(newNode, newOffset)`, which differs when the start node is not an inclusive ancestor of the end node.
- Extract clones: boundary Text clones are built with `dom.create_text` (`:303`, `:354`, `:392`), the fragment with
  `dom.create_document_fragment()` (`:266`); both pass `owner = None` (`crates/core/elidex-ecs/src/dom/mod.rs:492-494`,
  `:519-521`). No clone goes through the canonical "clone a node"
  (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:227` `clone_node_with_shadow_honor`).
- Extract guards: `NodeKind::Text` only (`:278`, `:333`, `:373`). Comment and CDATASection boundaries are skipped.
- VM commit: delete/extract clone the registered range, run the engine on the clone, then overwrite the registered
  range (`range_proto_mutation.rs:26-66`, `:83-99`, `:112-125`). `insertNode` does not (it reads start + a pre-call
  `was_collapsed`, `:144-153`, and applies step 13 only when `was_collapsed`, `:190-200`).
  `delete_from_document` repeats the clone-and-overwrite (`crates/dom/elidex-dom-api/src/selection.rs:513-546`).
- `EcsDom::append_child` checks only `parent == child`, existence and cycles (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:22-31`);
  `apply_append_child` adds only the ShadowRoot rejection and the fragment cycle check
  (`crates/script/elidex-script-session/src/mutation/mod.rs:480-497`). The canonical §4.2.3 check is
  `ensure_pre_insertion_validity` (`crates/dom/elidex-dom-api/src/child_node/mod.rs:251`), unused by `insert_node`.
- Clone post-passes run in the VM `cloneNode` shim per clone root: `enqueue_subtree_upgrade_reactions`
  (`crates/script/elidex-js/src/vm/host/node_methods_extras.rs:221`) and `elidex_form::apply_clone_form_state` (`:230`).
  `apply_clone_creation_ce_semantics` "MUST only be called on a freshly cloned subtree"
  (`crates/dom/elidex-custom-elements/src/upgrade.rs:121-123`).
- Parser path: `elidex-dom-api` has no `elidex-html-parser` dependency (`crates/dom/elidex-dom-api/Cargo.toml:14-19`);
  `elidex-script-session` has it (`crates/script/elidex-script-session/Cargo.toml:30`). innerHTML: the VM calls
  `apply_set_inner_html` (`crates/script/elidex-js/src/vm/host/dom_inner_html.rs:146`), which calls
  `parse_fragment_progressive` (`crates/script/elidex-script-session/src/mutation/html_fragment.rs:91`).
- `EcsDom::is_html_namespace` is implemented (`crates/core/elidex-ecs/src/dom/classify.rs:136-138`, namespace check);
  introduced as a stub in #186 (`753362c7`), implemented in #256 (`b6af9310`):
  `git log --oneline -G'is_html_namespace' -- crates/core/elidex-ecs`.
- Producers: `git grep -n 'NodeKind::ProcessingInstruction' -- crates ':!*tests*'` = 9 hits, all match arms or
  classification, no spawn; no `createCDATASection` / `create_cdata*` in `crates/` (`git grep -n -i
  'createCDATASection\|create_cdata_section' -- crates` = 0). ProcessingInstruction and CDATASection boundaries are
  not reachable from script; Comment and Text are.
- Child index on a shadow host (D13): `compare_points` (`range/mod.rs:508`) uses `child_index_in_ancestor`
  (`:549-571`), which counts raw siblings with `get_first_child` / `get_next_sibling`
  (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:22-36`, no ShadowRoot skip). A shadow root is linked as a raw
  child of its host (`crates/core/elidex-ecs/src/dom/shadow.rs:203`, `append_child(host, shadow_root_entity)`). The
  other indices skip it: `node_length` and `child_index` use `children_iter` (`range/mod.rs:481-503`;
  the `Iterator` impl for `ChildrenIter` skips `ShadowRoot`, `crates/core/elidex-ecs/src/dom/mod.rs:865-883`), and the
  `MutationEvent` index the live-range bridge receives is `EcsDom::index_in_parent` (`navigation.rs:61-75`, skips
  ShadowRoot), fired at `tree/mutation.rs:46-47`, `:65-73`.
- `ownerDocument` getter: `dom.owner_document(this).or_else(|| dom.document_root())`
  (`crates/dom/elidex-dom-api/src/node_methods/core.rs:279`). An unstamped node in the bound document's VM reports
  `document`, so `r.extractContents().ownerDocument === document` is green before any fix (it masks D8). C0 keeps
  this line's code (C0 memo §5.1 P-3 lists `core.rs:279` under "values change, code does not").
- Shallow clones: `clone_recording(deep = false)` → `EcsDom::clone_node_shallow` stamps no `AssociatedDocument`
  (`node_methods/clone.rs:313-327`; the shadow branch stamps its one shallow clone by hand, `:294-301`). Spec: "clone a
  single node" step 8 "Set copy's node document to document" for every copy (`webref body dom concept-node-clone`).
- `ClonedFrom` (`node_methods/clone.rs:77-82`): attached by pass 2 only to `<input>` / `<textarea>` clones
  (`mark_cloned_form_control`, `:368-387`, "tag-gated, so a large deep clone does not spray a marker"), read and
  removed by `elidex_form::apply_clone_form_state` over the clone root's template-inclusive descendants
  (`crates/dom/elidex-form/src/clone.rs:44-63`). A left-behind marker is documented as harmless *for the form
  consumer* (`node_methods/clone.rs:60-64`; `crates/core/elidex-ecs/src/dom/tree_clone.rs:29`). The CE pass
  (`apply_clone_creation_ce_semantics`, `crates/dom/elidex-custom-elements/src/upgrade.rs:125-156`) walks the root's
  shadow-inclusive descendants (not template contents) and acts only on entities carrying `CustomElementState`
  (`prepare_upgrade`, `:60-64`), which pass 1 attaches to a clone whose source has one (`propagate_ce_identity`,
  `node_methods/clone.rs:344-366`). Production callers of `clone_node_with_shadow_honor`: `CloneNode` only
  (`node_methods/clone.rs:154`), reached from the VM shim `native_node_clone_node`
  (`vm/host/node_methods_extras.rs:212`), which runs the CE pass (`:221`) then the form pass (`:230`):
  `git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates`.
- Error types: the Range engine methods return no error today (`range/mutation.rs:147`, `:265`, `:427-431`).
  `DomApiError { kind: DomApiErrorKind, message }` (`crates/script/elidex-script-session/src/types.rs:149-154`,
  `:195-219`, `#[non_exhaustive]`) has no `InvalidNodeTypeError` variant (surroundContents step 2 throws it).
  `RangePointError` (`range/boundary.rs:12-19`, with `InvalidNodeType`) serves `compare_point` /
  `is_point_in_range` only and is mapped in `vm/host/range_proto.rs:820-840`; `SelectionError`
  (`selection.rs:119-133`) is Selection's.
- Cross-lane ownership (read-only): IBP-sandbox N9 `IBP-sandbox-parser-mode` owns F11, the Fragment parser mode
  and the re-point of `range/mutation.rs:632` (`docs/plans/2026-09-ibp-sandbox.md:182-183`, `:538`); its input row
  makes one fragment entry derive the mode through F10 with `NodeDocumentOf(context)`
  (`docs/plans/2026-09-ibp-sandbox-input.md:66`); F11 applies at every production fragment-parse caller
  (`docs/plans/2026-09-inline-box-predicate.md:1773`). `#11-range-full-impl`'s re-eval was extended to 2026-11-01
  with N9's plan-memo as venue (`docs/plans/2026-09-ibp-sandbox.md:763`). C0 (`EH-attrs-at-creation-C0`, Layout,
  `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` rev 1 on branch `layout-dom-c0-insert-adopt-plan`,
  `fade8a19`, not on main) adds `EcsDom::node_document` (NEW: C0 D-1) and deletes `owner_document`'s tree-position fallbacks
  (D-1, `:110-121`), adopts at the three linking primitives but leaves a child's stamp alone under an ownerless
  parent (D-4, `:140-146`), lists `Range` insertion among the paths that reach those primitives (C0-1, `:382`), and
  leaves the census of ownerless creations to a scout question (P-8 `:447-448`, Q-S2 `:667-668`).

### §0.3 Premise errors in the brief (reported, not edited)

1. "extract steps ~18/21" for the partially-contained-ancestor clone: the current `concept-range-extract` has them
   at **steps 17 and 20** (step 18 = append contained children, 21 = return). The in-file cites are off by one in the
   same way (§6).
2. "spec 'insert' steps make the parent the start node's parent" for a Comment / PI start: `concept-range-insert`
   **step 1 throws HierarchyRequestError** when the start node is a ProcessingInstruction or Comment (or a parentless
   Text, or `node` itself). The parent-of-start rule (steps 3-5) applies to Text only. The suspicion is still right,
   for a different reason (§0.4 D10).
3. "toString `mutation.rs:16-86`": the function is `:16-68`; `:70-130` is `contained_top_level_nodes`.
4. "createContextualFragment: context = start node if Element else its parent": HTML §8.5.7 uses the **parent
   element**, and only for Text or Comment; any other start node (Document, DocumentFragment, ProcessingInstruction)
   gives null, and null (or an HTML-namespace `html` element in an HTML document) is replaced by a newly created
   `body` (`webref body html dom-range-createcontextualfragment`).
5. "check what allow-declarative-shadow flag createContextualFragment uses": it calls the "fragment parsing algorithm
   steps" with mode **Legacy**, so allowDeclarativeShadowRoots = **false**, and with runScripts = true, so the parser
   scripting mode is **Fragment**, not Inert (`webref body html fragment-parsing-algorithm-steps`). The scripting mode
   is a difference from innerHTML that the brief does not mention (§5.4).
6. "`clone_node_deep` already exists on EcsDom" (in-file, `mutation.rs:595`, and implied by "existing EcsDom
   create_*/clone helpers"): there is no `clone_node_deep`; the ECS cloners are `EcsDom::clone_subtree` and
   `EcsDom::clone_node_shallow` (`crates/core/elidex-ecs/src/dom/tree_clone.rs:216`, `:251`), and **neither is the
   right entry**: Range clones must use the canonical `clone_node_with_shadow_honor` plus the VM post-passes (§2 C1-C2).
7. "B1.2d-ii §11 deferred the partial ancestor deep-clone": the spec clone of a partially contained child is
   **shallow** (`concept-range-extract` step 17.1 "a clone of firstPartiallyContainedChild", no subtree flag), and its
   contents come from the recursive subrange. B1.2d-ii §11 (`docs/plans/2026-06-b1.2d-ii-range-records-plan.md:212`)
   says "deep-clone" and cites "steps 7-15"; neither matches.
8. `#11-range-full-impl` also covers **surroundContents** (ledger row: "cloneContents + surroundContents +
   createContextualFragment"); the brief names only its clone and contextual-fragment halves.

Rev 2, premises in the R1 dispositions that the code or the spec corrects (applied in the corrected form):

9. "one exposed-index derivation shared by `compare_points` and the MutationEvent index": the MutationEvent index is
   already the exposed `EcsDom::index_in_parent` (§0.2). The shared derivation exists; D13's fix moves
   `compare_points` (and `range/mod.rs` `child_index`) onto it.
10. "Steps 1 and 6 go onto `ensure_pre_insertion_validity`": only step 6 is ensure pre-insert validity; step 1 is
    the insert algorithm's own HierarchyRequestError check (`webref body dom concept-range-insert`, steps 1 and 6).
11. "`ClonedFrom` … a moved original never carries it": true, but the marker is tag-gated to `<input>` /
    `<textarea>` today (§0.2), so it does not mark the clones the CE pass acts on. The decision holds only with the
    conditions in §4.3 (gate widened to the CE pass's own predicate; one seam that always sweeps).
12. F5's list of § drifts is incomplete: the sweep (§6 method) also finds `range/tests.rs:529` (§4.10 for split) and
    `range/tests.rs:597` (§4.5 for normalize), and the Selection API §3.1 / §3.2 / §3.3 family in `selection.rs`.

### §0.4 Defect inventory (each silent-wrong row has a negative control in its slice's DoD)

| ID | Defect (observable) | Kind | Evidence | Home | Slot |
|---|---|---|---|---|---|
| D1 | toString, Element start container: includes text of children before `start_offset` | silent-wrong | §0.2 toString | RC-1 | `#11-range-tostring-boundary-text` |
| D2 | toString, Element end container: drops text of descendants before `end_offset` | silent-wrong | §0.2 toString | RC-1 | same |
| D3 | toString skips CDATASection text | silent-wrong, Rust-only reachable | `mutation.rs:53` | RC-1 | same |
| D4 | deleteContents / extractContents, Element end container: contained descendants before `end_offset` are neither removed nor moved | silent-wrong | `mutation.rs:93-95` | RC-1 (both halves: delete, and extract's interim enumeration) | new: proposed `#11-range-contained-enumeration-end-element` |
| D5 | deleteContents / extractContents / `Selection.deleteFromDocument` leave the range at `(start, startOffset)` instead of `(newNode, newOffset)` | silent-wrong | `mutation.rs:235-236`, `:399-400`; `selection.rs:537-542` | RC-1 (delete, Selection), RC-2 (extract, rewritten) | new: proposed `#11-range-delete-extract-new-boundary` |
| D12 | `insertNode` evaluates step 13 "if range is collapsed" before the insert; e.g. `r.selectNode(x); r.insertNode(x)` → spec end stays `(P, i+1)`, elidex predicts `(P, i)` (step 9's remove collapses the live range; step 13 is skipped) | silent-wrong (predicted by reading; measured first in RC-1; green-branch rule below) | `range_proto_mutation.rs:149`, `:190` | RC-1 | new: proposed `#11-range-insert-node-step13-timing` |
| D13 | Boundary-point comparison on a shadow host counts the ShadowRoot as a child: `h.attachShadow({mode:'open'}); h.append(a, b)`; range `(h, 1)`-`(h, 2)` reports `a` as contained (and `isPointInRange(h, 1)` / `comparePoint` / deleteContents on such ranges act on the shifted index) | silent-wrong (by reading, §0.2; measured first in RC-1) | `range/mod.rs:549-571` | RC-1 | new: proposed `#11-range-compare-points-shadow-index` |
| D10 | `insertNode` with a Comment start: no throw; predicted: `node` becomes a child of the Comment | silent-wrong (predicted; measured first in RC-1) | §0.2 append_child | RC-1 | new: proposed `#11-range-insert-node-validity` |
| D11 | `insertNode` step 6 checks cycles + ShadowRoot only, not the full "ensure pre-insert validity" (e.g. a doctype into an element, a Text into a Document) | silent-wrong (predicted; measured with D10) | `mutation.rs:476-494` | RC-1 | same as D10 |
| D6 | extractContents leaves partially contained ancestors in the tree and flattens their contained descendants into the fragment | silent-wrong | §0.2 extract | RC-2 | `#11-range-extract-partial-ancestor-clone` |
| D7 | extractContents with a Comment boundary: same-container is a no-op; cross-container skips the Comment's clone and splice | silent-wrong | `mutation.rs:278`, `:333`, `:373` | RC-2 | `#11-range-comment-extract-clone` |
| D8 | extractContents fragment / Text clones have no node document (`ownerDocument`) | silent-wrong (predicted; measured first in RC-2 on a non-singleton document; green-branch rule below) | `mutation.rs:266`, `:303`; masked in the bound document by `node_methods/core.rs:279` (§0.2) | RC-2, after C0 (§9) | new: proposed `#11-range-contents-node-document` |
| D9 | extractContents does not throw HierarchyRequestError for a contained doctype (step 11); e.g. `r.selectNodeContents(document); r.extractContents()` moves the doctype | silent-wrong | no doctype check in `mutation.rs:265-402` | RC-2 | new: proposed `#11-range-contents-doctype-check` |
| — | cloneContents unimplemented | visible (throws) | §0.2 stubs | RC-2 | `#11-range-full-impl` (clone part) |
| — | surroundContents unimplemented | visible | §0.2 | RC-3 | `#11-range-full-impl` (surround part) |
| — | createContextualFragment unimplemented | visible | §0.2 | RC-4 | `#11-range-full-impl` (contextual part) |

**Classification and ledger handling.** Every row is **pre-existing** at `ee6c7a1a` (none is introduced by this
program), and every proposed ID is **closed by the program's own home slice**. Slot IDs marked "proposed" are for the
PM to register or fold; the memo does not edit the ledger. If the PM folds a proposed ID into the program instead of
registering it, that ID still needs a "Not registered by design" homing row while it appears in memo text
(`memory/reference_slot-homing-checker.md` rule (d)). Each proposed slot's fields: defect = the table row; **Why
deferred** = the fix lands in its home slice, in the umbrella's order (§9), not in a separate PR; trigger = the home
slice's per-PR plan-review; re-eval date = **2026-12-01** (proposed), or that slice's landing if earlier.

**The `insertNode` measurement (D10/D11), and how it decides.** RC-1's per-PR plan runs it first, at the pre-RC-1
head, as VM tests: (a) `r.setStart(comment, 0); r.insertNode(span)` expects HierarchyRequestError and an unchanged
tree; (b) `r.setStart(div, 0); r.insertNode(doctypeClone)` and `r.setStart(document, 0); r.insertNode(text)` expect
HierarchyRequestError. Decision rule:
- **Red** (predicted): fixed in **RC-1**, which already rewrites `insertNode` (D12). "Insert a node into a live
  range" step 1 (its own HierarchyRequestError check: PI or Comment start, parentless Text start, start is `node`)
  runs first, and step 6 calls `ensure_pre_insertion_validity` (`child_node/mod.rs:251`), both before any mutation
  (E1). The red tests are RC-1's negative controls. RC-3 consumes the fixed insert unchanged.
- **Green** (some layer already rejects): no fix; the tests are committed in RC-1 as regression pins, and RC-1 still
  routes step 6 through `ensure_pre_insertion_validity` (one validity check, not a lucky layer).

**Green-branch rule for the other predicted rows (D8, D12, D13).** Each is measured first by its home slice's
negative control at the pre-fix head. Red → fixed in that slice, the test is the negative control. Green → no fix for
that row; the test is committed as a regression pin and the PR body records the green run; the proposed slot is not
needed. D8's control must not run in the bound document, where the getter fallback masks it (§0.2): it runs on a
non-singleton document (a DOMParser document or `document.cloneNode(true)`; the per-PR plan picks the producer), or
reads `EcsDom::node_document` at the Rust unit level.

## §1. Shape and verdict

**Edge-dense** (CLAUDE.md "Edge-dense work"): the work intersects boundary-point containment, MutationRecord
emission, live-range updates during the algorithm's own mutations, the clone-a-node pipeline (custom elements, form
state, template contents, shadow roots), and for RC-4 the parser / session path (§2). So: this umbrella, then one
per-PR plan per row below, each through **`/elidex-plan-review` before implementation** (rule, not judgment). A
per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| RC-0a | Pure-move test splits: `range/live.rs`, `selection.rs`, `range/tests.rs`, VM `tests_range.rs` (§7) | — | — | not required (pure move; `git diff --color-moved`) |
| RC-1 | Containment primitives; exposed child index (D13); live-range reference; toString, deleteContents, Selection.deleteFromDocument, insertNode (steps 1, 6, 13) moved onto them; extract's enumeration moved to the canonical set (interim) | RC-0a | `#11-range-tostring-boundary-text`; proposed D4, D5 (delete half), D10, D11, D12, D13 | required |
| RC-2 | One "contents" algorithm for clone and extract; cloneContents; VM post-clone seam shared with cloneNode; node documents of the fragment and every clone | RC-1; **C0** for the node-document part (§9) | `#11-range-extract-partial-ancestor-clone`, `#11-range-comment-extract-clone`, `#11-range-full-impl` clone part; proposed D5 (extract half), D8, D9 | required |
| RC-3 | surroundContents | RC-2 | `#11-range-full-impl` surround part | required |
| RC-0b | Pure-move split of `html_fragment.rs` inline tests (§7) | — | — | not required |
| RC-4 | createContextualFragment through the session fragment-parse path, consuming N9's Fragment-mode producer | RC-0b; **IBP-sandbox N9** (§5.4, §9); independent of RC-1..3 | `#11-range-full-impl` contextual part (the slot closes when RC-2, RC-3, RC-4 have all landed) | required |

## §2. Invariants

Axes: **B** boundary points and containment, **M** MutationRecord emission, **L** live-range updates, **C**
clone-a-node pipeline, **E** errors and `[CEReactions]`, **P** parser / session path (RC-4), **X** layering and
ECS-native.

- **B1 (one containment definition)**: "contained", "partially contained" and "get the common ancestor" are computed
  exactly as WHATWG DOM §5.5 defines them, on boundary-point positions (§5.2, today `compare_points`,
  `range/mod.rs:508`). They live in one module (`range/containment.rs` (NEW)). A tree walk may bound the search, but
  membership is decided by the predicate, and the walk's end is the end boundary point, not `end_container`.
- **B2 (one enumeration)**: every "nodes in the range" set (toString's contained Text nodes; deleteContents'
  nodesToRemove; clone/extract's first/last partially contained child and containedChildren) derives from B1.
  `contained_top_level_nodes` and `next_in_preorder_global` are deleted when their last consumer moves.
- **B3 (units)**: offsets in CharacterData are UTF-16 code units; child indices and node lengths use the same
  exposed child list. The one exposed-index derivation is `EcsDom::index_in_parent` (skips ShadowRoot; already the
  `MutationEvent` index, §0.2); `compare_points`' `child_index_in_ancestor` and `range/mod.rs` `child_index` move onto
  it in RC-1 (D13), so a boundary offset, a node's index and the bridge's event index count the same list.
- **M1 (one mutation path)**: every mutation of a node that existed before the call goes through the record-producing
  primitive: `apply_replace_data`, `apply_remove_child`, `apply_append_child` / `apply_insert_before`,
  `split_text_at_offset`, and the session's replace-all for surround step 4. Appends into containers created by the
  call (the fragment, clones) also use `apply_append_child`; delivery decides observability (the targets are new, so
  nothing observes them). The one raw write allowed is "set data" on a fresh clone (spec says set, not replace).
- **M2 (order and delivery)**: records are produced in spec step order and committed once through
  `commit_notify_records` (`crates/script/elidex-js/src/vm/host/dom_bridge.rs:57`). If an error is thrown after
  mutations ran (surround, §5.3), the records gathered so far are committed before the exception.
- **M3 (moves)**: a move is one `apply_append_child` (the B1.2a two-record move), never remove + append.
- **L1 (the live range itself)**: the target range is the registered live range, reached through a live-range
  reference (`LiveRangeRef` (NEW), §4.2). The algorithm reads its boundary at the spec's "originals" step and writes it
  at the spec's own step: deleteContents step 8, extract step 15, insert step 13 (evaluated after step 12), surround
  step 7 (select). Between those, the existing bridge updates it as each M1 primitive fires. There is no
  snapshot-then-overwrite.
- **L2 (subranges)**: the "new live range" of extract / clone steps 17.3 / 20.3 (13.3 / 16.3 for clone) is a plain,
  unregistered value. Justification: after its own step 3 the recursion reads only the captured originals, and its
  only write (extract step 15 on the subrange) has no reader.
- **L3 (other live ranges)**: change only through the hooks the M1 primitives fire (replace data, remove, insert,
  split). No algorithm adjusts another range directly.
- **L4 (lock)**: the ranges mutex (`range/live.rs` "Lock invariant") is never held across a DOM mutation;
  `LiveRangeRef` locks per read or write.
- **C1 (canonical clone)**: every clone made by clone/extract is "clone a node" through the canonical entry
  (`clone_node_with_shadow_honor`; no recording variant, §4.3), so custom-element identity, template
  contents, clonable shadow roots and the form-control marker follow the same rules as `cloneNode`. A CharacterData
  clone is a shallow canonical clone, then "set data" with the substring.
- **C2 (post-clone passes on created nodes only)**: the registry-dependent custom-element pass and the form-state pass
  visit exactly the entities created by cloning in this call, never the moved originals that extract places under a
  clone. The created set is carried by the existing transient `ClonedFrom` marker (driver decision; conditions in
  §4.3): both passes act only on marked entities, and a moved original never carries one. One VM post-clone seam
  serves `cloneNode` and the Range methods. Template-content clones are outside the CE pass (rule in §4.3), as today.
- **C3 (node document)**: the fragment is created given the start node's node document ("clone the contents" /
  "extract" step 2); every clone, shallow included, gets its operation's document ("clone a single node" step 8).
  Write path: the canonical cloner stamps `AssociatedDocument` on every clone root it returns, shallow included
  (`clone_recording(deep = false)` stamps today nothing, §0.2). That rule is shared with `cloneNode`, so it is also a
  **cloneNode behaviour change** (a shallow `cloneNode()` of an unstamped-document node), listed in RC-2's tests.
  Read path: one function, `EcsDom::node_document` (C0 D-1), used by C3, P4 and D8's control; this program adds no
  second node-document reader (§9 for the C0 dependency).
- **E1 (throw before mutating)**: errors the spec raises before any mutation (extract / clone step 11 doctype; surround
  steps 1-2; insert steps 1 and 6) leave the tree, the records and the range untouched. elidex's own refusal of a
  ProcessingInstruction boundary (PI clones are unsupported, `node_methods/clone.rs:126-131`; PI has no producer,
  §0.2) is also raised before any mutation: the contents algorithm checks **both** boundaries up front, never midway
  through the moves.
- **E2 (`[CEReactions]`)**: seven methods are `[CEReactions]`: Range `deleteContents`, `extractContents`,
  `cloneContents`, `insertNode`, `surroundContents` (`webref idl dom Range`), `createContextualFragment` (`webref
  idl html Range`), and `Selection.deleteFromDocument` (`webref idl selection-api Selection`). Reactions enqueued by
  their mutations (disconnected / connected via the mutation-event leg, upgrades via C2) run with the same timing as
  `appendChild`'s. Each slice measures that timing for its own methods. Decision rule: a timing difference in a
  slice's own methods is a defect fixed in that slice; a difference elsewhere (a method no slice of this program
  touches) gets a proposed slot.
- **E3 (engine→host error contract)**: an engine algorithm that can throw returns, on error, a typed
  `DomApiErrorKind` (`crates/script/elidex-script-session/src/types.rs:195-219`, the kind
  `CloneNode` already returns) **plus the records produced before the error**; `Ok` carries the result and its records.
  `DomApiErrorKind` gains `InvalidNodeTypeError` (surroundContents step 2; the enum is `#[non_exhaustive]`). The host
  maps the kind to a DOMException and commits the records (M2); it decides nothing. Whether `RangePointError`
  (`range/boundary.rs:12-19`) folds into `DomApiErrorKind` is an open item for RC-1's per-PR plan (one issue, one
  way), not decided here.
- **P1 (parser behind the session)**: RC-4 parses only through `elidex-script-session` (no new
  `elidex-dom-api` → `elidex-html-parser` dependency).
- **P2 (Legacy mode)**: allowDeclarativeShadowRoots = false; with #577 a top-level `<template shadowrootmode>` is a plain
  template anyway, and nested ones are plain because the flag is off.
- **P3 (context read-only; no leaked context)**: the live start node or its parent element is never mutated (fragment
  plan I1, `docs/plans/2026-10-fragment-dsd-not-on-context.md` §2); a created `body` context is destroyed before
  return, in every outcome.
- **P4 (result)**: a new DocumentFragment whose node document is the start node's node document; no MutationRecord.
- **X1 (layering)**: algorithms in `elidex-dom-api` (`range/`); `vm/host/range_proto_mutation.rs` does brand check,
  argument conversion, live-range-reference lookup, record commit, post-clone seam call, wrapper creation. No loop or
  walker in `vm/host/`.
- **X2 (ECS-native)**: no new component and no new entity-keyed side-store. `LiveRangeRef` is a handle into the
  existing `LiveRangeRegistry` map (Range objects are not entities). Clones use the existing ECS cloner; the created
  set is the existing transient `ClonedFrom` marker (a component on the clone, read by query), not a returned `Vec`.

Pairs (one line each; pairs not listed do not interact):

| Pair | Intersection |
|---|---|
| B × M | the enumeration order is the record order; nodesToRemove omits nested nodes, so one removal record per top-level node |
| B × L | containment is computed once from the originals captured before the first mutation, never re-evaluated against the tree the algorithm is mutating |
| M × L | the target range is set (L1) before the mutations, so each primitive's hook moves it exactly as the spec's live-range rules do |
| B × C | a partially contained child gets a shallow clone plus a recursive subrange; a contained child gets a deep clone (clone) or a move (extract) |
| M × C | clones are appended through M1 into new containers (records with unobserved targets); C2 runs after the mutations but visits only the created set |
| L × C | cloning touches no pre-existing node, so it fires no live-range update on existing ranges |
| E × M | a late error (surround after extract) commits the records produced so far (M2); an early error produces none (E1) |
| E × L | early error: range unchanged; late error: the range keeps the state the spec steps gave it (no rollback) |
| C × E | step 11 runs before the first clone, and a recursive subrange inside an element cannot contain a doctype, so no clone is created before a doctype throw |
| E × C | C2's upgrade reactions are reactions under E2 |
| B × E | the PI-boundary refusal and step 11's doctype test read the partition B1 computed from the originals, before the first mutation (E1) |
| X × C | the created set crosses the engine→host boundary as the `ClonedFrom` marker on the clones, so the VM seam walks nothing itself (X1); the passes' walkers live in `elidex-custom-elements` and `elidex-form` |
| P × M | parsed nodes go into a new fragment: no record (P4) |
| P × L | createContextualFragment mutates nothing that exists, so no live-range update |
| P × C | parsed elements get their custom-element state from the parser's own derivation (as innerHTML), not from C2 |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §5.2 Boundary points | position of a boundary point | same node / ancestor / descendant / tree order | `compare_points` (`range/mod.rs:508`), child index moved onto `index_in_parent` (D13, RC-1), consumed by B1 | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | contained; partially contained; get the common ancestor | start-side / end-side / both | `range/containment.rs` (NEW) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | stringification behavior steps 2-5 | same Text node / Text start / contained Text / Text end | `Range::to_string` (RC-1) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | deleteContents steps 3, 4, 5-8 (newNode / newOffset), 9-11 | same CharacterData / start inclusive ancestor of end / otherwise | `Range::delete_contents` (RC-1) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | extract steps 1-21 | collapsed / same CharacterData / partial CharacterData start-end / partial element start-end / contained / doctype throw | contents algorithm (RC-2) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | clone the contents steps 1-17 | same as extract, without mutation | contents algorithm (RC-2) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | insert a node into a live range steps 1-13 | PI or Comment start / parentless Text / start is node / Text split / fragment / node is referenceNode / step 13 collapsed | `Range::insert_node` (RC-1: steps 1, 6, 13) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | surroundContents steps 1-7 | non-Text partially contained / Document-DocumentType-DocumentFragment newParent / newParent has children / late throw from insert or append | `Range::surround_contents` (RC-3) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | select a node | parent null throws | `Range::select_node` (`range/boundary.rs:125`), consumed by RC-3 | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | ensure pre-insert validity; pre-insert; insert; append; remove; replace all | per-step rejection | `ensure_pre_insertion_validity` (`child_node/mod.rs:251`); session `apply_*` | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | queue a mutation record | childList / characterData | `commit_notify_records` (unchanged) | ✓ | no |
| WHATWG DOM §4.4 Interface Node | clone a node (step 3 "Run any cloning steps"); clone a single node (step 8 node document) | shallow / deep; shadow host; template; custom element | `clone_node_with_shadow_honor` (`node_methods/clone.rs:227`); `ClonedFrom` gate widened (§4.3); shallow stamp (C3) | ✓ | yes |
| WHATWG HTML §4.10.5 The input element / §4.10.11 The textarea element | cloning steps (reached via DOM §4.4 clone a node step 3) | input / textarea | `ClonedFrom` + `elidex_form::apply_clone_form_state` (input changes to marked entities, RC-2) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | cloning steps (contents cloned with the copy's template contents' node document) | subtree false / true | `replicate_template_contents` (unchanged); outside the CE pass (§4.3) | ✓ | yes |
| WHATWG HTML §4.13.3 Core concepts | look up a custom element definition (step 1: null registry → null) | — | CE pass scope over template contents (§4.3) | ✗ (registry of the inert document unconfirmed, §4.3) | no |
| WHATWG DOM §4.9 Interface Element | create an element | `body` context for RC-4 | element creation helper (RC-4) | ✓ | no |
| WHATWG DOM §4.10 Interface CharacterData | replace data; substring data | Text / Comment / CDATASection | `apply_replace_data` (unchanged); substring in the contents algorithm | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | split a Text node | insert step 7 | `split_text_at_offset` (unchanged) | ✓ | yes |
| WHATWG HTML §8.5.7 The createContextualFragment() method | steps 1-7 | Element / Text-Comment with parent element / null / `html` in an HTML document | `create_contextual_fragment` (RC-4) | ✓ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | fragment parsing algorithm steps | Legacy mode; runScripts; XML document | session fragment parse (RC-4) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm | context = target; fragment node document; scripting mode | `parse_fragment_progressive` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm step 8 (Disabled when contextDocument's scripting is disabled) | Fragment / Disabled | N9's fragment entry (consumed, §5.4) | ✓ | no |
| WHATWG HTML §13.2.4.5 Other parsing state flags | parser scripting mode | Inert / Fragment / Disabled | N9's Fragment-mode producer (consumed, §5.4) | ✓ | no |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | script start tag steps 2-3 (Fragment: no parser document; Inert: already started = true) | Inert / Fragment | N9's producer (consumed); RC-4 tests | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | create an element for a token steps 13, 15 (fragment parse: willExecuteScript false → DOM §4.9 create an element enqueues the upgrade) | definition null / non-null | question (d), §5.4 | ✗ | no |
| WHATWG HTML §4.12.1.1 Processing model | already started | Fragment mode leaves it unset | question (a), §5.4 | ✗ | no |
| Selection API §3 Selection interface | deleteFromDocument | rangeCount 0 / 1 | `Selection::delete_from_document` (`selection.rs:513`, RC-1) | ✓ | yes |

Numbers and titles: `webref heading dom 5.2`, `5.5`, `4.2.3`, `4.3.2` (via `coverage-map`), `4.4`, `4.9`, `4.10`,
`4.11`; `webref heading html 8.5.4`, `8.5.7`, `13.4`, `4.10.5`, `4.10.11`, `4.12.3`, `4.13.3`, `13.2.6.1`,
`13.2.6.4.4`, `4.12.1.1`; `webref heading selection-api 3` (§3 Selection interface; `heading selection-api 3.1` /
`3.2` / `3.3` → "no headings under §3.x", so sub-numbers are unconfirmed and not cited; `body selection-api
dom-selection-deletefromdocument` → anchor not in the rendered chapter, so its step text is unconfirmed); `webref coverage-map dom 5.5 dom 5.2 dom 4.2.3 dom 4.3.2 dom 4.4
dom 4.9 dom 4.10 dom 4.11 html 8.5.7 html 8.5.4 html 13.4 html 13.2.4.5 html 4.12.1.1`. Dfns: `webref dfn dom
contained` / `'partially contained'` / `'get the common ancestor'` (§5.5), `'clone a node'` / `'clone a single node'`
(§4.4), `'replace data'` / `'substring data'` (§4.10), `'split a Text node'` (§4.11), `'ensure pre-insert validity'` /
`'replace all'` / `remove` / `append` (§4.2.3), `select` (§5.5), `'create an element'` (§4.9); `webref dfn html
'fragment parsing algorithm steps'` (§8.5.4), `'parser scripting mode'` (§13.2.4.5), `'already started'`
(§4.12.1.1), `'create an element for a token'` (§13.2.6.1), `'look up a custom element definition'` (§4.13.3),
`'appropriate template contents owner document'` (§4.12.3); `webref dfn dom 'cloning steps'` (§4.4); `webref dfn
selection-api deleteFromDocument` (§3). Prose: `webref body dom concept-range-extract`, `concept-range-clone`, `dom-range-deletecontents`,
`concept-range-insert` (also holds surroundContents), `dom-range-stringifier`, `contained`, `concept-range-select`,
`get-the-common-ancestor`; `webref body html dom-range-createcontextualfragment`, `fragment-parsing-algorithm-steps`,
`html-fragment-parsing-algorithm` (step 8), `parser-scripting-mode`, `parsing-main-inhead` (script start tag
steps 2-3), `create-an-element-for-the-token` (steps 13, 15), `the-template-element` (cloning steps), `look-up-a-custom-element-definition`;
`webref body dom concept-node-clone` (clone a node step 3, clone a single node step 8) (2026-11-01). Breadth: K = 3
(DOM, HTML, Selection API), M = 27. M ≥ 20 is the split signal, and this memo is the split: each per-PR plan carries only its own rows.

Step numbers are cited where `webref body` shows them today; per-PR plans and code comments cite steps **by name**
where the number has moved before (extract's partially-contained-child steps, §0.3 item 1).

## §4. Architecture

### §4.1 Layering map

| Concern | Home | Note |
|---|---|---|
| Containment, toString, delete, contents (clone/extract), insert, surround, context resolution | `elidex-dom-api` `range/` | engine-independent |
| Live-range reference | `elidex-dom-api` `range/live.rs` | existing registry, new handle |
| Record-producing primitives, fragment parse into a new fragment | `elidex-script-session` `mutation/` | existing primitives; `html_fragment.rs` gains one function in RC-4 |
| Clone-a-node; the created-set carrier | `elidex-dom-api` `node_methods/clone.rs` | the canonical cloner marks each created clone a pass acts on with `ClonedFrom` and stamps every clone root's node document (RC-2, §4.3) |
| Created-set entry | `elidex-dom-api`: the `CloneNode` handler and the Range contents entry return their root (clone root / fragment); the created set is the marked entities under it | the VM seam only receives the root; it computes nothing (X1, X × C) |
| CE clone-creation pass; form-state pass | `elidex-custom-elements`, `elidex-form` | each walks in its own crate and acts only on marked entities (RC-2, §4.3) |
| Error contract | `elidex-dom-api` engine returns `DomApiErrorKind` + records produced before the error (E3) | the host maps and commits only |
| Brand check, argument conversion, live-range-reference lookup, error mapping, record commit, post-clone seam call, wrapper | `vm/host/range_proto_mutation.rs` | marshalling only (X1); no `with_range_mut`, no transient `Range::new` carrier (RC-1 DoD) |

### §4.2 Live-range reference (RC-1)

`LiveRangeRegistry` already shares its map with the bridge through `Arc<Mutex<HashMap<RangeId, Range>>>`
(`range/live.rs:336-338`). RC-1 adds an owned handle, `LiveRangeRef` (NEW) = (that `Arc`, `RangeId`), obtained from the
registry, with a read (applying the dangling-collapse rule of `finalize_pending` to its own entry) and a boundary write.
The engine methods take `(&LiveRangeRef, &mut EcsDom)`. The VM obtains the handle while borrowing HostData, releases
that borrow, then passes `host.dom()`; so `host_data/mod.rs` (2024 lines) is not touched and no new HostData split
accessor is needed. `delete_from_document` (`selection.rs:513`) takes the same path. `commit_range_after_mutation`
(`range_proto_mutation.rs:34-66`) and the Selection write-back (`selection.rs:537-542`) are deleted.

Not chosen: keep snapshot-then-overwrite and prove the final write equal to the spec's. The proof holds for
delete and extract (every later mutation is inside a partially contained child or at an index at or after newOffset),
but not for insert (D12) or surround (replace-all in step 4 can move the live range before step 5 reads it), so the
two patterns would remain.

Unit tests in `elidex-dom-api` register their range in a registry wired through the existing
`make_range_only_test_dispatcher` (`range/mod.rs:21`).

### §4.3 Contents algorithm and the post-clone seam (RC-2)

One recursive function implements `concept-range-extract` and `concept-range-clone` with a mode (Clone | Extract). The
modes differ in exactly three places, all named in the spec: the same-CharacterData case (extract also replaces data),
the partial CharacterData cases (extract also replaces data), and contained children (clone: deep clone; extract:
move). Extract alone sets the range (step 15). The partition (first / last partially contained child, contained
children) comes from B1. Before the first mutation it checks both boundaries (PI refusal, E1) and step 11 (doctype).

**Output (E3).** `Ok`: the fragment and the records. `Err`: a `DomApiErrorKind` and the records produced before the
error (empty for every error this algorithm raises, since all of them precede the first mutation; surround's late
errors are RC-3's). The host maps the kind and commits.

**Created set = the `ClonedFrom` marker (driver decision).** Both post-clone passes act only on entities that carry
the existing transient `ClonedFrom` marker; a moved original never carries one, so extract's moved originals under a
shallow clone are skipped without a returned `Vec`. No recording variant of the cloner is needed. Feasibility was
checked against the code (§0.2); the decision holds under these conditions, each a requirement on RC-2:

1. **Gate.** Pass 2 (`mark_cloned_form_control`, today `<input>` / `<textarea>` only) marks a clone when either pass
   acts on it: the source is `<input>` / `<textarea>`, **or** the source carries `CustomElementState` (the clone gets
   one from pass 1, and `prepare_upgrade` skips any entity without it, `upgrade.rs:60-64`). The "no spray" property
   of the gate (`node_methods/clone.rs:368-374`) survives: the marker lands only where a pass will act. For
   `cloneNode` the CE pass then visits the same entities as today (every CE-bearing entity under a clone root is a
   clone), so its behaviour is unchanged.
2. **Order and sweep.** The seam runs the CE pass first (it reads the marker and does not remove it), then the form
   pass, which already removes every marker it visits, unconditionally (`elidex-form/src/clone.rs:52-62`) over the
   root's template-inclusive descendants, which covers every marker pass 2 attached. This is today's shim order
   (`node_methods_extras.rs:221`, `:230`), now inside the one seam.
3. **No stale marker in production.** A left-behind marker is harmless to the form pass (generation-safe source),
   but for the CE pass it would mark a pre-existing element as fresh (is-value clear, upgrade enqueue). So every
   production call of the canonical cloner is followed by the seam: today `CloneNode` only, reached from
   `native_node_clone_node` (§0.2); RC-2 adds the Range natives. RC-2's DoD re-runs the census and rewrites the
   "harmless" doc rows (`node_methods/clone.rs:60-64`, `tree_clone.rs:29`) to the new contract. Unit tests that
   clone without the seam do not feed CE-pass tests.
4. **CE-pass scope over template contents.** The CE pass visits marked entities in the root's shadow-inclusive
   descendants and **not** template-content clones (today's walk, `for_each_shadow_inclusive_descendant`,
   `upgrade.rs:131`, does not reach template contents). Spec basis: template cloning steps step 2 clone the
   contents "with document set to copy's template contents's node document" (`webref body html
   the-template-element`), the inert template document "whose browsing context is null" (appropriate template
   contents owner document step 1.1.1); look up a custom element definition step 1 returns null for a null
   registry (`webref body html look-up-a-custom-element-definition`). That the inert document's registry is null
   is **unconfirmed** (webref: `dfn html 'effective global custom element registry'` → no dfn; the clone-a-single-
   node registry steps 2.2-2.3 route through it). The rule keeps `cloneNode` behaviour unchanged; a test pins it
   (a cloned `<template>` whose contents hold a defined custom element enqueues no upgrade for the content clone).
   C1 (`#11-template-contents-owner-document`) changes those contents' node document; the rule does not depend on it.

The seam (NEW, RC-2) is one VM function used by `native_node_clone_node` and the Range natives: it receives the root
the engine returned and calls the two passes. The engine-independent `apply_clone_creation_ce_semantics` keeps its
signature (root in, candidates out) and adds the marker filter; `apply_clone_form_state` is unchanged. Its contract
"MUST only be called on a freshly cloned subtree" (`upgrade.rs:121-123`) becomes "acts only on marked entities".

deleteContents stays its own algorithm (the spec's is separate and clones nothing); it shares B1 and the newNode /
newOffset helper with extract.

### §4.4 ECS-native check

| Spec / OO notion | elidex form |
|---|---|
| live range object updated by mutation observers inside the engine | existing `LiveRangeRegistry` map + `LiveRangeBridge` driven by `MutationDispatcher` events; `LiveRangeRef` is a handle, not a new registry |
| "clone a node" with per-class cloning steps | ECS cloner + post-pass worklist over (src, dst) pairs (existing); the created set is a marker component (`ClonedFrom`) on each acted-on clone, read by the passes (query), not a returned list |
| subrange objects | plain values on the stack (L2) |
| `[CEReactions]` element queues | existing reaction queue fed by the mutation-event leg and C2 |

No component, system or side-store is added. The existing `ClonedFrom` marker's gate widens (§4.3 condition 1).

## §5. Slices

Every slice: its own per-PR plan-memo and `/elidex-plan-review` before implementation; `cargo test -p <crate>
--all-features` per touched crate, then `mise run ci`. Each silent-wrong defect it closes has a **negative control:
a test that is red on the pre-fix head and green after**; the red run is recorded in the PR body. No WPT subset is
claimed; coverage is the engine-independent unit tests (`crates/dom/elidex-dom-api/src/range/tests.rs`, split in RC-0a)
plus JS-level VM tests (`crates/script/elidex-js/src/vm/tests/tests_range.rs`, split in RC-0a) and MutationObserver
tests (`crates/script/elidex-js/src/vm/tests/tests_mutation_observer/range_ops.rs`).

### §5.1 RC-1 — containment primitives, exposed index, live-range reference, toString, deleteContents, insertNode

- **Scope**: `range/containment.rs` (NEW): contained, partially contained, common ancestor (today
  `common_ancestor_container`, `range/mod.rs:121-145`, moves here), the commonAncestor child partition, contained-node
  iteration, newNode / newOffset. Child index (D13): `child_index_in_ancestor` and `child_index` move onto
  `EcsDom::index_in_parent` (B3). `LiveRangeRef` (§4.2). toString rewritten on B1 (D1-D3). deleteContents on B1 +
  newNode / newOffset written at step 8 (D4, D5). insertNode on `LiveRangeRef`: step 1 (its own HierarchyRequestError
  check) and step 6 (`ensure_pre_insertion_validity`) before any mutation (D10, D11, per the §0.4 rule), step 13
  evaluated after step 12 (D12). extractContents' flat moves switch from `contained_top_level_nodes` to B1's
  nodesToRemove set (interim; RC-2 replaces the body), so `contained_top_level_nodes` and `next_in_preorder_global`
  are deleted. `delete_from_document` (`selection.rs:513`) on `LiveRangeRef`. Error contract E3 for insertNode
  (`DomApiErrorKind` gains nothing here; HierarchyRequestError exists).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`, `crates/dom/elidex-dom-api/src/range/live.rs`, `crates/dom/elidex-dom-api/src/range/containment.rs` (NEW),
  `crates/dom/elidex-dom-api/src/selection.rs`, `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto.rs` (toString native only if its signature changes), tests.
- **Tests**: unit: toString with Element start (children before `start_offset` excluded) and Element end
  (descendants before `end_offset` included), CDATASection (Rust-constructed); delete with Element end container
  (`<div><p>ab</p><span>X Y</span></div>`, range `(ab, 1)`-`(span, 1)`: X removed); **extract with the same Element
  end container** (X moved into the fragment; D4's extract half, cross-checked against RC-2's rewritten extract,
  which must keep the test green); delete final boundary when start is not an inclusive ancestor of end; another
  live range at the deletion point; D13: `h.attachShadow({mode:'open'}); h.append(a, b)`, range `(h, 1)`-`(h, 2)`:
  `a` not contained, `comparePoint(h, 1)` / `isPointInRange` on the exposed index, deleteContents removes `b` only.
  VM: the same through JS; `Selection.deleteFromDocument` final boundary; `r.selectNode(x); r.insertNode(x)` end
  boundary; the D10/D11 cases of §0.4. MO: record order unchanged for existing `range_ops.rs` cases; new D4 cases
  emit the removal (delete) and move (extract) records for X.
- **DoD**: negative controls for D1, D2, D3, D4 (delete **and** extract), D5 (delete + Selection), D10, D11, D12,
  D13, each red-first or under its green-branch rule (§0.4), the outcome recorded in the PR body;
  `git grep -n 'contained_top_level_nodes\|next_in_preorder_global\|commit_range_after_mutation' -- crates`
  returns nothing; step 13 is not left in the host: `git grep -n 'with_range_mut\|Range::new' --
  crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` returns nothing (today 4 hits: `:50`, `:75`, `:158`,
  `:193`); `git grep -n 'child_index_in_ancestor' -- crates` returns nothing; E2 timing measured for
  deleteContents, insertNode, `Selection.deleteFromDocument`; RC-1 hygiene rows of §6 done.

### §5.2 RC-2 — one contents algorithm for clone and extract

- **Order**: after RC-1, and its node-document part (C3, D8, P4's reader) after **C0** has landed, or folded into C0 if
  the PM so decides (§9, Q-X1).
- **Scope**: §4.3. cloneContents implemented; extractContents replaced by the same function. Same-container and
  partial CharacterData for Text, Comment and CDATASection via `get_char_data` and canonical clone + set data (D7);
  partially contained element children cloned shallowly with recursive subranges (D6); doctype throw at step 11
  (D9, `HierarchyRequestError`, before any mutation); PI boundary refused up front (E1); fragment and clones with
  node documents (C3, D8): the fragment is created given `EcsDom::node_document(start node)`, the canonical cloner
  stamps every clone root, shallow included (a **cloneNode behaviour change** too); VM post-clone seam shared with
  `cloneNode`, with the `ClonedFrom` created set (§4.3 conditions 1-4). ProcessingInstruction has no producer (§0.2,
  unreachable from script); `clone.rs:126` stays an unslotted stub here and gets a proposed slot (§8).
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW), `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`,
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs`, `crates/dom/elidex-custom-elements/src/upgrade.rs`,
  `crates/core/elidex-ecs/src/dom/tree_clone.rs` (doc row only),
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, `crates/script/elidex-js/src/vm/host/node_methods_extras.rs`, `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs`,
  tests (`crates/dom/elidex-form/src/clone.rs` unchanged unless the per-PR plan finds otherwise).
- **Tests**: unit: extract and clone over `<p>a<b>bc<i>d</i>e</b>f</p>` with start inside `bc` and end inside `f`
  (fragment = `c`-text, shallow `<b>` holding `<i>d</i>` and `e`, then `f`-head); same for clone with the tree
  unchanged; Comment same-container and cross-container boundaries; doctype throw with tree unchanged; node
  document of fragment and clones read through `EcsDom::node_document`; clone of a clonable shadow host and of a
  `<template>` (contents' node document asserted as C0 / C1 leave it at RC-2's base, §9); a shallow `cloneNode()` now
  stamped; RC-1's D4 extract case still green. VM: D8 on a non-singleton document (§0.4); a custom element under a
  partially contained `<b>` keeps its identity and is not re-upgraded after extract (C2); a cloned custom element
  gets its upgrade reaction (C1); no upgrade for a template-content clone (§4.3 condition 4); `<input>` live value
  copied into a cloned contained subtree. MO: extract records in spec order (start replace data, recursive subrange
  removals on the partial child, contained moves, end replace data); clone produces none.
- **DoD**: negative controls for D6, D7, D8, D9, D5 (extract), D8 under its green-branch rule; the cloneContents stub
  test (`tests_range.rs:441-452`) replaced by behaviour tests; `git grep -n 'create_text(\|create_document_fragment()' --
  crates/dom/elidex-dom-api/src/range` returns nothing; one post-clone seam (`git grep -n
  'enqueue_subtree_upgrade_reactions\|apply_clone_form_state' -- crates/script/elidex-js/src/vm/host` shows one
  call site each, inside the seam); the production-cloner census of §4.3 condition 3 re-run (`git grep -n
  'clone_node_with_shadow_honor\|"cloneNode"' -- crates`), every caller followed by the seam; E2 timing measured for
  extractContents and cloneContents; RC-2 hygiene rows of §6 done.

### §5.3 RC-3 — surroundContents

- **Scope**: surroundContents steps 1-7 on `LiveRangeRef`: step 1 (non-Text partially contained → InvalidStateError)
  and step 2 (Document / DocumentType / DocumentFragment → InvalidNodeTypeError; `DomApiErrorKind` gains the variant,
  E3) before any mutation; step 3 = RC-2's extract; step 4 replace all through the session primitive; step 5 =
  RC-1's insert, unchanged; step 6 append; step 7 select (`range/boundary.rs:125`). Late throws (insert step 1 when
  the post-extract start is a Comment; append into a CharacterData `newParent`, the spec's "historical reasons"
  note) leave the mutations done and return their records with the error (E3), which the host commits (M2).
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW in RC-2), `crates/dom/elidex-dom-api/src/range/mutation.rs`,
  `crates/script/elidex-script-session/src/types.rs` (one enum variant), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each error branch with tree / range / records checked; surround with `newParent` already having
  children (replace-all records); surround around a Text-only partial selection; range after surround equals
  "select newParent"; late-throw cases with their records delivered.
- **DoD**: the surroundContents stub removed; E2 timing measured for surroundContents; RC-3 hygiene rows of §6 done.

### §5.4 RC-4 — createContextualFragment

- **Order**: after IBP-sandbox N9 (Layout) provides the Fragment parser mode, or with N9 providing the mode and RC-4
  as its consumer; the PM sets the cross-lane order (§9, Q-X2). RC-4 does **not** re-derive the mode or the
  scripting-disabled state.
- **Scope**: `create_contextual_fragment` (dom-api, `range/mutation.rs:637`) resolves the context per HTML §8.5.7 steps 3-6 (Element;
  Text / Comment → parent element; else null; null or HTML-namespace `html` in an HTML document → a new `body` created
  in the start node's node document), then calls the session's fragment entry (N9's one fragment entry, which derives
  the mode through F10 with `NodeDocumentOf(context)`, `docs/plans/2026-09-ibp-sandbox-input.md:66`) with mode
  Fragment and Legacy (no declarative shadow roots), and places the roots into a new DocumentFragment whose node
  document is `EcsDom::node_document(start node)` (P4). The created `body` is destroyed before return on every path
  (P3). Trusted Types: elidex has none (innerHTML passes the string through, `dom_inner_html.rs:146`), so step 2 is
  identity until it exists; the gap is program-wide, not this program's (§8 proposed slot).
- **Overlap with the fragment plan** (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574; PR 1 landed as #577
  `ee6c7a1a`): RC-4 relies on its I1 (the parse never mutates the context) and I2 (a top-level declarative template is
  plain); with Legacy mode the flag is off, so templates are plain at every depth (P2). The context rules differ from
  innerHTML's (target, or the shadow host) and outerHTML's (parent; DocumentFragment parent falls back to "div",
  `html_fragment.rs:138-142`, `:156-162`): RC-4's context is always an Element (the created `body` covers null), so no fallback is
  used. No change to the fragment plan's code.
- **Questions the per-PR plan must answer first (scout)**:
  (a) **Mode**: consume N9's Fragment-mode producer (HTML §13.2.6.4.4 script start tag step 2: in Fragment mode the
  script gets no parser document; step 3: only Inert sets "already started"). Remaining consumer-side question: does
  elidex run "prepare the script element" (HTML §4.12.1.1) when a fragment-parsed `<script>` is later inserted? If no,
  createContextualFragment's scripts never run where the spec runs them; conditional slot
  `#11-fragment-script-run-on-insert` (proposed, §8).
  (b) Does elidex have XML documents (fragment parsing algorithm steps step 1)? If not, the branch is unreachable and
  stated so.
  (c) **Scripting state**: consume N9's F10 `NodeDocumentOf(context)` derivation (HTML §13.4 HTML fragment parsing
  algorithm step 8: "If contextDocument's scripting is disabled, then set scriptingMode to Disabled"); RC-4 adds no
  `scripting_disabled` argument (today `dom_inner_html.rs:421`, `:476` pass `false`; N9 drops the field).
  (d) Are custom-element upgrade reactions for parsed elements enqueued at element creation (HTML §13.2.6.1 "create
  an element for a token" step 13: willExecuteScript is false for a fragment parse; step 15 → DOM §4.9 "create an
  element" enqueues the upgrade reaction) or only at insertion, in innerHTML today? RC-4 mirrors innerHTML; a
  difference is a parser-path defect, conditional slot `#11-fragment-parse-ce-upgrade-timing` (proposed, §8).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs` (or its successor module), `crates/script/elidex-script-session/src/mutation/html_fragment.rs`
  (+ re-export in `mutation/mod.rs`, 994 lines at `ee6c7a1a`; re-measured at the per-PR plan, §7), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each context branch (Element; Text and Comment with parent element; Text under a DocumentFragment → body;
  Document start → body; `html` element → body; SVG-namespace `html`-named element is not replaced); `<td>` parsed with
  a `<tr>` context vs body context; nested `<template shadowrootmode>` stays plain; fragment `ownerDocument`; context
  unchanged and no leaked `body` (`world().len()` before and after); no MutationRecord; a parsed `<script>` is not
  "already started" (Fragment mode).
- **DoD**: the createContextualFragment stub test (`tests_range.rs:454-463`) replaced; questions (a)-(d) answered in the
  per-PR plan; E2 timing measured for createContextualFragment; RC-4 hygiene rows of §6 done; `#11-range-full-impl`
  closed when RC-2, RC-3 and RC-4 have landed.

## §6. Hygiene, by owning slice

| Site | Fix | Slice |
|---|---|---|
| `range/mutation.rs:598` stale slot `#11-range-clone-and-surround-contents` (closed, merged into `#11-range-full-impl`) | deleted with the stub | RC-2 |
| `range/mutation.rs:595` "`clone_node_deep` already exists on EcsDom" | deleted with the stub | RC-2 |
| `range/mutation.rs:632` stale slot `#11-range-create-contextual-fragment` | one owner, coordinated with IBP-sandbox N9, which plans to re-point this line (`docs/plans/2026-09-ibp-sandbox.md:183`, `:538`): if N9 lands first, RC-4 deletes N9's re-pointed text with the stub; if RC-4 lands first, the stub and the line are gone and N9's re-point is moot. The PM records which (Q-X2) | RC-4 (with N9) |
| `range/mutation.rs:622-623`, `:633` "`is_html_namespace` forward-stub" | deleted; implemented in #256 (§0.2) | RC-4 |
| `range/mutation.rs:252-263`, `:276-277` deferral notes for `#11-range-comment-extract-clone` and the B1.2d-ii §11 carve | deleted with the old body | RC-2 |
| `range/mutation.rs` extract step cites "17.4", "20.4", "step 19", "steps 18/21", "4.1/17.1/20.1" (`:252`, `:259`, `:314`, `:343`, `:363`, `:381`) | rewritten by name (off by one against today's numbering) | RC-2 |
| `range/mutation.rs:422`, `range_proto_mutation.rs:186` "§5.10 splitText" (DOM has no §5.10: `webref heading dom 5.10` → none) | "§4.11 split a Text node" | RC-1 |
| `range_proto_mutation.rs:1-16` module doc ("3 stubs … NotSupportedError pending `#11-range-full-impl`") | rewritten as each stub goes | RC-2 (clone), RC-3 (surround), RC-4 (contextual) |
| `range_proto_mutation.rs:210-212`, `:226-228`, `:247-249` stub docs citing `#11-range-full-impl` | deleted with each stub | RC-2 / RC-3 / RC-4 |
| `range/mutation.rs:142-146` delete doc "partial-element-ancestor deep-clone … B1.2d-ii plan §11 carve" | deleted | RC-1 |
| `range/mod.rs:345`, `:374`; `range/live.rs:235`, `:240`, `:246`, `:661`, `:705`; `range/tests.rs:529` "§4.10" for split a Text node (`webref dfn dom 'split a Text node'` → §4.11 Interface Text; §4.10 = Interface CharacterData) | "§4.11 split a Text node" | RC-1 |
| `range/mod.rs:400`; `range/live.rs:283`, `:760`; `range/tests.rs:597` "§4.5" for normalize (`webref dfn dom normalize` → §4.4 Interface Node; §4.5 = Interface Document) | "§4.4 normalize()" | RC-1 |
| `selection.rs:499` "§3.3" for deleteFromDocument (`webref dfn selection-api deleteFromDocument` → §3 Selection interface), and the `selection.rs` Selection API "§3.1" / "§3.2" / "§3.3" family (`:25`, `:52`, `:55`, `:84`, `:87`, `:120`, `:124`, `:128`, `:162`, `:175`, `:187`, `:202`, `:218`, `:252`, `:286`, `:304`, `:322`, `:372`, `:420`, `:447`, `:450`, `:472`, `:485`, `:493`, `:549`, `:578`, `:604`; unconfirmed: `webref heading selection-api 3.1` → "no headings under §3.1", same for 3.2 / 3.3) | "Selection API §3" + the member name; no sub-number until webref confirms one | RC-1 |
| `selection.rs:690` `#[allow(dead_code)]` on the test helper `elem` ("kept for future cross-doc / no-association tests") | connect it (a no-association test RC-1 needs anyway, e.g. D8-style ownerless nodes) or delete it | RC-1 |
| every § and step-number cite in the files a slice edits | **4-pattern sweep**, each with its hit count in the per-PR plan: (1) truth first: `webref dfn` / `heading` for the concept; (2) with the spec-name prefix (`git grep -n -P 'DOM §4\.10(?![.\d])' -- <files>`); (3) without it (`'§4\.10(?![.\d])'`); (4) by § number for every section the files cite, plus probes for non-existent sections (`'§5\.10(?![.\d])'`; `webref heading dom 5.10` → none). Step numbers: `git grep -n -E 'step [0-9]+(\.[0-9]+)?' -- <file>`, compared with `webref body`. At `ee6c7a1a` over RC-1's files (`range/`, `selection.rs`, `range_proto.rs`, `range_proto_mutation.rs`): (2) `DOM §4.10` 3, `DOM §4.5` 1; (3) `§4.10` 14 = 8 split drifts (rows above) + 6 correct "replace data" cites (`range/mod.rs:289`, `:319`, `mutation.rs:157`, `tests.rs:447`, `:467`, `:768`), `§4.5` 4 (all normalize drifts, rows above); (4) `§5.10` 2 (both in the splitText row above), `§3.x` 32 = 28 Selection API (`selection.rs`, row above) + 4 WebIDL (`selection.rs:376`, `range_proto.rs:682`, `:693`, `:885`) | by name where the number has moved; kept where `webref body` shows it today | each slice, its own files |

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Measured with `wc -l` at `ee6c7a1a`:

| File | Lines | Touched by | Action |
|---|---|---|---|
| `crates/dom/elidex-dom-api/src/range/live.rs` | 1021 | RC-1 (`LiveRangeRef`) | **RC-0a**: inline tests (`:498-1021`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/selection.rs` | 1036 | RC-1 | **RC-0a**: inline tests (`:676-1036`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/range/tests.rs` | 806 | RC-1, RC-2, RC-3 | **RC-0a** (projected past 1000): live-range-update tests (`adjust_ranges_*`, `:332-684`) vs method tests |
| `crates/script/elidex-js/src/vm/tests/tests_range.rs` | 929 | RC-1..RC-4 | **RC-0a** (projected): split on its existing section headers (`:32`, `:69`, `:137`, `:184`, `:437`, `:464`, `:481`, `:539`, `:601`) |
| `crates/script/elidex-script-session/src/mutation/html_fragment.rs` | 858 (tests from `:277`) | RC-4 | **RC-0b** (projected): inline tests to `mutation/tests_html_fragment.rs` (NEW), as `mutation/tests_replace_all.rs` already is |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 2024 | not touched (§4.2) | none; if a per-PR plan finds it must touch it, the split comes first |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | RC-4 (+1-2 re-export); also edited by C0 (D-5's fragment arm, C0 memo O-4 (b)) | **re-measured at each per-PR plan** that touches it: if C0 (or anything else) has taken it to ≥ 1000, a standalone pure-move split comes first (CLAUDE.md touch-time split); at `ee6c7a1a` none |
| `crates/dom/elidex-dom-api/src/range/mutation.rs` | 640 | RC-1..RC-4 | none as prereq; new code goes into `containment.rs` / `contents.rs` (NEW) so it does not grow past ~800 |
| `range/mod.rs` 587, `range/boundary.rs` 299, `range_proto_mutation.rs` 262, `range_proto.rs` 906, `tests_mutation_observer/range_ops.rs` 586, `node_methods/clone.rs` 458, `elidex-custom-elements/src/upgrade.rs` 526, `node_methods_extras.rs` 293, `custom_elements/upgrade.rs` 487, `elidex-form/src/clone.rs` 304, `child_node/mod.rs` 459, `dom_bridge.rs` 696 | under 1000 | various | each per-PR plan re-measures; `range_proto.rs` (906) is touched only if toString's native changes |

RC-0a and RC-0b are pure moves (no text change beyond module declarations), reviewed with `git diff --color-moved`.

## §8. Out of scope, and slots

- **`#11-shadow-tree-live-range-tracking`** (open, unchanged): shadow-tree live-range boundary updates. The program
  keeps the shallow light-tree contract of `range/live.rs:38-55`. (D13 is a different defect: the light-tree index on
  a host, not boundaries inside the shadow tree.)
- **`#11-range-client-rects`** (open, owner Layout): `getBoundingClientRect` / `getClientRects`, a consumer of the
  forced style/layout flush umbrella. Not touched.
- **`#11-insertadjacenthtml-context-validity`** (open): its trigger lists "Range work touching
  `apply_insert_adjacent_html`"; RC-4 does not touch it, so the trigger does not fire. Its defect text also names
  `Range.createContextualFragment` in the parser-path family; that entry is **moot**: createContextualFragment parses
  into a new fragment and inserts nothing, so there is no pre-insert validity step to skip (HTML §8.5.7 steps 1-7).
  The PM can narrow the slot's family to innerHTML / outerHTML / setHTMLUnsafe / insertAdjacentHTML.
- **Selection.containsNode**: uses the Selection spec's own containment (`node_fully_contained`,
  `selection.rs:652-670`), not DOM "contained"; B1 does not replace it.
- **IBP-sandbox N9** (Layout): the Fragment parser mode, the scripting-disabled derivation and F11 at every
  fragment-parse caller are N9's (§0.2); RC-4 consumes them.
- **C0 / C1** (Layout): `EcsDom::node_document` and insertion adoption are C0's; template contents' owner document is
  C1's (`#11-template-contents-owner-document`). RC-2 consumes them (§9).

**Proposed slots outside the program's own fixes** (for the PM to register, fold or decline; owner left to the PM):

| Proposed ID | Defect | Why deferred | Trigger | Re-eval |
|---|---|---|---|---|
| `#11-trusted-types` | elidex has no Trusted Types; every sink that takes `(TrustedHTML or DOMString)` (createContextualFragment step 2, innerHTML, …) treats the string as is | program-wide gap (one mechanism for every sink), not a Range defect; until it exists, createContextualFragment step 2 is identity, like innerHTML | a Trusted Types / CSP `require-trusted-types-for` program | 2026-12-01 |
| `#11-processing-instruction-clone` | `CloneNode` refuses ProcessingInstruction with NotSupportedError (`node_methods/clone.rs:126-131`), an unslotted stub | unreachable from script (no PI producer, §0.2); the Range contents algorithm refuses a PI boundary before any mutation (E1) | a PI producer (XML parsing, `createProcessingInstruction`) | 2026-12-01 |
| `#11-fragment-script-run-on-insert` (conditional, §5.4 (a)) | a fragment-parsed `<script>` (Fragment mode) does not run when inserted, where the spec runs "prepare the script element" | only if RC-4's question (a) finds it; the defect is in dynamic script execution, not in Range | RC-4's per-PR plan answering (a) "no" | 2026-12-01 |
| `#11-fragment-parse-ce-upgrade-timing` (conditional, §5.4 (d)) | parsed custom elements get their upgrade reaction at insertion instead of at creation (HTML §13.2.6.1 steps 13, 15) | only if RC-4's question (d) finds a difference; a parser-path defect shared with innerHTML | RC-4's per-PR plan answering (d) | 2026-12-01 |
| (E2 rule) | a `[CEReactions]` timing difference found in a method no slice of this program touches | only if a slice's E2 measurement finds one | that slice's per-PR plan | 2026-12-01 |

The two conditional slots and the E2 slot are registered only if their trigger finds the defect. The script-run
slot's owner (dynamic script execution) is for the PM to assign.

## §9. Order, suspension points, cross-lane dependencies, definition of done

**Order.** RC-0a → RC-1 → RC-2 → RC-3; RC-0b → RC-4 (RC-4 is independent of RC-1..3). Cross-lane:

- **Suspension points (PM decision 201).** Each RC is a suspension point: after any merged slice, L3 may switch to P1
  when C0 lands ("L3 switches to P1 at a slice boundary once C0 lands", PM 201). No slice is left half-landed; the
  program resumes at the next RC.
- **RC-2 depends on C0.** RC-2's node-document work (C3, D8, P4's reader) reads `EcsDom::node_document` (C0 D-1) and
  must land after C0, or be C0's own if C0's census covers these sites. C0 as written does not cover them: under an
  ownerless parent D-4 leaves the child's stamp alone (`:140-146`), so the ownerless extract fragment
  (`create_document_fragment()`, owner `None`) and the unstamped shallow clones stay unstamped after C0; C0 leaves
  the census of ownerless creations to its scout question Q-S2 (`:447-448`, `:667-668`). So which PR owns the stamp
  is a PM question (Q-X1). RC-1 does not depend on C0. RC-2's non-node-document parts could land before C0 only by
  splitting RC-2, which this memo does not propose (one algorithm, one PR).
- **`<template>` clone tests and C1.** RC-2's template tests assert the contents' node document as RC-2's base gives
  it: before C1, the template's own document (C0 memo D-3); after C1, the inert template document. The per-PR plan
  re-measures at its base; if C1 lands mid-review the expectation follows C1, and the CE-pass rule (§4.3 condition
  4) does not change.
- **RC-4 depends on N9.** Either N9 lands first and RC-4 consumes its Fragment-mode producer and F10
  `NodeDocumentOf(context)`, or N9 provides the mode inside RC-4's window with RC-4 as its consumer. The order is a
  PM question (Q-X2; Layout owns N9). The `range/mutation.rs:632` row has one owner (§6).
- **`mutation/mod.rs`** (994 lines, C0 edits it): re-measured at each per-PR plan (§7).

**Definition of done.**

- RC-0a, RC-1, RC-2, RC-3 merged in that order; RC-0b then RC-4 merged, after N9 per Q-X2.
- Every §0.4 silent-wrong row has its negative control, red before and green after, recorded in its PR body, or its
  green-branch regression pin (§0.4).
- `git grep -n -E 'range-clone-and-surround-contents|range-create-contextual-fragment|range-full-impl|range-comment-extract-clone|range-extract-partial|range-tostring' -- crates`
  returns nothing (today: 8 hits, `range/mutation.rs:261`, `:277`, `:598`, `:632`; `range_proto_mutation.rs:16`,
  `:212`, `:228`, `:249`).
- `git grep -n 'NotSupportedError' -- crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` returns nothing.
- Ledger closure texts (for the PM): `#11-range-tostring-boundary-text` closed by RC-1; `#11-range-full-impl` closed
  with the PR numbers and squash SHAs of RC-2 / RC-3 / RC-4; `#11-range-extract-partial-ancestor-clone` and
  `#11-range-comment-extract-clone` closed by RC-2; the proposed slots of §0.4 closed by their home slices (or, if
  folded, given "Not registered by design" homing rows, §0.4).

**Ledger and roadmap texts for the PM** (the memo edits neither):

- Homes: `#11-range-extract-partial-ancestor-clone` and `#11-range-comment-extract-clone` → RC-2;
  `#11-range-tostring-boundary-text` → RC-1 (was "slice 3"); that row's `range/mutation.rs:16-86` → `:16-68`
  (`project_open-defer-slots.md:679`).
- `#11-range-full-impl` re-eval (due 2026-11-01, venue N9's plan-memo, `docs/plans/2026-09-ibp-sandbox.md:763`):
  **disposition = this umbrella is the re-eval outcome** — the slot is homed in the Range contents program (RC-2 clone,
  RC-3 surround, RC-4 contextual, the last consuming N9's Fragment mode); it closes when all three land. The PM
  records it.
- Roadmap row (`m4-12-platform-gap-roadmap.md:1252`), replacement for its description and timing cells: "Range
  cloneContents + surroundContents (DOM §5.5) + createContextualFragment (HTML §8.5.7) through the Range contents
  program (`docs/plans/2026-11-range-contents-program.md`: RC-2 / RC-3 / RC-4). createContextualFragment parses
  through `elidex-script-session` (no `elidex-dom-api` → `elidex-html-parser` dependency, P1) and consumes IBP-sandbox
  N9's Fragment mode." Timing: "program order (§9)" in place of "PR7 前" (D-26 PR7 landed in #458, `315ba316`). The
  current text "+ elidex-html-parser dep on dom-api" contradicts P1, and its "§4.4" / "DOM Parsing §3.2" cites are
  stale (`webref heading dom 4.4` = Interface Node; createContextualFragment is HTML §8.5.7).

## Open questions — for the PM

**Ratifications** (changes to PM decision 202's cut):

- **R1** Slice renumbering: PM 202's slices 3 / 1 / 2 / 4 become RC-1 / RC-2 / RC-3 / RC-4, plus RC-0a / RC-0b (§0.1),
  with the ledger home texts of §9.
- **R2** The reorder (toString first, with the containment primitives; §0.1).
- **R3** The added mechanism: `LiveRangeRef` (§4.2).
- **R4** D10 / D11 fixed in RC-1, not RC-3 (§0.4).
- **R5** The created-set carrier = the `ClonedFrom` marker with its gate widened to the CE pass's predicate (§4.3
  conditions 1-4; driver decision, recorded here because it changes an existing marker's documented contract).

**Cross-lane order** (Layout owns N9, C0, C1):

- **Q-X1** RC-2's node-document stamp (ownerless extract fragment, shallow clones): C0's (via its Q-S2 census) or
  RC-2's after C0? Either way RC-2 lands after C0 and reads `EcsDom::node_document`.
- **Q-X2** RC-4 vs N9: N9 first and RC-4 consumes it, or N9 provides the mode with RC-4 as consumer; and who owns the
  `range/mutation.rs:632` line (§6).
- **Q-X3** Confirm the suspension rule: L3 may switch to P1 at any RC boundary once C0 lands (PM 201), so RC-2 may be
  the first slice after the switch-back.
- **Q-X4** (observation, not this memo's to fix) `docs/plans/2026-09-ibp-sandbox-input.md:66` cites HTML §13.4 "steps
  5, 10" for the mode derivation; the scripting-disabled step is step 8 (`webref body html
  html-fragment-parsing-algorithm`).

**Proposed slots** (register, fold or decline; Why / trigger / re-eval in §0.4 and §8):

- From §0.4, closed by the program's own slices: `#11-range-contained-enumeration-end-element` (D4),
  `#11-range-delete-extract-new-boundary` (D5), `#11-range-insert-node-step13-timing` (D12),
  `#11-range-compare-points-shadow-index` (D13), `#11-range-insert-node-validity` (D10, D11),
  `#11-range-contents-node-document` (D8), `#11-range-contents-doctype-check` (D9). Folded IDs need "Not registered
  by design" homing rows.
- From §8, outside the program: `#11-trusted-types`, `#11-processing-instruction-clone`; conditional
  `#11-fragment-script-run-on-insert` (owner: dynamic script execution, to assign), `#11-fragment-parse-ce-upgrade-timing`.
- Narrow `#11-insertadjacenthtml-context-validity`: drop its createContextualFragment entry (§8).
- Record the roadmap row text and the `#11-range-full-impl` re-eval disposition (§9).

**For the per-PR plans** (not PM questions): RC-1: whether `RangePointError` folds into `DomApiErrorKind` (E3); D10 /
D11 / D12 / D13 measurements. RC-2: D8 producer choice (DOMParser or `document.cloneNode(true)`). RC-4: §5.4 (a)-(d).
