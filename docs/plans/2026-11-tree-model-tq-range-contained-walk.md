# Tree model tq — Range's contained-node walk takes tree children (per-PR plan-memo)

**Revision log.** R1 fixes F1–F12 (plan-review R1, 0/5/9/8). R1.2: Step 4.5 re-check fixes G1–G8, G11 (0/2/10/0);
P-5 → precondition on tsel (PM 884/885).

Slice **`tq`** of the umbrella
[2026-11-tree-model-shadow-root-is-a-root-umbrella.md](2026-11-tree-model-shadow-root-is-a-root-umbrella.md)
(ratified at `4918a6c7`, PR #648; slice placed by D3, approved PM 820; order tp → tq → ta, and with the
precondition of §11 P-5 (PM 884 / 885) tp → tsel → tq → ta). Census:
[2026-11-tree-model-census.md](2026-11-tree-model-census.md) (C-3, C-14). Base: `origin/main` `3cbd5b88`
(2026-11-21); every code fact below was measured there, and the probes of §6 ran there. Owner lane: L3 DOM/form.
Security-relevant: closes the closed-shadow-root exposure of umbrella §0.1 item 6 on its second path (Range).

Stated in the umbrella and only referred to here, by section: the rule and why `tq` is its own slice (§1.6, the
second path), the pair matrix (§2), the program order (§5.1), the probe gate (§5.2), `tq`'s acceptance (§5.3), the
slot and its two-step closing (§7.1), the landing items (§7.4). This memo adds the caller census, the accessor
check, the base probes, the cells and the mutants.

Anchor check: `git diff --stat 4918a6c7 3cbd5b88 -- docs/plans/2026-11-tree-model-shadow-root-is-a-root-umbrella.md
docs/plans/2026-11-tree-model-census.md crates/dom/elidex-dom-api/src/range crates/dom/elidex-dom-api/src/selection.rs
crates/core/elidex-ecs` → empty, so the umbrella's anchors into these files hold unchanged at this base (`range/mod.rs`
`:573-584` = `next_in_preorder_global`, the raw steps at `:574`, `:579`, the upward step at `:582`; verified
2026-11-21).

## §0. Scope

### §0.1 In scope

One function, `next_in_preorder_global` (`crates/dom/elidex-dom-api/src/range/mod.rs:573`), takes the two existing
exposed child steps in place of the raw ones:

| Line | Base | `tq` |
|---|---|---|
| `range/mod.rs:574` | `dom.get_first_child(current)` | `dom.children_iter(current).next()` |
| `range/mod.rs:579` | `dom.get_next_sibling(node)` | `dom.next_exposed_sibling(node)` |
| `range/mod.rs:582` | `dom.get_parent(node)?` | unchanged (`ta`'s: pending, `Flip::Tr` (NEW: `ta`), census C-2) |

and its doc comment (`range/mod.rs:572`, "Global pre-order next (not confined to a root).") is rewritten to say
what the walk now is: its downward and sideways steps stay in the node tree (WHATWG DOM §5.6 *contained*: "node's
root is range's root", so a shadow tree's node is never a candidate), and its upward step still climbs by the stored
parent, so a walk that starts inside a shadow tree still leaves it (unchanged; `ta` names it, `tr` flips it; the new
comment names the slot that tracks that step, `#11-shadow-root-is-a-tree-root`, not the slice names only). A second
doc comment follows the body: `contained_top_level_nodes` (`range/mutation.rs:70-88`, the function at `:89`) says
*contained* is the two position clauses ("A node is **contained** … iff `(node, 0)` is after the range start AND `(node, node's length)` is
before the range end") and that the walk is "only an over-approximation"; from `tq` the walk enforces the root clause
for every candidate it reaches by descent, so the comment is rewritten as a whole (comment-only edit in
`mutation.rs`, the doc comment `:70-88`) to state all three clauses of §5.6 *contained* and which part enforces
which (root clause: the walk's two child steps; position clauses: `compare_points`), citing WHATWG DOM §5.6 for
*contained* (`#contained`); the paragraph it keeps (`nodesToRemove`, cited "§5.5" at `:72`) moves to §5.6 with it,
so the rewritten comment carries no "§5.5". This is so that a later edit cannot restore a raw step under a comment
that still reads true. Plus the cells of §7. Nothing else: no new symbol in production code, no guard (umbrella
§1.6: "`tq` adds no guard and nothing in it is deleted later").

### §0.2 Out of scope (and whose it is)

- The upward step `:582` → `ta` (re-typed), `tr` (flipped). Umbrella §5 `tq`, §5.1.
- The other raw reads of `range/mod.rs`, in `child_index_in_ancestor` (`:549-571`: `get_parent`, `get_first_child`,
  `get_next_sibling`) → D13 (a), `tc` (the child side) and `tr` (the climb); slot `#11-range-compare-points-shadow-index`
  (umbrella §7.1, closed by `tc`). `compare_points` → `tr` (umbrella §2 UP-T × RANGE).
- The renaming of the two exposed steps into the program's one child set → `tc` (umbrella §1.6: "`tc` renames these
  two calls with the rest of the child side").
- The child-of question of `removeChild` / `insertBefore` / `replaceChild` (the first path of §0.1 item 6) → `ta`
  (umbrella §1.6, the first path).
- TreeWalker / NodeIterator and the 29 `traversal.rs` lines → `tc` → `tr` (umbrella §2 CHILD × RANGE).
- The `Range::to_string` overshoot (`crates/dom/elidex-dom-api/src/range/mutation.rs:47-58`: an end container that is
  an ancestor of the start container is never met, so the walk runs to the end of the document) → a facet of
  `#11-range-full-impl` (umbrella §1.6, §7.4 (N7), applied under PM 826). Not fixed here.
- `deleteContents` / `extractContents` / `deleteFromDocument` leaving the range at `(start, startOffset)` instead of
  `(newNode, newOffset)` → D5 of the Range contents program (`docs/plans/2026-11-range-contents-program.md:729`,
  RC-1). `extract`'s partial-clone steps 17 / 20 → D6 of the same program
  (`docs/plans/2026-11-range-contents-program.md:735`, RC-2b; D6 names the slot
  `#11-range-extract-partial-ancestor-clone`, open in the ledger). The code comment at `mutation.rs:252-255` still
  points at "B1.2d-ii plan §11"; that stale pointer is not `tq`'s.
- `cloneContents` / `surroundContents` are stubs (`mutation.rs:612`, `:621`) and read no walk; not reached. Their
  slot is `#11-range-full-impl` (the code comment's `#11-range-clone-and-surround-contents` is closed: the ledger
  merged it into `#11-range-full-impl` in the 2026-05-15 sweep; home RC-2b clone / RC-3 surround).

### §0.3 The declared behaviour change

One (umbrella §1.6, §3.1 `tq`, PM 820): **Range's contained-node walk no longer enters a shadow tree.**
Script-visible consequences, each probed at base (§6) and pinned (§7):

1. `deleteContents` over a host (closed shadow root) neither unlinks the shadow root nor reports it in a
   `MutationRecord`. The same holds through `Selection.deleteFromDocument`, which runs the same `delete_contents`
   (§5.1; not named by the umbrella, see §11 P-1).
2. `Range.prototype.toString` and `Selection.prototype.toString` no longer include a shadow tree's text.
3. `extractContents` over a host: unchanged (predicted by umbrella §1.6, confirmed by probe P2): at base the shadow
   root reaches `apply_append_child`, whose `rejects_shadow_root_insertion` refuses it
   (`crates/script/elidex-script-session/src/mutation/child_list.rs:189`, `:202`); from `tq` it never gets there.

## §1. Shape and verdict

One PR: a two-line edit and a doc comment in one function of `elidex-dom-api` (`range/mod.rs`), a comment-only
rewrite of the `contained_top_level_nodes` doc comment in `range/mutation.rs` (651 lines), unit cells in
`elidex-dom-api` `range/tests.rs`, VM end-to-end cells in three `elidex-js` test files. No VM `host/` code, no ECS
state, no new type.
It meets the edge-dense trigger on its own (TREE × WR, CHILD × RANGE, LINK, MO; ≥3 axes), so the per-PR
`/elidex-plan-review` is required before its code (umbrella §5 `tq`, *Per-PR plan-review: required*; CLAUDE.md,
*Edge-dense work*), and it is then one PR, not split further. It is not the "separate guard PR before `ta`" that PM 800
rejected (umbrella §1.6, §4).

## §2. Coupled invariants of this slice

The matrix is umbrella §2; `tq`'s rows are TREE × WR (the child half) and CHILD × RANGE, and it touches the elements
LINK and MO of the same table. One line each, for this slice only:

| Pair | Intersection in `tq` |
|---|---|
| TREE × WR (the child half) | the walk is the second script path of umbrella §0.1 item 6: it hands a shadow root to a tree writer without a child-of question (`delete_contents` → `apply_remove_child(h, sr)`, `mutation.rs:216-218`; `extract_contents` → `apply_append_child(frag, sr)`, `:370-377`); from `tq` no shadow root is a candidate, so neither writer is reached with one. The writers themselves are unchanged (their child-of check is `ta`'s / `ts`'s). |
| CHILD × RANGE | the contained-node walk's two child steps become the exposed ones, so its candidate set is the node tree's; the containment filter (`compare_points`) and `child_index_in_ancestor` keep their raw reads (`tc` / `tr`), and no candidate reaches them from a shadow tree any more unless the range itself starts inside one. |
| TREE × LINK | base `deleteContents` takes the shadow root out of the host's stored child list while `ShadowRoot.host` and `ShadowHost.shadow_root` stay (probe P1: the two homes of the backward fact diverge); from `tq` the walk never writes either home, so the link pair and the stored tree stay as they were. |
| CHILD × MO | base delivers a childList record on the host whose `removedNodes` holds a closed shadow root (probe P1, P5); from `tq`, for a range in the host's tree, no record names a shadow root or a shadow-tree node, and the records that remain are those of the contained light nodes, in tree order, unchanged in shape (cells U2, V1). |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §5.6 Interface Range | *contained*: "node's root is range's root, and (node, 0) is after range's start, and (node, node's length) is before range's end" | a node of a shadow tree under a host inside the range: its root is the shadow root, not the range's root → not contained | `next_in_preorder_global` (`range/mod.rs:574`, `:579`) feeds `contained_top_level_nodes` (`mutation.rs:89`); the two position clauses stay `compare_points` (`mutation.rs:104-111`, unchanged) | ✗ (the root clause is enforced for descent only; a range whose start is inside a shadow tree still climbs out by the upward step, `tr`) | yes |
| WHATWG DOM §5.6 Interface Range | `deleteContents()` steps 4 and 10 | step 4 *nodesToRemove* = contained nodes omitting those whose parent is contained; step 10 *remove* each | `delete_contents` (`mutation.rs:216-219`) | ✗ (steps 6-8 *newNode* / *newOffset*: D5, RC-1) | yes |
| WHATWG DOM §5.6 Interface Range | *extract* steps 10 and 18 | step 10 *containedChildren* = children of *commonAncestor* contained in range; step 18 append each to *fragment* | `extract_contents` (`mutation.rs:370-377`, which enumerates through `contained_top_level_nodes`) | ✗ (steps 17 / 20 partial clone: D6, RC-2b; steps 13-15: D5) | yes |
| WHATWG DOM §5.6 Interface Range | stringification behavior steps 3-5 | step 4: "the data of all Text nodes that are contained in this, in tree order" | `Range::to_string` (`mutation.rs:47-58`) | ✗ (the end-ancestor overshoot: `#11-range-full-impl`) | yes |
| WHATWG DOM §4.2.2 Shadow tree | "A shadow tree is a node tree whose root is a shadow root." | why a shadow-tree node's root is never the range's root when the range is in the host's tree | none (the reason for the two steps) | ✓ | yes |
| WHATWG DOM §4.2.3 Mutation algorithms | *remove* step 16 (queue a tree mutation record for parent with « node ») | reached only for nodes in *nodesToRemove* | `apply_remove_child` (`child_list.rs:356`), unchanged; no longer called with a shadow root from Range | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | `shadowRoot` getter step 2 | "If shadow is null or its mode is \"closed\", then return null." | none (what probe P1's record breaks: it hands page script the closed root) | ✓ | yes |
| WHATWG DOM §1.1 Trees | *root*: "The root of an object is itself, if its parent is null, or else it is the root of its parent." | the parent chain of a shadow-tree node ends at the shadow root (a shadow root has a null parent in the node tree), so its root is the shadow root | none (what *contained*'s root clause reads; elidex's `find_tree_root` stops at the `ShadowRoot` marker, `crates/core/elidex-ecs/src/dom/tree/walkers.rs:408-426`) | ✓ | yes |
| SELECTION API §3 Selection interface | `deleteFromDocument()`: "The method must invoke deleteContents() on this's range if this is not empty and both focus and anchor are in the document tree. Otherwise the method must do nothing." | empty → nothing; non-empty × anchor and focus both in the document tree → `deleteContents()` on the range; non-empty × either not in the document tree → nothing | `SelectionState::delete_from_document` (`selection.rs:513`, → `delete_contents` `:533`) | ✗ (the "both in the document tree" check is absent at base: elidex runs `deleteContents()` whenever the selection has a range (`selection.rs:518-533`); the prereq slice `tsel` of PM 884 / 885 takes it, §11 P-5. The cells of §7 are all in the "both in the document tree" branch) | yes |
| SELECTION API §3 Selection interface | stringifier: "The stringification must return the string, which is the concatenation of the rendered text if there is a range associated with this." | rangeCount 0 / 1 | `SelectionState` `to_string` (`selection.rs:580`, → `Range::to_string` `:585`) | ✗ ("rendered text" is not defined in Selection API and not linked to a definition, so rank (1) does not fix the string for a range over a host; P4 / V5's values are rank (2): see below) | yes |

**Lookups** (all `.claude/tools/webref`, 2026-11-21): `heading dom 5.6` → §5.6 Interface Range `#interface-range`
(the Range code's comments say "§5.5", but `heading dom 5.5` → §5.5 Interface StaticRange; see §11 P-3); `dfn dom contained` → §5.6 `#contained`; `dfn dom 'partially contained'` →
§5.6 `#partially-contained` (not used by the walk: elidex's extract does not compute partially contained children);
`body dom contained`; `body dom dom-range-deletecontents` (steps 1-11); `body dom concept-range-extract` (steps 1-21);
`body dom dom-range-stringifier` (steps 1-6); `heading dom 4.2.2` → §4.2.2 Shadow tree `#shadow-trees`;
`body dom shadow-trees`; `dfn dom remove` → §4.2.3 Mutation algorithms `#concept-node-remove`, `body dom
concept-node-remove` (steps 1-17, step 16 the record); `dfn dom shadowRoot` → §4.9 `#dom-element-shadowroot`, `body
dom dom-element-shadowroot`; `dfn dom root` → §1.1 Trees `#concept-tree-root`, `body dom concept-tree-root`;
`dfn selection-api deleteFromDocument` → §3 Selection interface `#dom-selection-deletefromdocument`; `dfn
selection-api stringifier` → §3 `#dom-selection-stringifier`; `heading selection-api 3` → §3 Selection interface
`#selection-interface`; `coverage-map dom 5.6 dom 1.1 dom 4.2.2 dom 4.2.3 dom 4.9 selection-api 3` → spec=2, single
PR. The two Selection API quotes are not from `webref body`: `webref body selection-api
dom-selection-deletefromdocument` and `... dom-selection-stringifier` both answer "anchor … not encountered" (the
rendered ReSpec page does not carry the ids its dfns index points at; a webref limit, for the PM / CI/tools). They
are read from webref's own cached ReSpec source of the spec
(`~/.cache/elidex-webref/c0d97c3dfdf1cb8a59fa73f678d007b67d8cef81`, `https://w3c.github.io/selection-api/`,
last-modified 2026-06-11), the `<dfn>deleteFromDocument()</dfn>` and `<dfn>stringifier</dfn>` entries, markup
stripped.

**Rank of the Selection string values (P4a/b/c, V5).** Derived by the basis order of CLAUDE.md (*Design philosophy*,
the 至上命題 bullet, verbatim):

> 根拠の順位: (1) 仕様 → (2) 仕様が沈黙なら「ECS ベースのブラウザとしてどうあるべきか」(本節の原則) → (3) それでも決まらない時だけ既存ブラウザの実測を「証拠」として記録。

1. **Spec.** Selection API §3's stringifier says "the concatenation of the rendered text"; "rendered text" is not
   defined in Selection API and is not linked to a definition. HTML §3.2.7 The innerText and outerText properties
   has *rendered text collection steps* (`webref dfn html 'rendered text collection steps'` → §3.2.7
   `#rendered-text-collection-steps`, 2026-11-21), but Selection API does not reference it. Read literally
   ("rendered"), a closed host's shadow text is rendered and an unslotted light child of the host is not, so that
   reading points the other way (towards base-like values that include "SHADOW"). It is a plausible reading, not the
   spec's definition: rank (1) does not fix the value.
2. **elidex principle.** The value is already decided by the ratified umbrella: §7.4 (N6) re-syncs
   `#11-shadow-root-accepted-as-ref-or-old-child` to "this also stops Range.prototype.toString and
   Selection.prototype.toString from including a shadow tree's text (probed in tq)" (PM 812 / 820;
   `git show 4918a6c7:docs/plans/2026-11-tree-model-shadow-root-is-a-root-umbrella.md`, `:1089-1099`). The
   principle it rests on is the CLAUDE.md *Design philosophy* bullet whose head reads, verbatim:

   > **Security by structure, not review convention**: security boundary は後付けしない。

   A closed shadow root's text is behind the encapsulation boundary that
   `tq` closes on its Range path (umbrella §0.1 item 6), and the boundary is held by the one walk itself, not by a
   check added per caller. The discipline applied is *One issue, one way* (CLAUDE.md *Design discipline*):
   Selection's stringification delegates to the one Range stringification (`selection.rs:585`, `with_range(id, dom,
   Range::to_string)`), so a selection over a range has that range's §5.6 string and no second computation exists.
3. **Re-derivation trigger.** If Selection API defines "rendered text" (or links a definition), V5's expectation
   (and P4a/b/c's spec column) is re-derived from that definition.

No browser is measured for it (rank (3) is not reached).

**Breadth**: K=2 specs (DOM, Selection API), M=10 rows (preflight: `total entries (M): 10`; its label table has no Selection API entry, so those two rows are "unmapped-label" (`unmapped-label rows: 2`) and their citations are the `dfn selection-api` lookups above). **Split decision**: single PR (a terminal slice).

### §3.1 User-input touch audit

All inputs are author-controlled: the range's boundary points (`setStart` / `setEnd` / `selectNode` / Selection's
`addRange` / `setBaseAndExtent`), the tree shape, `attachShadow`'s mode. `tq` changes the output of five natives,
all through the one function:

| Native (`crates/script/elidex-js/src/vm/host/`) | Path | Change |
|---|---|---|
| `Range.prototype.deleteContents` (`range_proto_mutation.rs:68`, → `:93`) | `delete_contents` | declared (§0.3 item 1) |
| `Selection.prototype.deleteFromDocument` (`dom_selection_mutation.rs:312`, → `delete_selection_contents` `:121` → `:140`) | `delete_from_document` → `delete_contents` | the same declared change (§0.3 item 1) |
| `Range.prototype.extractContents` (`range_proto_mutation.rs:103`, → `:122`) | `extract_contents` | none observable (§0.3 item 3) |
| `Range.prototype.toString` (`range_proto.rs:890`, → `:898`) | `Range::to_string` | declared (§0.3 item 2) |
| `Selection.prototype.toString` (`dom_selection_proto.rs:643`, → `:650`) | `SelectionState` `to_string` → `Range::to_string` | declared (§0.3 item 2) |

Exposure decreases: a closed shadow root is no longer handed to page script through a `MutationRecord`, the host's
links no longer diverge, and shadow-tree text no longer leaks into a light-tree range's string. Adjacent
pre-existing, unchanged: the overshoot (§0.2) and the upward step (a range that starts inside a shadow tree still
walks out of it; `tr`).

## §4. Mechanism

### §4.1 The exposed accessors (elidex-ecs, unchanged)

- `EcsDom::children_iter` (`crates/core/elidex-ecs/src/dom/tree/navigation.rs:305`) starts a `ChildrenIter` at the
  stored first child; `ChildrenIter`'s `next` (`crates/core/elidex-ecs/src/dom/mod.rs:898-916`) advances by the stored
  next sibling and skips every entity carrying a `ShadowRoot` component (`mod.rs:911`), with a `MAX_ANCESTOR_DEPTH`
  iteration cap (`mod.rs:49`, 10 000). So `children_iter(x).next()` is the first stored child of `x` that is not a
  shadow root, i.e. the first exposed child (the first node-tree child): it is `get_first_child(x)` unless that is
  `x`'s shadow root, in which case it is the stored sibling after it. A host has at most one shadow root
  (`attach_shadow` refuses a second, `crates/core/elidex-ecs/src/dom/shadow.rs:188`), so the skip is of at most one
  entity and the cap is never what ends it.
- `next_exposed_sibling` (`navigation.rs:155-164`) walks the stored next-sibling chain and returns the first
  entity without a `ShadowRoot` component (`navigation.rs:158`); its doc says it "Mirrors the filtering
  `children_iter` applies". So `next_exposed_sibling(n)` equals `get_next_sibling(n)` unless that is a shadow root,
  in which case it is the stored sibling after it (or `None`).
- Both are already the steps other dom-api algorithms take to pick a child: the ChildNode / ParentNode mixins, `splitText`, `insertNode`
  and the self-reference advance (umbrella §1.6, the `apply_insert_before` population). They are census C-4's
  compensating accessors, which `tc` renames into the one child set.

### §4.2 Where the output changes (by walk shape)

The walk differs from base exactly where a raw step would return a shadow root. With the storage at base
(`attach_shadow_with_init` appends the shadow root to the host's stored child list, `shadow.rs:210`), that is:

| Step from | Base | `tq` | Effect |
|---|---|---|---|
| a host whose stored first child is its shadow root (no light child stored before it) | the shadow root, then its tree | the first light child stored after it (the "after" shape: script order `attachShadow` → `appendChild`), or else the host's next tree sibling / an ancestor's (the "no light" shape) | declared; both branches have cells (§6.1, §7), and mutant M3 drops the first |
| the last light child stored before the host's shadow root | the shadow root, then its tree | the light child stored after it, or else the climb | declared |
| any other node (including a shadow root itself, a node inside a shadow tree, and the climb out of one) | unchanged: neither raw step returns a shadow root there, because a shadow root is only ever a stored child of its own host | unchanged | none |

A host inside a shadow tree (a nested host) is the same case one level down: a range inside the outer shadow tree
no longer enters the inner one (still the declared change; *contained* is the same test).

**The same-root premise of the climb row.** The third row's "the climb out of one" matters only for a range whose
start is inside a shadow tree and whose end is not, i.e. a range with two roots. Every path that writes a range
boundary at `3cbd5b88`, and what holds on it:

| Write path | Same root kept? | Basis |
|---|---|---|
| Range natives `setStart` / `setEnd` (`range_proto.rs:552`, `:567`) → `set_start_to_boundary` / `set_end_to_boundary` (`range/boundary.rs:52`, `:67`); `setStartBefore` / `-After`, `setEndBefore` / `-After`, `selectNode` (`boundary.rs:82-128`) through the same two | yes | the two collapse the other boundary when the new one's `find_tree_root` differs, and `find_tree_root` stops at the `ShadowRoot` marker (`walkers.rs:408-426`) |
| `selectNodeContents`, `collapse` (`boundary.rs:114-137`) | yes | both boundaries are set to one node |
| the live-range hooks (`LiveRangeBridge`, `range/live.rs:112-300`: removal, insertion, text change, replace data, split text, normalize merge) | yes, read from the code (not probed) | each hook keeps a boundary in its node, moves it within its node's tree (`splitText`'s new node, `normalize`'s merge target), or moves a boundary inside a removed subtree to `(parent, index)` of the removed node, whose root is the root the other boundary already had |
| `LiveRangeRegistry::finalize_pending` (`range/live.rs:434-450`, run by every `with_range` / `with_range_mut`) | **not shown** | it re-homes start and end to the owner document **separately**, each when its own container is no longer in the `EcsDom` (`!dom.contains`), so a range whose start container is gone and whose end container is alive in a shadow tree becomes `(document, 0)`-`(end in the shadow tree)`. Whether a page script can reach that one-ended re-home (a range container despawned without the removal hook having moved the boundary) is **not measured**; §11 P-6 |
| Selection setters (`collapse`, `extend`, `set_base_and_extent`, `select_all_children`, `selection.rs:278-440`) → raw `set_start` / `set_end` (`boundary.rs:35`, `:42`) after a node-document check only (`same_document`, `selection.rs:613-618`) | **no** (measured, §11 P-5) | a Selection range can have its start in the light tree and its end in a shadow tree; `tq` changes that shape's output. Precondition: `tsel` (§11 P-5) |
| `range_proto_mutation.rs:159-160` (raw `set_start` / `set_end` on a collapsed transient for `insertNode`) | yes | one node, so one root |

So the claim this memo makes is scoped: on the Range natives and the live-range hooks a range has one root and the
climb row is unchanged by `tq`; on the Selection setters it is not, until `tsel` (§11 P-5); for `finalize_pending`
it is not shown (§11 P-6).

### §4.3 ECS-native / layering check

`elidex-dom-api` is the engine-independent crate the algorithm belongs to (CLAUDE.md, *Layering mandate*); no
`vm/host/` code changes. No component, no side-store, no new accessor: the edit takes the program's own relation
(tree children) early on one function (umbrella §1.6, R-TREE 4), so `tc` renames it with the rest and nothing is
deleted later.

## §5. Populations

### §5.1 Every caller of `next_in_preorder_global`, and what it feeds

`git grep -n -w 'next_in_preorder_global' -- crates` at `3cbd5b88` → 4 hits:

| Hit | Kind |
|---|---|
| `crates/dom/elidex-dom-api/src/range/mod.rs:573` | definition (private `fn`) |
| `crates/dom/elidex-dom-api/src/range/mutation.rs:9` | `use super::{..}` import |
| `crates/dom/elidex-dom-api/src/range/mutation.rs:49` | call in `Range::to_string` |
| `crates/dom/elidex-dom-api/src/range/mutation.rs:92` | call in `contained_top_level_nodes` |

The function is private to `range` and has no test caller. Downstream, `git grep -n -P
'\b(contained_top_level_nodes|delete_contents|extract_contents|delete_from_document)\(' -- crates ':!*test*'` → 11
hits: the four definitions (`mutation.rs:89`, `:147`, `:266`, `selection.rs:513`), one comment (`selection.rs:524`)
and six calls: `mutation.rs:216` and `:370` (`contained_top_level_nodes`), `selection.rs:533` (`delete_contents`),
`range_proto_mutation.rs:93` and `:122` (the two Range natives), `dom_selection_mutation.rs:140`
(`delete_from_document`). And `git grep -n -P 'Range::to_string|\.to_string\(\s*dom\s*\)|\.to_string\(\s*reg\b' --
crates ':!*test*'` → 3 hits: `selection.rs:585` (`with_range(id, dom, Range::to_string)`), `range_proto.rs:898`,
`dom_selection_proto.rs:650` (the two natives).

| Caller | Feeds | Output changes? | Reason |
|---|---|---|---|
| `Range::to_string` (`mutation.rs:49`) | `Range.prototype.toString` (`range_proto.rs:898`); `SelectionState` `to_string` (`selection.rs:585`) → `Selection.prototype.toString` (`dom_selection_proto.rs:650`) | yes, declared (§0.3 item 2) | the walk appends every Text node it passes; at base it passes a host's shadow tree (probes P3, P4) |
| `contained_top_level_nodes` (`mutation.rs:92`) → `delete_contents` (`:216`) | `Range.prototype.deleteContents` (`range_proto_mutation.rs:93`); `SelectionState::delete_from_document` (`selection.rs:533`) → `Selection.prototype.deleteFromDocument` (`dom_selection_mutation.rs:140`) | yes, declared (§0.3 item 1) | at base the shadow root passes the `compare_points` filter and is removed with a record (probes P1, P5) |
| `contained_top_level_nodes` (`mutation.rs:92`) → `extract_contents` (`:370`) | `Range.prototype.extractContents` (`range_proto_mutation.rs:122`) | no | at base the shadow root reaches `apply_append_child`, which refuses it and returns no record (probe P2); from `tq` it is not a candidate |

No caller changes beyond the declared change. One feed is not named by the umbrella: `Selection.deleteFromDocument`
(§11 P-1). It runs the same `delete_contents`, so its change is the declared one reached through a second native, as
`toString` is through two; it gets its own cell (V2).

### §5.2 Existing cells

No existing cell has a shadow tree's node in its expectation. Measured, not read: with the two-line edit applied
temporarily at `3cbd5b88` (and reverted), `cargo test -p elidex-dom-api --all-features` → 845 passed, 0 failed, and
`cargo test -p elidex-js --all-features --lib -- range selection mutation_observer shadow` → 493 passed, 0 failed
(rustc 1.99.1, `unset RUSTUP_TOOLCHAIN`, `-j 2`). Search for the cells that combine a shadow root with a Range
method: for each file of `git ls-files crates | /usr/bin/grep -i -E 'range|selection|shadow' | /usr/bin/grep -i
test`, `/usr/bin/grep -c -E 'attach_shadow|attachShadow'` and `/usr/bin/grep -c -E
'deleteContents|extractContents|toString|delete_contents|extract_contents|to_string|deleteFromDocument|delete_from_document'`:
two files have both. `crates/dom/elidex-dom-api/src/range/tests.rs` (1 and 25): its one shadow cell (`:254-277`)
is `insert_node` rejecting a shadow root, which does not read the walk.
`crates/script/elidex-js/src/vm/tests/tests_shadow_dom/attach.rs` (55 and 1): the one hit is the test name
`attach_shadow_mode_coerces_via_to_string` (`:176`), a dictionary-coercion cell with no Range.

## §6. Base probes (run at `3cbd5b88`, 2026-11-21)

Umbrella §5.2 asks `tq` to probe, at its base and before review, the `deleteContents` probe of census C-14 (second
row), `extractContents` over the same host, and the `toString` facet through both natives with a range whose end
container follows the host. Run as one temporary VM test (`tests_shadow_dom/probe_tq.rs` under `crates/script/elidex-js/src/vm/tests/`,
a temporary file (NEW: removed),
registered in that directory's `mod.rs`; both removed afterwards, `git status` clean):

```
unset RUSTUP_TOOLCHAIN && rustc --version && \
  cargo test -j 2 -p elidex-js --all-features --lib tq_probe_all -- --nocapture
# rustc 1.99.1 (58cf8f9c3 2026-10-13)
```

Tree (each probe on a fresh document): `body` = [`pre` (Text "pre"), `h` (div), `post` (Text "post")]; for the
"light" shape `h` first gets a Text child `light` ("LIGHT"), stored before the shadow root; then `sr =
h.attachShadow({mode: "closed"})` holds `sp` (span) with Text "SHADOW". A `MutationObserver` observes `h` with
`{childList: true}`. The link columns are read from the `EcsDom` after the script (stored parent of `sr`,
`get_shadow_root(h)`, `sr` in `h`'s stored child list).

| # | Script | Observed at base | Spec value | With the edit applied (temporary) |
|---|---|---|---|---|
| P1 | `rg.setStart(h, 0); rg.setEnd(h.parentNode, idx(h) + 1); rg.deleteContents()` (no light child) | no throw; 1 record, `target === h`, `removedNodes` = [`sr`] (the closed root, in page script's hands); `sp.isConnected` true, `sr.host === h`; `sr` out of `h`'s stored child list (stored parent ≠ `h`) while `get_shadow_root(h) === sr` | no node is contained (`h` is the start node; `sr` and its tree have root `sr`): 0 records, nothing removed | 0 records; links and stored list as before the call |
| P2 | same range, `rg.extractContents()` | fragment has 0 children, 0 records; links intact | the same: no contained child, empty fragment | the same |
| P3a | `rg.setStart(pre, 1); rg.setEnd(post, 2); rg.toString()` (no light child) | `"reSHADOWpo"` | `"repo"` (steps 3, 4, 5; "SHADOW" is not contained) | `"repo"` |
| P3b | the same, light shape | `"reLIGHTSHADOWpo"` | `"reLIGHTpo"` | `"reLIGHTpo"` |
| P4a | P3a's range through `getSelection().addRange(rg); getSelection().toString()` | `"reSHADOWpo"` | rank (2), not the spec's: `"repo"`, P3a's Range string (§3, *Rank of the Selection string values*) | `"repo"` |
| P4b | the same, light shape | `"reLIGHTSHADOWpo"` | rank (2): `"reLIGHTpo"` | `"reLIGHTpo"` |
| P5 | P1's range through `addRange` + `getSelection().deleteFromDocument()` | as P1: 1 record naming `sr`; links diverge | as P1 | as P1 at `tq`: 0 records, links intact |
| P6 | light shape, `rg.setStart(pre, 1); rg.setEnd(post, 2); rg.deleteContents()` (the host is contained) | `h` removed whole (`body` has 2 children, `sp.isConnected` false); `sr` stays `h`'s; no record on `h` | `h` is contained and removed with its shadow tree | the same (no change: the shadow root is dominated by `h`) |

P1 reproduces census C-14's second row at this base. P2's "unchanged" and P6's "unchanged" confirm the umbrella's
prediction. Mutants, run the same way: raw first child restored at `:574` alone → P1 and P3a back to base, P3b
stays fixed; raw next sibling restored at `:579` alone → P3b back to base (`"reLIGHTSHADOWpo"`), P1 and P3a stay
fixed. So each mutant needs its own shape, and §7 has both. Each probe is run again at the PR head (§10).

### §6.1 The "after" shape (R1, F1; run at `3cbd5b88`, 2026-11-21)

The "light" shape of §6 stores the light child before the shadow root. The other script order, `sr =
h.attachShadow({mode: "closed"})` (holding `sp` with Text "SHADOW") and then `h.appendChild(light)` (Text "LIGHT"),
stores it after the root (`attach_shadow_with_init` appends the root, `shadow.rs:210`; `appendChild` appends after
it). No probe of §6 and no cell built that shape, so a first-child step that returns `None` when the stored first
child is a shadow root (mutant M3 below) survived every cell. Probed the same way as §6 (one temporary VM test,
`tests_shadow_dom/probe_tq.rs` (NEW: removed) registered in that `mod.rs`; both removed afterwards, `git status`
clean; `unset RUSTUP_TOOLCHAIN && rustc --version && cargo test -j 2 -p elidex-js --all-features --lib
tq_probe_after -- --nocapture`, rustc 1.99.1 (58cf8f9c3 2026-10-13)). The `MutationObserver` observes `h` with
`{childList: true}`, read with `takeRecords()`; the link columns are read from the `EcsDom` after the script.

| # | Script (after shape) | Observed at base | Spec value | With the edit applied (temporary) | Mutant M3 (temporary) |
|---|---|---|---|---|---|
| P1c | P1's range, `rg.deleteContents()` | 2 records on `h`: `removedNodes` [`sr`], then [`light`]; `sp.isConnected` true, `sr.host === h`; `h`'s stored child list empty, stored parent of `sr` ≠ `h`, `get_shadow_root(h)` still `sr` | `light` is the one contained node (its root is the range's, `(light, 0)` is after `(h, 0)`): 1 record, `removedNodes` [`light`] | 1 record, [`light`]; stored list [`sr`], stored parent of `sr` = `h` | 0 records; `light` stays (`h.childNodes.length` 1) |
| P2c | P1's range, `rg.extractContents()` | fragment [`light`], 1 record [`light`]; links intact, stored list [`sr`] | the same | the same | fragment empty, 0 records |
| P3c | P3a's range, `rg.toString()` | `"reSHADOWLIGHTpo"` | `"reLIGHTpo"` | `"reLIGHTpo"` | `"repo"` |
| P4c | P3a's range through `addRange`, `getSelection().toString()` | `"reSHADOWLIGHTpo"` | rank (2): `"reLIGHTpo"` | `"reLIGHTpo"` | `"repo"` |
| P5c | P1's range through `addRange`, `getSelection().deleteFromDocument()` | as P1c at base | as P1c | as P1c with the edit | as P1c under M3 |

Mutants on this shape, the same run: M3 (`range/mod.rs:574` = `dom.get_first_child(current).filter(|c|
!dom.is_shadow_root(*c))`, the sibling step as in `tq`) turns every row red, P2c included (it is a killer through
`extract` too, but F1 does not give U3 / V3 this shape). M1 (raw first child at `:574`, exposed sibling at `:579`):
P1c, P3c, P4c, P5c back to base; P2c as at base and at `tq` (no kill). M2 is not killed by this shape (the light child is the last
stored one, so the raw and exposed sibling of it agree; predicted from the code, not run). On the shapes of §6, M3
equals `tq` (no light: the first stored child is `sr`, `None` then the sibling step, as `tq`; light before: the
first stored child is `light`, as `tq`), predicted from the code; hence the new shape.

## §7. Cells and mutants

Where each cell's expected value comes from:
- **Spec values** (§3, rank (1)): the Range cells U2, U3, U4, V1, V3, V4 (DOM §5.6 *contained*, `deleteContents()`,
  *extract*, stringification), and V2 (Selection API §3 `deleteFromDocument()` runs `deleteContents()` on the range,
  so its value is DOM §5.6's `deleteContents()`).
- **An elidex design value, not a spec value**: U1. It asserts what the private walk returns, which is §0.1's
  design (the root clause of *contained* is held by the walk's descent: its two child steps stay in the node tree);
  the spec has no such function.
- **Rank (2)**: V5, and V6 (NEW) (§3, *Rank of the Selection string values*).

None is any engine's. Each shape below is the probe's of §6, §6.1 or §11 P-5.

**Unit, `crates/dom/elidex-dom-api/src/range/tests.rs`** (the module is a child of `range`, so it can call the
private walk):
- U1 `next_in_preorder_global_does_not_enter_shadow_root` (NEW): (a) host with a shadow root and no light child,
  followed by a sibling `n` → `next_in_preorder_global(host)` is `Some(n)`; (b) host with light child `l` stored
  before its shadow root, followed by `n` → `next_in_preorder_global(l)` is `Some(n)`; (c) contrast, unchanged: from
  the shadow root itself → its first child; (d) the after shape (§6.1): host whose shadow root is stored first and
  light child `l` after it → `next_in_preorder_global(host)` is `Some(l)`.
- U2 `delete_contents_over_host_leaves_shadow_root_linked` (NEW): P1 shape → returned records empty; P1-light shape
  (range `(host, 0)`-`(parent, index(host) + 1)` with `l` stored before the root) → exactly one record, `target`
  host, `removed_nodes` = [`l`] (deleteContents step 4: `l` is contained, the root is not); P1c shape (the same
  range, `l` stored after the root, §6.1) → exactly one record, `target` host, `removed_nodes` = [`l`]; in all three,
  no record's
  `removed_nodes` holds the shadow root or a node of its tree, and `get_parent(sr)`, `get_shadow_root(host)`,
  `shadow_host(sr)` read the same after the call as before it (storage-neutral: the cell says "unchanged by the
  call", so it holds after `ts` too).
- U3 `extract_contents_over_host_leaves_shadow_root_linked` (NEW): P2 shape → fragment has no child, no record,
  links unchanged; P2-light shape → fragment children = [`l`], the shadow root still the host's (`get_shadow_root`,
  `shadow_host` unchanged), no record names it. Green at base and head (the pin of §0.3 item 3; the no-light shape is probe P2, the light
shape is predicted: at base the shadow root is a candidate and `apply_append_child` refuses it).
- U4 `to_string_over_host_skips_shadow_tree_text` (NEW): P3a shape → `"repo"`; P3b shape → `"reLIGHTpo"`; P3c shape
  → `"reLIGHTpo"`.

**VM end-to-end** (closed mode and the script-visible record):
- V1 `delete_contents_over_closed_host_reports_no_shadow_root` (NEW), in
  `crates/script/elidex-js/src/vm/tests/tests_mutation_observer/range_ops.rs`: P1 → `records.length` 0,
  `sp.isConnected` true, `sr.host === h`; P1-light → 1 record, `target === h`, `removedNodes` = [`light`],
  `h.childNodes.length` 0; P1c → 1 record, `target === h`, `removedNodes` = [`light`], `sp.isConnected` true,
  `sr.host === h`.
- V2 `selection_delete_from_document_over_closed_host_reports_no_shadow_root` (NEW), same file: P5 → as V1's P1.
- V3 `extract_contents_over_closed_host_reports_no_shadow_root` (NEW), same file: P2 → fragment empty, 0 records,
  `sp.isConnected` true.
- V4 `range_to_string_skips_shadow_tree_text` (NEW), in `crates/script/elidex-js/src/vm/tests/tests_range.rs`: P3a
  → `"repo"`, P3b → `"reLIGHTpo"`, P3c → `"reLIGHTpo"`.
- V5 `selection_to_string_skips_shadow_tree_text` (NEW), in
  `crates/script/elidex-js/src/vm/tests/tests_selection.rs`: P4a → `"repo"`, P4b → `"reLIGHTpo"` (rank (2), §3).
- V6 `selection_cross_root_set_base_and_extent_to_string_is_empty` (NEW), same file: §11 P-5's tree (`body` =
  [`pre` "pre", `h`, `post` "post", `p` "TAIL"], `h` closed shadow root holding `sp` > `st` "SHADOW");
  `getSelection().setBaseAndExtent(pre, 1, st, 3); getSelection().toString()` → `""`. This is the post-`tsel`
  expectation (the range collapses at `(st, 3)`; Selection string rank (2)): V6 is red until `tsel` lands (with the
  edit alone it reads `"repostTAILSHA"`, at base `"reSHA"`, §11 P-5), which is why `tq` lands after `tsel` (§9,
  §10). It pins `tq`'s dependency: on `tq`'s base, `setBaseAndExtent` no longer hands the walk a cross-root range.

V1-V6 build the tree in a connected `body` (the shadow DOM tests' `build_doc` shape) so `isConnected` is meaningful;
the MO cells read records in a second `eval` after the first one's microtask checkpoint, as the existing
`range_ops.rs` cells do.

**Mutants** (run at the final head only, at most 2 in parallel):
- M1 `range/mod.rs:574` back to `dom.get_first_child(current)` → red: U1 (a) and (d), U2 / V1 / V2 P1 shape, U2 /
  V1 P1c shape, U4 / V4 / V5 no-light shape, U4 / V4 P3c shape.
- M2 `range/mod.rs:579` back to `dom.get_next_sibling(node)` → red: U1 (b), U2 / V1 P1-light shape, U4 / V4 / V5
  light shape.
- M3 `range/mod.rs:574` = `dom.get_first_child(current).filter(|c| !dom.is_shadow_root(*c))` ("`None` if the first
  stored child is a shadow root") → red: U1 (d), U2 P1c shape, U4 P3c shape, V1 P1c, V4 P3c. Green on every other
  shape (it equals `tq` there, §6.1).

Observed in the plan's run (§6, §6.1): M1 on the P1, P3a, P1c, P3c, P4c and P5c shapes, M2 on the P3b shape, M3 on
P1c, P2c, P3c, P4c and P5c. The other kills listed are
predicted from the same walk (the P1-light shape under M2: the raw sibling of `l` is the shadow root, which then passes
the `compare_points` filter as in P1) and are checked at the final head (§10). U3 / V3 are not mutant killers (green
at base); they pin that `extract` no longer depends on `apply_append_child`'s refusal.

### §7.1 1000-line re-count at `3cbd5b88`

`wc -l crates/dom/elidex-dom-api/src/range/tests.rs crates/script/elidex-js/src/vm/tests/tests_mutation_observer/range_ops.rs
crates/script/elidex-js/src/vm/tests/tests_range.rs crates/script/elidex-js/src/vm/tests/tests_selection.rs
crates/dom/elidex-dom-api/src/range/mod.rs crates/dom/elidex-dom-api/src/range/mutation.rs` → 453, 586, 443, 787, 589,
651 (the first five the same as umbrella §8 at `2ac9fa44`; `range/mutation.rs` takes a comment-only edit). None
is at or over 1000, and the largest after the cells (`tests_selection.rs`, about +50 with V5 and V6) stays under
1000, so CLAUDE.md's
*1000-line debt* rule asks for no split and no new test file.

## §8. Landing items

From umbrella §7.4 at `4918a6c7`, by reference only, plus the one item this plan PR adds there (below):

- **At `tq`** (umbrella `:1100-1101`; PM 882 (1)). The text, verbatim, in the umbrella §7.4 style:

  > `#11-shadow-root-accepted-as-ref-or-old-child`: gains "Range path closed by tq (<sha>, <date>): both deleting
  > natives, `Range.prototype.deleteContents` (`native_range_delete_contents`, elidex-js
  > `vm/host/range_proto_mutation.rs`) and `Selection.prototype.deleteFromDocument`
  > (`native_selection_delete_from_document`, elidex-js `vm/host/dom_selection_mutation.rs`, through
  > `SelectionState::delete_from_document`), by the one fix in the shared `Range::delete_contents` (elidex-dom-api
  > `range/mutation.rs`, whose candidates come from `contained_top_level_nodes`)"; the row stays open.

  The row closes at `ta` (umbrella §7.1). The quoted text names files and symbols only, no line numbers, so it does
  not drift; at `3cbd5b88` the symbols are at `range_proto_mutation.rs:68`, `dom_selection_mutation.rs:312`,
  `selection.rs:513`, `mutation.rs:147`.
- The landing list `pending-at-landing/tree-model-umbrella.md` ("Later") addresses umbrella §7.4 by item heading
  ("At `tq`", "(N6)", ...), not by line; the umbrella line numbers in this section are this memo's citations at
  `4918a6c7` only.
- (N6) (umbrella `:1089-1099`, "at `tq`'s landing or at this umbrella's ratify, whichever is first"): ratify came
  first; the PM's landing list records it as applied on `4918a6c7` (pmo 8, 2026-11-20). Nothing left for `tq`.

- **At `tq`, added by this plan PR** (umbrella `:1102-1103`; erratum carried by this plan PR (PM 882 (2)), reviewed
  in plan-review R1's re-check). The text, verbatim:

  > `#11-range-compare-points-shadow-index`: gains "D13 (b) (the walk) fixed by tq (<sha>, <date>); (a) stays tc's";
  > the row stays open (erratum, tq plan PR, PM 882 (2)).

  It lands in the slot that tracks D13: the ledger row `#11-range-compare-points-shadow-index` points at
  `docs/plans/2026-11-range-contents-program.md:731`, the D13 row of its §0.4 defect table (D13 (a): the
  boundary-point comparison's child index, `range/mod.rs:549-571`; D13 (b): the pre-order walk, `range/mod.rs:573-584`),
  closed by `tc` (umbrella §7.4 "At `tc`"). The ledger has that one row for it (`/usr/bin/grep -c
  'range-compare-points-shadow-index'` on the ledger → 1, at `:1077`; the ledger names D13 only through the `:731`
  pointer, `grep -c 'D13'` → 0), and the program's D13 row names that slot and no other.

## §9. Gate

1. The plan PR: this memo plus the one-item umbrella §7.4 erratum of §8, through `/elidex-plan-review` in full
   (umbrella §5 `tq`: required; *Skip OK* closed, ≥3 axes).
2. Then the code PR, one PR from a worktree on `origin/main`: the edits of §0.1 (the walk's two steps and its doc
   comment in `range/mod.rs`, the comment-only edit in `range/mutation.rs`) and the cells of §7, nothing else.
3. Prereq landed: the code PR merges only after `tsel` (`#11-selection-boundary-setter-skips-root-check`, PM 884 /
   885) has landed on `origin/main` (§11 P-5); its base is a `main` that contains `tsel`.

## §10. Definition of done

1. `next_in_preorder_global` reads no raw child or sibling: `git grep -n -P '\b(get_first_child|get_next_sibling)\('
   -- crates/dom/elidex-dom-api/src/range/mod.rs` → 2 hits, both in `child_index_in_ancestor` (`:558`, `:564` at
   base; at base the same command gives 4, these two plus `:574`, `:579`), and the walk's body holds
   `children_iter(current).next()` and `next_exposed_sibling(node)`; its `get_parent` line is unchanged. For the code
   PR, `git diff origin/main --stat` → `range/mod.rs`, `range/mutation.rs` (comment only: `git diff origin/main --
   crates/dom/elidex-dom-api/src/range/mutation.rs` changes `///` lines only) and the four test files of §7.1 only.
2. U1-U4 and V1-V6 present and green; M1, M2 and M3 each observed red by the cells listed in §7 at the final head.
3. The probes P1-P6 and P1c-P5c re-run at the head (the temporary test, removed again) give the "with the edit
   applied" column of §6 and §6.1.
4. `cargo test -p elidex-dom-api --all-features`, `cargo test -p elidex-js --all-features`, then `mise run ci`.
5. The new doc comment cites WHATWG DOM §5.6 *contained* (`#contained`), by the number `heading dom 5.6` gives, and
   names `#11-shadow-root-is-a-tree-root` for the unchanged upward step; the rewritten `contained_top_level_nodes`
   comment (rewritten as a whole) cites WHATWG DOM §5.6 for *contained* and for the `nodesToRemove` paragraph it
   keeps, holds no "§5.5", and states the three clauses of *contained* and which part enforces which (§0.1).
6. Prereq landed: `tsel` (`#11-selection-boundary-setter-skips-root-check`) is in the code PR's base (§9 item 3),
   and V6 is green there.

## §11. Premises found wrong or not foreseen

- **P-1** The umbrella names three users of the walk and two natives for `toString` (§1.6, *What the edit
  reaches*), but not `Selection.deleteFromDocument`, which runs `delete_contents` (`selection.rs:533`) and so
  carries the same exposure (probe P5). Its change is the declared one (the same algorithm), not a change beyond it,
  so this memo names it (§0.3, §3.1, §5.1) and pins it (V2) instead of stopping. Placed by PM 882 (1): the "At
  `tq`" landing item carries both natives (§8).
- **P-2** Umbrella §7.4 has no item for D13 (b) of the Range contents program, which `tq` fixes (the walk half of
  D13; census C-3 already says the function "leaves this population at `tq`'s head"). The slot
  `#11-range-compare-points-shadow-index` stays `tc`'s for D13 (a). Placed by PM 882 (2): this plan PR carries the
  umbrella §7.4 erratum that records the (b) half (§8).
- **P-3** The Range code's comments cite "DOM §5.5" for Range 47 times in `range/` (`git grep -n -F '§5.5' --
  crates/dom/elidex-dom-api/src/range | wc -l` → 47; `§5.6` → 1); `webref heading dom 5.5` → Interface StaticRange,
  `5.6` → Interface Range. Pre-existing and not `tq`'s; `tq`'s own new comments use §5.6, and its rewrite of the
  `contained_top_level_nodes` comment takes two of the 47 (`mutation.rs:72`, `:77`), so the count after `tq` is 47 − the lines `tq` rewrites
  (45 expected, measured at the code PR). Placed by PM 882 (3) under
  the sweep `project_dom-events-section-renumber-sweep`. Three facts for the PM's sweep record (the record is the
  PM's): its pattern `§\s?5\.5(?![.0-9])` misses a sentence-final "§5.5." (`git grep -n -P` with it on
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` → 5 hits of the file's 7 `§5.5`; the two missed are
  `:211` and `:227`); `crates/script/elidex-js/src/vm/host_data/mod.rs:352` ("§5.5)") is not in its list; and
  Layout's branch `layout-nodekind-inference-shim-removal` (`712b32d5`) already rewrites 11 of the 47 in `range/`
  (`git diff $(git merge-base 3cbd5b88 712b32d5) 712b32d5 -- crates/dom/elidex-dom-api/src/range` → 11 removed lines
  with `§5.5`; at `712b32d5` the `range/` count is 36). Reported to the PM; applied in the sweep record
  (`project_dom-events-section-renumber-sweep`).
- **P-4** The brief's "§2 rows … LINK / MO that name tq": umbrella §2 has no LINK or MO row naming `tq`; LINK and MO
  are elements of the matrix, and the rows naming `tq` are TREE × WR (the child half) and CHILD × RANGE. §2 above
  writes the LINK and MO intersections on the existing pairs TREE × LINK and CHILD × MO.
- **P-5** (found while writing F3 of R1; decided by PM 884 / 885: a precondition, below) The R1 decision record
  states that the raw `set_start` / `set_end` are in-crate test helpers, so that every range has one root and the
  climb row of §4.2 is not a new exposure. At `3cbd5b88` they are also the production write path of
  `SelectionState`: `collapse`
  (`selection.rs:278-279`), `extend` (`:357-361`), `set_base_and_extent` (`:408-412`), `select_all_children`
  (`:439-440`), each after a node-document check only (`same_document`, `:613-618`; a shadow-tree node has the same
  node document), and `range_proto_mutation.rs:159-160` (a collapsed transient for `insertNode`, one node, so one
  root). Selection API §3 `extend()` step 5 collapses on a root mismatch, and `setBaseAndExtent()` step 5 "set the
  start"s through DOM's *set the start or end* (WHATWG DOM §5.6 `#concept-range-bp-set` step 4: "If range's root
  is not node's root …", `webref body dom concept-range-bp-set`), which collapses too; elidex does neither. So a Selection range can
  span two roots, and `tq` changes its output. Probed at `3cbd5b88` (temporary VM test, removed, `git status`
  clean; rustc 1.99.1; tree `body` = [`pre` "pre", `h`, `post` "post", `p` "TAIL"], `h` closed shadow root holding
  `sp` > `st` "SHADOW"):

  | Script | Base | With the edit (temporary) | Spec value |
  |---|---|---|---|
  | `getSelection().setBaseAndExtent(pre, 1, st, 3); getSelection().toString()` | `"reSHA"` (range `(pre, 1)`-`(st, 3)`, not collapsed) | `"repostTAILSHA"` (the walk never meets the end container in the shadow tree, so it runs to the document's end, then adds the end text's head) | the range collapses at `(st, 3)`; Selection string rank (2): `""` |
  | `collapse(pre, 1); extend(st, 3); toString()` | `"reSHA"` | `"repostTAILSHA"` | the same: collapsed, `""` |
  | `setBaseAndExtent(pre, 1, st, 3); deleteFromDocument()` | `pre` → "p", `st` → "DOW", `sp` disconnected, 1 record under `body` | the same | nothing (focus is not in the document tree) |

  Both columns are wrong at base already (the cross-root range is the defect); `tq` changes the `toString` row's
  wrong value, which is not in the declared change of §0.3.

  **Precondition (PM 884 / 885).** `tq` lands after the prereq slice **`tsel`**, slot
  `#11-selection-boundary-setter-skips-root-check`, which makes Selection's boundary setters apply the spec's
  preconditions (its one declared change: "Selection's boundary setters apply the spec's preconditions"). Its design
  is `tsel`'s own plan, not this memo's. Order: tp → `tsel` → `tq` → ta (in the form of umbrella §5.1's order;
  the umbrella's order erratum is carried by `tsel`'s plan PR, not by this one). `tq`'s dependency on it is one cell
  and two gate lines: V6 (§7; the post-`tsel` value `""`, red until `tsel` lands), §9 item 3 and §10 item 6 ("prereq
  landed"). §0.3 is unchanged: removing the Selection setters' cross-root shape is `tsel`'s declared change, so on a
  base that contains `tsel` the `toString` row above is not `tq`'s output and `tq`'s change stays the declared one; V6
  checks that on the code PR's base.
- **P-6** (R1.2, G3; for the PM) `LiveRangeRegistry::finalize_pending` (`range/live.rs:434-450`) re-homes a range's
  start and end to the owner document separately, each when its own container is no longer in the `EcsDom`, so a
  range whose start container is gone and whose end is alive in a shadow tree becomes cross-root (§4.2, the
  write-path table). Whether a page script can reach that one-ended re-home (a container despawned while a live range
  still points at it, the removal hook not having moved the boundary) is **not measured**; this memo makes no
  same-root claim for that path. Where it belongs (a probe in `tq`'s code PR, `tsel`, or a slot) is the PM's.
