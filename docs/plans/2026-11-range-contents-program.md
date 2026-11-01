# Range contents program (umbrella plan-memo)

Slots: `#11-range-full-impl` (cloneContents / surroundContents / createContextualFragment), `#11-range-extract-partial-ancestor-clone`,
`#11-range-comment-extract-clone`, `#11-range-tostring-boundary-text`, plus the defects this memo finds (§0.4).
Base: `origin/main` `ee6c7a1a`. Owner lane: L3 DOM/form. Decision: PM decision 202 (program = umbrella; §0.1).

**Rev 1** (2026-11-01) — first draft. Every fact below is re-runnable at `ee6c7a1a`: file:line, or the command that
produced it. Line numbers move; the commands do not.

## §0. Frame, premises, decisions

### §0.1 The decision and how this memo refines the cut

PM decision 202 fixed an umbrella with four slices: (1) clone/extract canonical, (2) surroundContents, (3) toString,
(4) createContextualFragment. The memo keeps all four and the one-algorithm rule for clone/extract. It changes the
order and adds one mechanism, both forced by the code:

| PM 202 slice | Here | Why it moved |
|---|---|---|
| 3 toString | **RC-1** (first), with the shared containment primitives and `deleteContents` | toString's defect and a `deleteContents` / `extractContents` defect have one root: both walk pre-order and `break` at `end_container` (`crates/dom/elidex-dom-api/src/range/mutation.rs:50-52` and `:93-95`). The fix is one containment primitive. If it landed with clone/extract and toString kept its own walk until slice 3, two enumerations of "nodes in the range" would coexist (strangler). So the primitive lands once and every consumer moves in that PR. |
| 1 clone/extract | **RC-2** | Unchanged in substance: one algorithm parameterised by clone vs move. It consumes RC-1. |
| 2 surroundContents | **RC-3** | Unchanged; also home of the `insertNode` fix (§0.4 D10). |
| 4 createContextualFragment | **RC-4** | Unchanged. Independent of RC-1..3 (parser path). |
| — | **PR 0a / PR 0b** | Prereq pure-move splits (§7). |

The added mechanism is the **live-range reference** (§4.2, RC-1): the mutating algorithms read and write the
*registered* live range at the spec's own steps, instead of snapshot-then-commit. The surround algorithm (extract,
then replace-all, then insert, then select, all on the same live range) cannot be written faithfully on a snapshot,
and `insertNode` already shows a silent-wrong caused by the snapshot (§0.4 D12).

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

### §0.4 Defect inventory (each silent-wrong row has a negative control in its slice's DoD)

| ID | Defect (observable) | Kind | Evidence | Home | Slot |
|---|---|---|---|---|---|
| D1 | toString, Element start container: includes text of children before `start_offset` | silent-wrong | §0.2 toString | RC-1 | `#11-range-tostring-boundary-text` |
| D2 | toString, Element end container: drops text of descendants before `end_offset` | silent-wrong | §0.2 toString | RC-1 | same |
| D3 | toString skips CDATASection text | silent-wrong, Rust-only reachable | `mutation.rs:53` | RC-1 | same |
| D4 | deleteContents / extractContents, Element end container: contained descendants before `end_offset` are neither removed nor moved | silent-wrong | `mutation.rs:93-95` | RC-1 | propose widening `#11-range-tostring-boundary-text` to "contained-node enumeration" (same root) |
| D5 | deleteContents / extractContents / `Selection.deleteFromDocument` leave the range at `(start, startOffset)` instead of `(newNode, newOffset)` | silent-wrong | `mutation.rs:235-236`, `:399-400`; `selection.rs:537-542` | RC-1 (delete, Selection), RC-2 (extract, rewritten) | new: proposed `#11-range-delete-extract-new-boundary` |
| D12 | `insertNode` evaluates step 13 "if range is collapsed" before the insert; e.g. `r.selectNode(x); r.insertNode(x)` → spec end stays `(P, i+1)`, elidex predicts `(P, i)` (step 9's remove collapses the live range; step 13 is skipped) | silent-wrong (predicted by reading; measured first in RC-1) | `range_proto_mutation.rs:149`, `:190` | RC-1 | new: proposed `#11-range-insert-node-step13-timing` |
| D6 | extractContents leaves partially contained ancestors in the tree and flattens their contained descendants into the fragment | silent-wrong | §0.2 extract | RC-2 | `#11-range-extract-partial-ancestor-clone` |
| D7 | extractContents with a Comment boundary: same-container is a no-op; cross-container skips the Comment's clone and splice | silent-wrong | `mutation.rs:278`, `:333`, `:373` | RC-2 | `#11-range-comment-extract-clone` |
| D8 | extractContents fragment / Text clones have no node document (`ownerDocument`) | silent-wrong (predicted; measured first in RC-2) | `mutation.rs:266`, `:303` | RC-2 | new: proposed `#11-range-contents-node-document` |
| D9 | extractContents does not throw HierarchyRequestError for a contained doctype (step 11); e.g. `r.selectNodeContents(document); r.extractContents()` moves the doctype | silent-wrong | no doctype check in `mutation.rs:265-402` | RC-2 | new: proposed `#11-range-contents-doctype-check` |
| — | cloneContents unimplemented | visible (throws) | §0.2 stubs | RC-2 | `#11-range-full-impl` (clone part) |
| D10 | `insertNode` with a Comment start: no throw; predicted: `node` becomes a child of the Comment | silent-wrong (predicted; **measured in RC-1**, decided below) | §0.2 append_child | RC-3 | new: proposed `#11-range-insert-node-validity` |
| D11 | `insertNode` step 6 checks cycles + ShadowRoot only, not the full "ensure pre-insert validity" (e.g. a doctype into an element, a Text into a Document) | silent-wrong (predicted; measured with D10) | `mutation.rs:476-494` | RC-3 | same as D10 |
| — | surroundContents unimplemented | visible | §0.2 | RC-3 | `#11-range-full-impl` (surround part) |
| — | createContextualFragment unimplemented | visible | §0.2 | RC-4 | `#11-range-full-impl` (contextual part) |

Slot IDs marked "proposed" are for the PM to register or fold; the memo does not edit the ledger. Each proposed slot's
defect sentence is the table row; trigger = its home slice's per-PR plan-review; re-eval = that slice's landing.

**The `insertNode` measurement (D10/D11), and how it decides.** RC-1's per-PR plan runs it first, at the pre-RC-1
head, as committed VM tests: (a) `r.setStart(comment, 0); r.insertNode(span)` expects HierarchyRequestError and an
unchanged tree; (b) `r.setStart(div, 0); r.insertNode(doctypeClone)` and `r.setStart(document, 0);
r.insertNode(text)` expect HierarchyRequestError. Decision rule:
- **Red** (predicted): the fix is "insert a node into a live range" steps 1 and 6 on `ensure_pre_insertion_validity`.
  It lands in **RC-3**, not a new slot, because surroundContents is that algorithm's second consumer and RC-3 already
  rewrites it; the red tests are committed in RC-3 as its negative controls (RC-1 records the red run in its PR body
  and does not commit failing tests). The proposed ledger slot only tracks it until RC-3 lands.
- **Green** (some layer already rejects): no code change; the tests are committed in RC-1 as regression pins, and RC-3
  still routes step 6 through `ensure_pre_insertion_validity` (one validity check, not a lucky layer).

## §1. Shape and verdict

**Edge-dense** (CLAUDE.md "Edge-dense work"): the work intersects boundary-point containment, MutationRecord
emission, live-range updates during the algorithm's own mutations, the clone-a-node pipeline (custom elements, form
state, template contents, shadow roots), and for RC-4 the parser / session path (§2). So: this umbrella, then one
per-PR plan per row below, each through **`/elidex-plan-review` before implementation** (rule, not judgment). A
per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| PR 0a | Pure-move test splits: `range/live.rs`, `selection.rs`, `range/tests.rs`, VM `tests_range.rs` (§7) | — | — | not required (pure move; `git diff --color-moved`) |
| RC-1 | Containment primitives; live-range reference; toString, deleteContents, Selection.deleteFromDocument, insertNode boundary handling moved onto them; extract's enumeration moved to the canonical set (interim); D10/D11 measurement | PR 0a | `#11-range-tostring-boundary-text` (+ D4 if folded), D5 (delete half), D12 | required |
| RC-2 | One "contents" algorithm for clone and extract; cloneContents; VM post-clone seam shared with cloneNode | RC-1 | `#11-range-extract-partial-ancestor-clone`, `#11-range-comment-extract-clone`, `#11-range-full-impl` clone part, D5 (extract half), D8, D9 | required |
| RC-3 | surroundContents; "insert a node into a live range" steps 1 and 6 on `ensure_pre_insertion_validity` | RC-2 | `#11-range-full-impl` surround part, D10, D11 | required |
| PR 0b | Pure-move split of `html_fragment.rs` inline tests (§7) | — | — | not required |
| RC-4 | createContextualFragment through the session fragment-parse path | PR 0b; independent of RC-1..3 | `#11-range-full-impl` contextual part (the slot closes when RC-2, RC-3, RC-4 have all landed) | required |

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
  exposed child list.
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
  (`clone_node_with_shadow_honor`, or a recording variant of it (NEW, RC-2)), so custom-element identity, template
  contents, clonable shadow roots and the form-control marker follow the same rules as `cloneNode`. A CharacterData
  clone is a shallow canonical clone, then "set data" with the substring.
- **C2 (post-clone passes on created nodes only)**: the registry-dependent custom-element pass and the form-state pass
  visit exactly the entities created by cloning in this call, never the moved originals that extract places under a
  clone. One VM post-clone seam serves `cloneNode` and the Range methods.
- **C3 (node document)**: the fragment is created given the start node's node document; clones keep their source's
  node document.
- **E1 (throw before mutating)**: errors the spec raises before any mutation (extract / clone step 11 doctype; surround
  steps 1-2; insert steps 1 and 6) leave the tree, the records and the range untouched.
- **E2 (`[CEReactions]`)**: all six methods are `[CEReactions]` (`webref idl dom Range`, `webref idl html Range`).
  Reactions enqueued by their mutations (disconnected / connected via the mutation-event leg, upgrades via C2) run with
  the same timing as the existing `deleteContents` native. Each slice measures that timing for its methods, against
  `appendChild`'s.
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
  existing `LiveRangeRegistry` map (Range objects are not entities). Clones use the existing ECS cloner and the existing
  transient `ClonedFrom` marker.

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
| P × M | parsed nodes go into a new fragment: no record (P4) |
| P × L | createContextualFragment mutates nothing that exists, so no live-range update |
| P × C | parsed elements get their custom-element state from the parser's own derivation (as innerHTML), not from C2 |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §5.2 Boundary points | position of a boundary point | same node / ancestor / descendant / tree order | `compare_points` (`range/mod.rs:508`), unchanged, consumed by B1 | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | contained; partially contained; get the common ancestor | start-side / end-side / both | `range/containment.rs` (NEW) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | stringification behavior steps 2-5 | same Text node / Text start / contained Text / Text end | `Range::to_string` (RC-1) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | deleteContents steps 3, 4, 5-8 (newNode / newOffset), 9-11 | same CharacterData / start inclusive ancestor of end / otherwise | `Range::delete_contents` (RC-1) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | extract steps 1-21 | collapsed / same CharacterData / partial CharacterData start-end / partial element start-end / contained / doctype throw | contents algorithm (RC-2) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | clone the contents steps 1-17 | same as extract, without mutation | contents algorithm (RC-2) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | insert a node into a live range steps 1-13 | PI or Comment start / parentless Text / start is node / Text split / fragment / node is referenceNode / step 13 collapsed | `Range::insert_node` (RC-1 boundary handling, RC-3 steps 1 and 6) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | surroundContents steps 1-7 | non-Text partially contained / Document-DocumentType-DocumentFragment newParent / newParent has children / late throw from insert or append | `Range::surround_contents` (RC-3) | ✓ | yes |
| WHATWG DOM §5.5 Interface Range | select a node | parent null throws | `Range::select_node` (`range/boundary.rs:125`), consumed by RC-3 | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | ensure pre-insert validity; pre-insert; insert; append; remove; replace all | per-step rejection | `ensure_pre_insertion_validity` (`child_node/mod.rs:251`); session `apply_*` | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | queue a mutation record | childList / characterData | `commit_notify_records` (unchanged) | ✓ | no |
| WHATWG DOM §4.4 Interface Node | clone a node; clone a single node | shallow / deep; shadow host; template; custom element | `clone_node_with_shadow_honor` (`node_methods/clone.rs:227`) + recording variant (NEW) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | create an element | `body` context for RC-4 | element creation helper (RC-4) | ✓ | no |
| WHATWG DOM §4.10 Interface CharacterData | replace data; substring data | Text / Comment / CDATASection | `apply_replace_data` (unchanged); substring in the contents algorithm | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | split a Text node | insert step 7 | `split_text_at_offset` (unchanged) | ✓ | yes |
| WHATWG HTML §8.5.7 The createContextualFragment() method | steps 1-7 | Element / Text-Comment with parent element / null / `html` in an HTML document | `create_contextual_fragment` (RC-4) | ✓ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | fragment parsing algorithm steps | Legacy mode; runScripts; XML document | session fragment parse (RC-4) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm | context = target; fragment node document; scripting mode | `parse_fragment_progressive` (unchanged) | ✓ | yes |
| WHATWG HTML §13.2.4.5 Other parsing state flags | parser scripting mode | Inert / Fragment / Disabled | `ParseFragmentOptions` (question, §5.4) | ✓ | no |
| WHATWG HTML §4.12.1.1 Processing model | already started | Fragment mode leaves it unset | question (§5.4) | ✗ | no |

Numbers and titles: `webref heading dom 5.2`, `5.5`, `4.2.3`, `4.3.2` (via `coverage-map`), `4.4`, `4.9`, `4.10`,
`4.11`; `webref heading html 8.5.4`, `8.5.7`, `13.4`; `webref coverage-map dom 5.5 dom 5.2 dom 4.2.3 dom 4.3.2 dom 4.4
dom 4.9 dom 4.10 dom 4.11 html 8.5.7 html 8.5.4 html 13.4 html 13.2.4.5 html 4.12.1.1`. Dfns: `webref dfn dom
contained` / `'partially contained'` / `'get the common ancestor'` (§5.5), `'clone a node'` / `'clone a single node'`
(§4.4), `'replace data'` / `'substring data'` (§4.10), `'split a Text node'` (§4.11), `'ensure pre-insert validity'` /
`'replace all'` / `remove` / `append` (§4.2.3), `select` (§5.5), `'create an element'` (§4.9); `webref dfn html
'fragment parsing algorithm steps'` (§8.5.4), `'parser scripting mode'` (§13.2.4.5), `'already started'`
(§4.12.1.1). Prose: `webref body dom concept-range-extract`, `concept-range-clone`, `dom-range-deletecontents`,
`concept-range-insert` (also holds surroundContents), `dom-range-stringifier`, `contained`, `concept-range-select`,
`get-the-common-ancestor`; `webref body html dom-range-createcontextualfragment`, `fragment-parsing-algorithm-steps`,
`html-fragment-parsing-algorithm`, `parser-scripting-mode`, `parsing-main-inhead` (2026-11-01). Breadth: K = 2,
M = 20. M ≥ 20 is the split signal, and this memo is the split: each per-PR plan carries only its own rows.

Step numbers are cited where `webref body` shows them today; per-PR plans and code comments cite steps **by name**
where the number has moved before (extract's partially-contained-child steps, §0.3 item 1).

## §4. Architecture

### §4.1 Layering map

| Concern | Home | Note |
|---|---|---|
| Containment, toString, delete, contents (clone/extract), insert, surround, context resolution | `elidex-dom-api` `range/` | engine-independent |
| Live-range reference | `elidex-dom-api` `range/live.rs` | existing registry, new handle |
| Record-producing primitives, fragment parse into a new fragment | `elidex-script-session` `mutation/` | existing primitives; `html_fragment.rs` gains one function in RC-4 |
| Clone-a-node | `elidex-dom-api` `node_methods/clone.rs` | recording variant in RC-2 |
| CE clone-creation pass; form-state pass | `elidex-custom-elements`, `elidex-form` | input changes from "clone root" to "created entities" in RC-2 (§4.3) |
| Brand check, argument conversion, record commit, post-clone seam, wrapper | `vm/host/range_proto_mutation.rs` | marshalling only (X1) |

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
children) comes from B1. Output: the fragment, the records, and the list of entities created by cloning.

The created list feeds one VM post-clone seam (NEW, RC-2), used by `native_node_clone_node` and the Range natives:
the registry-dependent CE pass (`apply_clone_creation_ce_semantics`, input changed from a clone root to the created
list; `cloneNode` passes its own created list) and the form-state pass. Today's per-root walk would visit the moved
originals that extract places under a shallow clone (C2).

deleteContents stays its own algorithm (the spec's is separate and clones nothing); it shares B1 and the newNode /
newOffset helper with extract.

### §4.4 ECS-native check

| Spec / OO notion | elidex form |
|---|---|
| live range object updated by mutation observers inside the engine | existing `LiveRangeRegistry` map + `LiveRangeBridge` driven by `MutationDispatcher` events; `LiveRangeRef` is a handle, not a new registry |
| "clone a node" with per-class cloning steps | ECS cloner + post-pass worklist over (src, dst) pairs (existing); registry-dependent pass over the created set |
| subrange objects | plain values on the stack (L2) |
| `[CEReactions]` element queues | existing reaction queue fed by the mutation-event leg and C2 |

No component, system, query or side-store is added.

## §5. Slices

Every slice: its own per-PR plan-memo and `/elidex-plan-review` before implementation; `cargo test -p <crate>
--all-features` per touched crate, then `mise run ci`. Each silent-wrong defect it closes has a **negative control:
a test that is red on the pre-fix head and green after**; the red run is recorded in the PR body. No WPT subset is
claimed; coverage is the engine-independent unit tests (`crates/dom/elidex-dom-api/src/range/tests.rs`, split in PR 0a)
plus JS-level VM tests (`crates/script/elidex-js/src/vm/tests/tests_range.rs`, split in PR 0a) and MutationObserver
tests (`crates/script/elidex-js/src/vm/tests/tests_mutation_observer/range_ops.rs`).

### §5.1 RC-1 — containment primitives, live-range reference, toString, deleteContents

- **Scope**: `range/containment.rs` (NEW): contained, partially contained, common ancestor (today
  `common_ancestor_container`, `range/mod.rs:121-145`, moves here), the commonAncestor child partition, contained-node
  iteration, newNode / newOffset. `LiveRangeRef` (§4.2). toString rewritten on B1 (D1-D3). deleteContents on B1 +
  newNode / newOffset written at step 8 (D4, D5). insertNode on `LiveRangeRef` with step 13 evaluated after step 12
  (D12); its steps 1 and 6 are untouched (RC-3). extractContents' flat moves switch from `contained_top_level_nodes` to
  B1's nodesToRemove set (interim; RC-2 replaces the body), so `contained_top_level_nodes` and
  `next_in_preorder_global` are deleted. `delete_from_document` (`selection.rs:513`) on `LiveRangeRef`. The D10/D11 measurement
  (§0.4).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`, `crates/dom/elidex-dom-api/src/range/live.rs`, `crates/dom/elidex-dom-api/src/range/containment.rs` (NEW),
  `crates/dom/elidex-dom-api/src/selection.rs`, `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto.rs` (toString native only if its signature changes), tests.
- **Tests**: unit: toString with Element start (children before `start_offset` excluded) and Element end
  (descendants before `end_offset` included), CDATASection (Rust-constructed); delete with Element end container
  (`<div><p>ab</p><span>X Y</span></div>`, range `(ab, 1)`-`(span, 1)`: X removed); delete final boundary when start is
  not an inclusive ancestor of end; another live range at the deletion point. VM: same cases through JS;
  `Selection.deleteFromDocument` final boundary; `r.selectNode(x); r.insertNode(x)` end boundary. MO: record order
  unchanged for existing `range_ops.rs` cases; new D4 case emits the removal record for X.
- **DoD**: negative controls for D1, D2, D3, D4, D5 (delete + Selection), D12; D10/D11 measured and the result recorded
  in the PR body with the decision (§0.4); `git grep -n 'contained_top_level_nodes\|next_in_preorder_global\|commit_range_after_mutation' -- crates`
  returns nothing; RC-1 hygiene rows of §6 done.

### §5.2 RC-2 — one contents algorithm for clone and extract

- **Scope**: §4.3. cloneContents implemented; extractContents replaced by the same function. Same-container and
  partial CharacterData for Text, Comment and CDATASection via `get_char_data` and canonical clone + set data (D7);
  partially contained element children cloned shallowly with recursive subranges (D6); doctype throw at step 11
  (D9, `HierarchyRequestError`, before any mutation); fragment and clones with node documents (D8, measured first:
  `r.extractContents().ownerDocument === document`); VM post-clone seam shared with `cloneNode`;
  `apply_clone_creation_ce_semantics` takes the created list. ProcessingInstruction has no producer (§0.2); the
  clone path rejects it the way `CloneNode` does (`node_methods/clone.rs:126`) rather than producing a payload-less
  clone.
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW), `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`,
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs`, `crates/dom/elidex-custom-elements/src/upgrade.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, `crates/script/elidex-js/src/vm/host/node_methods_extras.rs`, `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs`,
  possibly `crates/dom/elidex-form/src/clone.rs` (if its input changes), tests.
- **Tests**: unit: extract and clone over `<p>a<b>bc<i>d</i>e</b>f</p>` with start inside `bc` and end inside `f`
  (fragment = `c`-text, shallow `<b>` holding `<i>d</i>` and `e`, then `f`-head); same for clone with the tree
  unchanged; Comment same-container and cross-container boundaries; doctype throw with tree unchanged; node
  document of fragment and clones; clone of a clonable shadow host and of a `<template>`; extract's final boundary.
  VM: the same through JS; a custom element under a partially contained `<b>` keeps its identity and is not
  re-upgraded after extract (C2); a cloned custom element gets its upgrade reaction (C1); `<input>` live value copied
  into a cloned contained subtree. MO: extract records in spec order (start replace data, recursive subrange
  removals on the partial child, contained moves, end replace data); clone produces none.
- **DoD**: negative controls for D6, D7, D8, D9, D5 (extract); the cloneContents stub test
  (`tests_range.rs:441-452`) replaced by behaviour tests; `git grep -n 'create_text(\|create_document_fragment()' --
  crates/dom/elidex-dom-api/src/range` returns nothing; one post-clone seam (`git grep -n
  'enqueue_subtree_upgrade_reactions\|apply_clone_form_state' -- crates/script/elidex-js/src/vm/host` shows one
  call site each, inside the seam); RC-2 hygiene rows of §6 done.

### §5.3 RC-3 — surroundContents and the insert fix

- **Scope**: surroundContents steps 1-7 on `LiveRangeRef`: step 1 (non-Text partially contained → InvalidStateError)
  and step 2 (Document / DocumentType / DocumentFragment → InvalidNodeTypeError) before any mutation; step 3 = RC-2's
  extract; step 4 replace all through the session primitive; step 5 = insert; step 6 append; step 7 select
  (`range/boundary.rs:125`). Insert steps 1 and 6 through `ensure_pre_insertion_validity` (D10, D11). Late throws
  (insert step 1 when the post-extract start is a Comment; append into a CharacterData `newParent`, the spec's
  "historical reasons" note) leave the mutations done and commit their records (M2).
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW in RC-2), `crates/dom/elidex-dom-api/src/range/mutation.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each error branch with tree / range / records checked; surround with `newParent` already having
  children (replace-all records); surround around a Text-only partial selection; range after surround equals
  "select newParent"; late-throw cases with their records delivered; D10/D11 cases from §0.4.
- **DoD**: negative controls for D10 and D11 (if RC-1 measured them red); the surroundContents stub removed;
  RC-3 hygiene rows of §6 done.

### §5.4 RC-4 — createContextualFragment

- **Scope**: `create_contextual_fragment` (dom-api, `range/mutation.rs:637`) resolves the context per HTML §8.5.7 steps 3-6 (Element;
  Text / Comment → parent element; else null; null or HTML-namespace `html` in an HTML document → a new `body` created
  in the start node's node document), then calls a new session function in `html_fragment.rs` that parses with
  `parse_fragment_progressive` and places the roots into a new DocumentFragment with that node document (P4). The
  created `body` is destroyed before return on every path (P3). Trusted Types: elidex has none (innerHTML passes the
  string through, `dom_inner_html.rs:146`), so step 2 is identity, the same as innerHTML.
- **Overlap with the fragment plan** (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574; PR 1 landed as #577
  `ee6c7a1a`): RC-4 relies on its I1 (the parse never mutates the context) and I2 (a top-level declarative template is
  plain); with Legacy mode the flag is off, so templates are plain at every depth (P2). The context rules differ from
  innerHTML's (target, or the shadow host) and outerHTML's (parent; DocumentFragment parent falls back to "div",
  `html_fragment.rs:138-142`, `:156-162`): RC-4's context is always an Element (the created `body` covers null), so no fallback is
  used. No change to the fragment plan's code.
- **Questions the per-PR plan must answer first (scout)**: (a) does elidex run "prepare the script element" when a
  parsed `<script>` is later inserted (HTML §4.12.1.1)? If yes, the parser needs the Fragment scripting mode (leave
  "already started" unset) and `ParseFragmentOptions` gains it; if no, createContextualFragment's scripts never run
  where the spec runs them on insertion, and a slot is proposed for that defect, owned by whoever owns dynamic script
  execution. (b) Does elidex have XML documents (fragment parsing algorithm steps step 1)? If not, the branch is
  unreachable and stated so. (c) Is the context document's scripting state available (HTML fragment parsing algorithm
  step 8, Disabled), and how do innerHTML and DOMParser get it today (`dom_inner_html.rs:421`, `:476` pass `false`)?
  (d) Are custom-element upgrade reactions for parsed elements enqueued at parse time (spec) or only at insertion, in
  innerHTML today? RC-4 mirrors innerHTML; a difference found here is a parser-path slot, not RC-4 scope.
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs` (or its successor module), `crates/script/elidex-script-session/src/mutation/html_fragment.rs`
  (+ re-export in `mutation/mod.rs`, 994 lines, +1-2), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each context branch (Element; Text and Comment with parent element; Text under a DocumentFragment → body;
  Document start → body; `html` element → body; SVG-namespace `html`-named element is not replaced); `<td>` parsed with
  a `<tr>` context vs body context; nested `<template shadowrootmode>` stays plain; fragment `ownerDocument`; context
  unchanged and no leaked `body` (`world().len()` before and after); no MutationRecord.
- **DoD**: the createContextualFragment stub test (`tests_range.rs:454-463`) replaced; questions (a)-(d) answered in the
  per-PR plan; RC-4 hygiene rows of §6 done; `#11-range-full-impl` closed when RC-2, RC-3 and RC-4 have landed.

## §6. Hygiene, by owning slice

| Site | Fix | Slice |
|---|---|---|
| `range/mutation.rs:598` stale slot `#11-range-clone-and-surround-contents` (closed, merged into `#11-range-full-impl`) | deleted with the stub | RC-2 |
| `range/mutation.rs:595` "`clone_node_deep` already exists on EcsDom" | deleted with the stub | RC-2 |
| `range/mutation.rs:632` stale slot `#11-range-create-contextual-fragment` | deleted with the stub | RC-4 |
| `range/mutation.rs:622-623`, `:633` "`is_html_namespace` forward-stub" | deleted; implemented in #256 (§0.2) | RC-4 |
| `range/mutation.rs:252-263`, `:276-277` deferral notes for `#11-range-comment-extract-clone` and the B1.2d-ii §11 carve | deleted with the old body | RC-2 |
| `range/mutation.rs` extract step cites "17.4", "20.4", "step 19", "steps 18/21", "4.1/17.1/20.1" (`:252`, `:259`, `:314`, `:343`, `:363`, `:381`) | rewritten by name (off by one against today's numbering) | RC-2 |
| `range/mutation.rs:422`, `range_proto_mutation.rs:186` "§5.10 splitText" (DOM has no §5.10: `webref heading dom 5.10` → none) | "§4.11 split a Text node" | RC-1 |
| `range_proto_mutation.rs:1-16` module doc ("3 stubs … NotSupportedError pending `#11-range-full-impl`") | rewritten as each stub goes | RC-2 (clone), RC-3 (surround), RC-4 (contextual) |
| `range_proto_mutation.rs:210-212`, `:226-228`, `:247-249` stub docs citing `#11-range-full-impl` | deleted with each stub | RC-2 / RC-3 / RC-4 |
| `range/mutation.rs:142-146` delete doc "partial-element-ancestor deep-clone … B1.2d-ii plan §11 carve" | deleted | RC-1 |
| every step-number cite in the files a slice edits (`git grep -n -E 'step [0-9]+(\.[0-9]+)?' -- <file>`, then compare with `webref body`) | by name where the number has moved; kept where `webref body` shows it today | each slice, its own files |

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Measured with `wc -l` at `ee6c7a1a`:

| File | Lines | Touched by | Action |
|---|---|---|---|
| `crates/dom/elidex-dom-api/src/range/live.rs` | 1021 | RC-1 (`LiveRangeRef`) | **PR 0a**: inline tests (`:498-1021`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/selection.rs` | 1036 | RC-1 | **PR 0a**: inline tests (`:676-1036`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/range/tests.rs` | 806 | RC-1, RC-2, RC-3 | **PR 0a** (projected past 1000): live-range-update tests (`adjust_ranges_*`, `:332-684`) vs method tests |
| `crates/script/elidex-js/src/vm/tests/tests_range.rs` | 929 | RC-1..RC-4 | **PR 0a** (projected): split on its existing section headers (`:32`, `:69`, `:137`, `:184`, `:437`, `:464`, `:481`, `:539`, `:601`) |
| `crates/script/elidex-script-session/src/mutation/html_fragment.rs` | 858 (tests from `:277`) | RC-4 | **PR 0b** (projected): inline tests to `mutation/tests_html_fragment.rs` (NEW), as `mutation/tests_replace_all.rs` already is |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 2024 | not touched (§4.2) | none; if a per-PR plan finds it must touch it, the split comes first |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | RC-4 (+1-2 re-export) | none (stays under 1000) |
| `crates/dom/elidex-dom-api/src/range/mutation.rs` | 640 | RC-1..RC-4 | none as prereq; new code goes into `containment.rs` / `contents.rs` (NEW) so it does not grow past ~800 |
| `range/mod.rs` 587, `range/boundary.rs` 299, `range_proto_mutation.rs` 262, `range_proto.rs` 906, `tests_mutation_observer/range_ops.rs` 586, `node_methods/clone.rs` 458, `elidex-custom-elements/src/upgrade.rs` 526, `node_methods_extras.rs` 293, `custom_elements/upgrade.rs` 487, `elidex-form/src/clone.rs` 304, `child_node/mod.rs` 459, `dom_bridge.rs` 696 | under 1000 | various | each per-PR plan re-measures; `range_proto.rs` (906) is touched only if toString's native changes |

PR 0a and PR 0b are pure moves (no text change beyond module declarations), reviewed with `git diff --color-moved`.

## §8. Out of scope, and slots

- **`#11-shadow-tree-live-range-tracking`** (open, unchanged): shadow-tree live-range boundary updates. The program
  keeps the shallow light-tree contract of `range/live.rs:38-55`.
- **`#11-range-client-rects`** (open, owner Layout): `getBoundingClientRect` / `getClientRects`, a consumer of the
  forced style/layout flush umbrella. Not touched.
- **`#11-insertadjacenthtml-context-validity`** (open): its trigger lists "Range work touching
  `apply_insert_adjacent_html`". RC-4 does not touch it (createContextualFragment inserts nothing), so the trigger does
  not fire.
- **Selection.containsNode**: uses the Selection spec's own containment (`node_fully_contained`,
  `selection.rs:652-670`), not DOM "contained"; B1 does not replace it.
- ProcessingInstruction and CDATASection boundaries: no producer (§0.2). Handled by the generic CharacterData path
  where it costs nothing; PI clones are rejected as `CloneNode` rejects them. No slot (nothing reachable is wrong).
- Trusted Types: none in elidex; same as innerHTML. No slot proposed here.
- Proposed new slots: the "proposed" rows of §0.4, and RC-4's conditional slot (§5.4 (a)).

## §9. Definition of done (program) and slot closure

- PR 0a, RC-1, RC-2, RC-3 merged in that order; PR 0b then RC-4 merged (RC-4 may land before RC-1..3).
- Every §0.4 silent-wrong row has its negative control, red before and green after, recorded in its PR body.
- `git grep -n -E 'range-clone-and-surround-contents|range-create-contextual-fragment|range-full-impl|range-comment-extract-clone|range-extract-partial|range-tostring' -- crates`
  returns nothing (today: 8 hits, `range/mutation.rs:261`, `:277`, `:598`, `:632`; `range_proto_mutation.rs:16`,
  `:212`, `:228`, `:249`).
- `git grep -n 'NotSupportedError' -- crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` returns nothing.
- Ledger closure texts (for the PM): `#11-range-tostring-boundary-text` and `#11-range-full-impl` closed with the
  PR numbers and squash SHAs of RC-1 and RC-2/RC-3/RC-4; `#11-range-extract-partial-ancestor-clone` and
  `#11-range-comment-extract-clone` closed by RC-2; the proposed slots of §0.4 closed by their home slices (or never
  registered, if the PM folds them into this program).
- The ledger home of `#11-range-tostring-boundary-text` moves from "slice 3" to RC-1 (§0.1).

## Open questions

- PM: register or fold the §0.4 "proposed" slots; widen `#11-range-tostring-boundary-text` to cover D4.
- RC-1's per-PR plan: the D10/D11 measurement and its outcome (§0.4 decision rule); the `[CEReactions]` timing of the
  existing Range natives (E2).
- RC-2's per-PR plan: D8 measurement; the exact signature change of `apply_clone_creation_ce_semantics` and whether
  `apply_clone_form_state` also takes the created list (C2 requires only that both visit the created set).
- RC-4's per-PR plan: §5.4 questions (a)-(d).
- Unverified, for the RC-1 per-PR plan (scout): whether `child_index_in_ancestor` (`range/mod.rs:549-571`, raw
  sibling walk) counts a shadow host's internal ShadowRoot entity while `node_length` (`children_iter`, which skips
  it) does not, which would break B3 on shadow hosts.
