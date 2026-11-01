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

**Rev 3** (2026-11-01) — plan-review R2 (0 CRIT / 12 IMP / 24 MIN) and PM decision 207. The node document of
Range-created nodes (C3's write rule, D8, shallow-clone stamping) leaves the program: it is a facet of
`#11-ownerless-node-creation`, owned by Layout (C0 / C1), and Appendix A lists the Range creation sites for C0 rev 2.
RC-2 is no longer gated on C0. RC-4 adds the `Fragment` value to N9's `ParserScriptingMode` (R2's "N9 owns Fragment"
premise was wrong, §0.3 item 13) and reads the node document through C0's `EcsDom::node_document` (one reader, PM
157). RC-1 depends on #578. RC-0b lands before N9. D13 widens to raw-sibling tree access, and B3 gains a cap policy.
E3 is restated as `DomApiError` plus records. Every §0.4 row is marked predicted or measured, with a green-branch rule.
§6 keeps only non-citation hygiene; citation correctness is one per-PR rule (§6). The inert-document registry is
resolved (§4.3 condition 4). New pairs: X × E, X × P, E × P, X × L. D-a (the explicit created-element list) is not
applied: its equality premise fails at `ee6c7a1a` (§0.3 item 14). **Driver re-decision D-a′** replaces both D-a and
the R2 `ClonedFrom` widening: the contents algorithm returns its **clone roots** in creation order, each with a scope
(`Subtree` / `SelfOnly`), and the CE pass runs today's walk once per root; the form pass and the `ClonedFrom` marker
stay exactly as today (§2 C2, X2, §4.3 conditions 1-3). R5 is withdrawn. SelfOnly's visit set is decided by the
driver (**D-a″**, §0.3 item 17): the root element, then today's walk over the root's shadow root if it has one. **New defect D14** (§0.4, RC-2): the cloner clones shadow
children with the operation's `subtree` flag instead of true (was X4 on the PM list).

## §0. Frame, premises, decisions

### §0.1 The decision and how this memo refines the cut

PM decision 202 fixed an umbrella with four slices: (1) clone/extract canonical, (2) surroundContents, (3) toString,
(4) createContextualFragment. The memo keeps all four and the one-algorithm rule for clone/extract. It changes the
order and adds one mechanism, both forced by the code:

| PM 202 slice | Here | Why it moved |
|---|---|---|
| 3 toString | **RC-1** (first), with the shared containment primitives and `deleteContents` | toString's defect and a `deleteContents` / `extractContents` defect have one root: both walk pre-order and `break` at `end_container` (`crates/dom/elidex-dom-api/src/range/mutation.rs:50-52` and `:93-95`). The fix is one containment primitive. If it landed with clone/extract and toString kept its own walk until slice 3, two enumerations of "nodes in the range" would coexist (strangler). So the primitive lands once and every consumer moves in that PR. |
| 1 clone/extract | **RC-2** | Unchanged in substance: one algorithm parameterised by clone vs move. It consumes RC-1. The node document of the nodes it creates is not this program's (Layout, `#11-ownerless-node-creation`, §8). |
| 2 surroundContents | **RC-3** | Unchanged. It consumes RC-1's fixed `insertNode` (§0.4 D10/D11 are fixed in RC-1, which already rewrites `insertNode`). |
| 4 createContextualFragment | **RC-4** | Independent of RC-1..3 (parser path). It adds the `Fragment` value to the `ParserScriptingMode` enum that IBP-sandbox N9 introduces, with its semantics in both parser backends, and it reads the node document through C0's `EcsDom::node_document` (§5.4, §9). |
| — | **RC-0a / RC-0b** | Prereq pure-move splits (§7). RC-0b lands before N9 (PM 207). |

The added mechanism is the **live-range reference** (§4.2, RC-1): the mutating algorithms read and write the
*registered* live range at the spec's own steps, instead of snapshot-then-commit. The surround algorithm (extract,
then replace-all, then insert, then select, all on the same live range) cannot be written faithfully on a snapshot,
and `insertNode` already shows a silent-wrong caused by the snapshot (§0.4 D12).

The renumbering, the reorder, the added mechanism and the D10 / D11 move are ratification items for the PM (Open
questions, R1-R4). R5 (the created-set carrier) is withdrawn: the carrier is driver decision D-a′ (§4.3).

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
- Raw pre-order walks (D13): `next_in_preorder_global` (`range/mod.rs:573-584`) steps with `get_first_child` /
  `get_next_sibling` / `get_parent`, so from a shadow host it enters the ShadowRoot entity and its tree. toString
  (`range/mutation.rs:49`) and `contained_top_level_nodes` (`:93-95`) use it.
- Caps: `children_iter` / `children` stop at `MAX_ANCESTOR_DEPTH` = 10,000 siblings (`crates/core/elidex-ecs/src/dom/mod.rs:46`,
  `navigation.rs:255-263`); `index_in_parent` returns `None` past it (`navigation.rs:55-75`), and the tree primitives
  then fire no `MutationEvent` (`tree/mutation.rs:46`, `:75`, `:191`). `child_list_uncapped` (`navigation.rs:284`)
  is the uncapped exposed list (skips ShadowRoot); `insert_node` step 13 already uses it (`range/mutation.rs:575-580`,
  Codex PR418 R2).
- `ownerDocument` getter: `dom.owner_document(this).or_else(|| dom.document_root())`
  (`crates/dom/elidex-dom-api/src/node_methods/core.rs:279`). An unstamped node in the bound document's VM reports
  `document`, so `r.extractContents().ownerDocument === document` is green before any fix (it masks R2's D8, now Layout's, §0.4). C0 keeps
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
  `:195-219`, `#[non_exhaustive]`) has no `InvalidNodeTypeError` variant (surroundContents step 2 throws it). The VM
  maps it in one place, `dom_api_error_to_vm_error` (`crates/script/elidex-js/src/vm/host/dom_bridge.rs:390`).
  `RangePointError` (`range/boundary.rs:12-19`, with `InvalidNodeType`) serves `compare_point` /
  `is_point_in_range` only and is mapped in `vm/host/range_proto.rs:820-840`; `SelectionError`
  (`selection.rs:119-133`) is Selection's.
- Cross-lane ownership (read-only):
  - IBP-sandbox N9 `IBP-sandbox-parser-mode` introduces F11's `ParserScriptingMode` (`Normal | Disabled | Inert`)
    in `elidex-html-parser-strict::result`, with the fragment derivation in `html_fragment.rs`
    (`docs/plans/2026-09-ibp-sandbox.md:510-511`). Its D4 **omits** `Fragment` ("its producer is a stub under
    `#11-range-full-impl`", `:182-183`; §3 row `:450` "Fragment omitted (D4)"), so the Fragment value is this
    program's (RC-4). N9 re-points `range/mutation.rs:632` (`:183`, `:538`). Its input row makes one fragment entry
    derive the mode through F10 with `NodeDocumentOf(context)` (`docs/plans/2026-09-ibp-sandbox-input.md:66`); F11
    applies at every production fragment-parse caller (`docs/plans/2026-09-inline-box-predicate.md:1773`).
    `#11-range-full-impl`'s re-eval was extended to 2026-11-01 with N9's plan-memo as venue (`ibp-sandbox.md:763`).
  - C0 (`EH-attrs-at-creation-C0`, Layout, `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` rev 1 on branch
    `layout-dom-c0-insert-adopt-plan`, `fade8a19`, not on main) adds `EcsDom::node_document` (NEW: C0 D-1) and deletes
    `owner_document`'s tree-position fallbacks (D-1, `:110-121`). Its §7 lists "ownerless node creation" as out of
    scope (`:562`), with slot text R-6 (a) `#11-ownerless-node-creation` (`:614-617`), now registered by the PM
    (PM 207). R-6 (c) `#11-selection-document-membership` names `selection.rs:614-619` (informational, §8).
  - PM 157: N4's planned `EcsDom::effective_node_document` (NEW) (in N4's plan, not in the code; `ibp-sandbox.md:228`) is replaced by C0's `node_document` in C0's
    plan-review, so the engine has one node-document reader. PM 207 fixes the Layout order N1 → C0 → C1 → T3 → N9.
  - PR #578 (`vm-host-data-split`, open, PM 203) moves `HostData::split_dom_*` projections, among them
    `split_dom_and_live_ranges` (at `ee6c7a1a`: `crates/script/elidex-js/src/vm/host_data/mod.rs:939`), into
    `host_data/engine_feature/bind.rs`, and leaves `host_data/mod.rs` at 729 lines (`gh pr view 578 -R send/elidex`).

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
   scripting mode is **Fragment**, not Inert (`webref body html fragment-parsing-algorithm-steps`), unless
   contextDocument's scripting is disabled, when HTML §13.4 HTML fragment parsing algorithm step 8 sets it to
   **Disabled** (`webref body html html-fragment-parsing-algorithm`). The scripting mode
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
    `<textarea>` today (§0.2), so it does not mark the clones the CE pass acts on. Rev 2 widened the gate; R2 found
    the widening broken (`ClonedFrom` is in `elidex-dom-api`, which `elidex-custom-elements` cannot depend on; and
    a widened gate makes the form pass copy form state into customized-built-in `button` / `select` / `output` /
    `meter` / `progress` clones, which changes `:disabled` inheritance). Rev 3 withdraws the widening: the marker
    stays the form pass's alone, and the CE pass gets clone roots (D-a′, §4.3).
12. F5's list of § drifts was incomplete. Rev 3 does not complete it: the per-PR sweeps own citation correctness
    (§6 rule).

Rev 3, premises in the R2 dispositions that the code or the memos correct:

13. R1 F6 "N9 owns the Fragment parser mode" (adopted in Rev 2): N9's D4 omits Fragment and leaves its producer
    under `#11-range-full-impl` (§0.2). The R2 dispositions correct this (D-b); Rev 3 applies it.
14. D-a "the created-element list from the pairs worklist equals the set today's shadow-inclusive walk visits, so
    `cloneNode` is unchanged": **fails** at `ee6c7a1a`, on two counts.
    - **Order.** `apply_clone_creation_ce_semantics` returns its candidates in walk order
      (`for_each_shadow_inclusive_descendant`, `crates/core/elidex-ecs/src/dom/tree/walkers.rs:54-80`: pre-order,
      each node's light subtree before its shadow tree), and `enqueue_subtree_upgrade_reactions` pushes them in that
      order (`crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs:142`, `:151-153`), so the order is the
      order the upgrade constructors run. The pairs worklist is not in that order. `clone_children_recursive` appends
      each parent's children as one group (`tree_clone.rs:403-422`), so for `<div><x-a><x-b></x-b></x-a><x-c></x-c></div>` the
      pairs are div, x-a, x-c, x-b, while the walk gives div, x-a, x-b, x-c. Spec order is the walk's: "clone a node"
      creates the copy (step 2), then the children in tree order (step 5), then the shadow children (step 6.7)
      (`webref body dom concept-node-clone`). Shadow pairs are also appended after the whole light tree
      (`node_methods/clone.rs:286-303`). Passing the list as is would change the upgrade order for `cloneNode`.
    - **Depth cap.** The walk skips nodes deeper than `MAX_ANCESTOR_DEPTH` (`walkers.rs:60-62`). The pairs have no
      depth cap. The sets differ for trees deeper than 10,000.
    - The other half holds with one addition: the cloner can exclude template-content clones, but only
      transitively. `replicate_template_contents` pushes the content pairs (`node_methods/clone.rs:426-430`), and pass
      4 then pushes the shadow-tree pairs of any clonable host inside that content (`:286-303`). The walk reaches
      neither (it does not enter template contents), so those must be excluded too.
    - Per the driver's rule, D-a is not applied. The driver re-decided (D-a′, §4.3): clone roots with a scope, and
      today's walk per root, so order and cap are today's within each root. Item 17 records the check of D-a′.
15. R2 Ax3-MIN5 "the 4 re-homed slots": three slots are re-homed (`#11-range-extract-partial-ancestor-clone`,
    `#11-range-comment-extract-clone`, `#11-range-tostring-boundary-text`). With `#11-range-full-impl` the program
    holds four pre-existing slots, and §9 dates all four.
16. Not in the dispositions: the cloner gives shadow children the operation's `subtree` flag
    (`node_methods/clone.rs:288-290`, "Step 6.7: shadow children clone with the operation's own `subtree` flag"), but
    the current spec passes "subtree set to true" (`webref body dom concept-node-clone`, step 6.7). So a shallow
    clone of a clonable shadow host gets a shallowly cloned shadow tree. RC-2's shallow clone of a partially
    contained shadow host reaches this path. It is pre-existing. Driver decision: it is D14 (§0.4), fixed in RC-2.

Rev 3, the check of driver decision D-a′ against the code at `ee6c7a1a`:

17. **D-a′ order, clone mode: matches, except one place.** "Clone the contents" creates, in order: the fragment
    (not an element); the first partially contained child's clone (step 13.1), then its subrange's clones
    (step 13.4); each contained child's deep clone (step 14.1); the last partially contained child's clone, then its
    subrange's (steps 16.1, 16.4); CharacterData clones at steps 4, 12 and 15 are not elements (`webref body dom
    concept-range-clone`). Clone roots listed in that order, recursing into each subrange where the spec does, give
    the spec's order of roots. Within a `Subtree` root, today's walk (`walkers.rs:54-80`: visit the node, then its
    light children's subtrees in order, then its shadow children) is the order of "clone a node": the copy
    (step 2), the children in tree order, each recursively (step 5), then the shadow children (step 6.7). Template
    cloning steps (step 3) create the content clones, which the walk skips, as today (§4.3 condition 4).
    - **The mismatch: a `SelfOnly` root that is a clonable shadow host.** "Clone a node" with subtree false still
      runs step 6: it attaches a shadow root to the copy and clones the source's shadow children into it (with
      subtree true, once D14 is fixed; shallowly today). Those copies are created elements, created right after
      the root and before the subrange's clones. A SelfOnly scope that "visits only the root element" skips them:
      no upgrade reaction and no async-autonomous is-value clear for the shadow-tree clones. Today's walk on that
      root is also wrong: it would visit the root's light children, which are later clone roots (visited twice) in
      clone mode or moved originals (not fresh) in extract mode. **Driver decision D-a″**: `SelfOnly`
      visits the root element, then runs today's shadow-inclusive walk (order and cap unchanged) over the root's
      shadow root if it has one, and never its light children. Every element in that shadow tree was created by
      step 6 of this clone (subtree true for the shadow children, after D14), so all of them are fresh; the light
      children are either later clone roots (listed on their own) or moved originals. Order = spec order (root,
      then step 6.7, then the subrange's roots).
    - **SelfOnly root's own element: correct.** At `ee6c7a1a` the walk visits its root first (`walkers.rs:58-63`,
      depth 0 is under the cap), and `prepare_upgrade` reads only that entity (`upgrade.rs:55-99`), so a
      single-element visit gives that root the same candidacy and is-value clear the walk gives it.
    - **Depth cap.** Each root's walk starts at depth 0, so a cap applies per root: an element deeper than 10,000
      below its own root is skipped, as `cloneNode` skips it today. Root depth inside the fragment does not count.
    - The brief's Subtree case "the fragment-level deep clone" has no counterpart in "clone the contents" or
      "extract": the fragment is created (step 1), never cloned. The one other Subtree root is `cloneNode`'s.

### §0.4 Defect inventory (each silent-wrong row has a negative control in its slice's DoD)

Basis: **predicted** = read from the code at `ee6c7a1a`, not yet run; **measured** = an existing test shows it. Every
predicted row is measured first by its home slice, at the pre-fix head, under the green-branch rule below.

| ID | Defect (observable) | Kind | Basis | Evidence | Home | Slot |
|---|---|---|---|---|---|---|
| D1 | toString, Element start container: includes text of children before `start_offset` | silent-wrong | predicted | §0.2 toString | RC-1 | `#11-range-tostring-boundary-text` |
| D2 | toString, Element end container: drops text of descendants before `end_offset` | silent-wrong | predicted | §0.2 toString | RC-1 | same |
| D3 | toString skips CDATASection text | silent-wrong, Rust-only reachable | predicted | `mutation.rs:53` | RC-1 | same |
| D4 | deleteContents / extractContents, Element end container: contained descendants before `end_offset` are neither removed nor moved | silent-wrong | predicted | `mutation.rs:93-95` | RC-1 (both halves: delete, and extract's interim enumeration) | new: proposed `#11-range-contained-enumeration-end-element` |
| D5 | deleteContents / extractContents / `Selection.deleteFromDocument` leave the range at `(start, startOffset)` instead of `(newNode, newOffset)` | silent-wrong | predicted | `mutation.rs:235-236`, `:399-400`; `selection.rs:537-542` | RC-1 (delete, Selection), RC-2 (extract, rewritten) | new: proposed `#11-range-delete-extract-new-boundary` |
| D12 | `insertNode` evaluates step 13 "if range is collapsed" before the insert; e.g. `r.selectNode(x); r.insertNode(x)` → spec end stays `(P, i+1)`, elidex gives `(P, i)` (step 9's remove collapses the live range; step 13 is skipped) | silent-wrong | predicted | `range_proto_mutation.rs:149`, `:190` | RC-1 | new: proposed `#11-range-insert-node-step13-timing` |
| D13 | Raw-sibling tree access in Range. (a) Boundary-point comparison on a shadow host counts the ShadowRoot as a child: `h.attachShadow({mode:'open'}); h.append(a, b)`; range `(h, 1)`-`(h, 2)` reports `a` as contained (and `isPointInRange(h, 1)` / `comparePoint` / deleteContents on such ranges act on the shifted index). (b) The pre-order walk of toString and `contained_top_level_nodes` enters the ShadowRoot of a host inside the range, so toString can include shadow-tree text and deleteContents / extractContents can act on the shadow tree | silent-wrong | predicted | (a) `range/mod.rs:549-571`; (b) `range/mod.rs:573-584` (§0.2) | RC-1 | new: proposed `#11-range-compare-points-shadow-index` |
| D10 | `insertNode` with a Comment start: no throw; `node` becomes a child of the Comment | silent-wrong | predicted | §0.2 append_child | RC-1 | new: proposed `#11-range-insert-node-validity` |
| D11 | `insertNode` step 6 checks cycles + ShadowRoot only, not the full "ensure pre-insert validity" (e.g. a doctype into an element, a Text into a Document) | silent-wrong | predicted (measured with D10) | `mutation.rs:476-494` | RC-1 | same as D10 |
| D6 | extractContents leaves partially contained ancestors in the tree and flattens their contained descendants into the fragment | silent-wrong | predicted | §0.2 extract | RC-2 | `#11-range-extract-partial-ancestor-clone` |
| D7 | extractContents with a Comment boundary: same-container is a no-op; cross-container skips the Comment's clone and splice | silent-wrong | predicted | `mutation.rs:278`, `:333`, `:373` | RC-2 | `#11-range-comment-extract-clone` |
| D14 | "Clone a node" step 6.7 clones a clonable shadow root's children with the operation's `subtree` flag, not true: `h.attachShadow({mode:'open', clonable:true}).append(x)` with `x` holding a child; `h.cloneNode(false).shadowRoot.firstChild` has no children (spec: a deep copy of `x`). RC-2's shallow clone of a partially contained shadow host hits the same path | silent-wrong | predicted | `node_methods/clone.rs:288-290` (`clone_recording(dom, child, deep, …)`); `webref body dom concept-node-clone` step 6.7 "subtree set to true" | RC-2 (in the canonical cloner, so `cloneNode` and the Range methods get it at once) | new: proposed `#11-clone-shadow-children-subtree` |
| D9 | extractContents does not throw HierarchyRequestError for a contained doctype (step 11); e.g. `r.selectNodeContents(document); r.extractContents()` moves the doctype | silent-wrong | predicted | no doctype check in `mutation.rs:265-402` | RC-2 | new: proposed `#11-range-contents-doctype-check` |
| — | extractContents fragment / Text clones have no node document (R2's D8) | silent-wrong | predicted | `mutation.rs:266`, `:303`; Appendix A | **out of the program**: Layout, facet of `#11-ownerless-node-creation` (PM 207, §8) | R2's proposed `#11-range-contents-node-document` withdrawn |
| — | cloneContents unimplemented | visible (throws) | measured (`tests_range.rs:441-452`) | §0.2 stubs | RC-2 | `#11-range-full-impl` (clone part) |
| — | surroundContents unimplemented | visible | predicted (no stub test: `git grep -n surroundContents -- crates/script/elidex-js/src/vm/tests/tests_range.rs` = 0) | §0.2 (`range_proto_mutation.rs:213-262`) | RC-3 | `#11-range-full-impl` (surround part) |
| — | createContextualFragment unimplemented | visible | measured (`tests_range.rs:454-463`) | §0.2 | RC-4 | `#11-range-full-impl` (contextual part) |

**Classification and ledger handling.** Every row is **pre-existing** at `ee6c7a1a` (none is introduced by this
program), and every proposed ID is **closed by the program's own home slice**. Slot IDs marked "proposed" are for the
PM to register or fold; the memo does not edit the ledger. If the PM folds a proposed ID into the program instead of
registering it, that ID still needs a "Not registered by design" homing row while it appears in memo text
(`memory/reference_slot-homing-checker.md` rule (d)). This also applies to the withdrawn
`#11-range-contents-node-document`, which still appears here. Each proposed slot's fields: defect = the table row;
**Why deferred** = the fix lands in its home slice, in the umbrella's order (§9), not in a separate PR; trigger = the
home slice's per-PR plan-review; re-eval date = **2026-12-01** (proposed), or that slice's landing if earlier.

**The `insertNode` measurement (D10/D11).** RC-1's per-PR plan runs it first, at the pre-RC-1 head, as VM tests:
(a) `r.setStart(comment, 0); r.insertNode(span)` expects HierarchyRequestError and an unchanged tree;
(b) `r.setStart(div, 0); r.insertNode(doctypeClone)` and `r.setStart(document, 0); r.insertNode(text)` expect
HierarchyRequestError. If red, the fix is in **RC-1**, which already rewrites `insertNode` (D12). "Insert a node into
a live range" step 1 (its own HierarchyRequestError check: PI or Comment start, parentless Text start, start is
`node`) runs first, and step 6 calls `ensure_pre_insertion_validity` (`child_node/mod.rs:251`), both before any
mutation (E1). The red tests are RC-1's negative controls. RC-3 consumes the fixed insert unchanged.

**Green-branch rule (every predicted row).** Red → fixed in the home slice; the test is the negative control.
Green (some layer already gives the spec result) → no fix *for that row*; the test is committed as a regression pin,
the PR body records the green run, and the proposed slot is not needed. The slice's structural change lands either
way, because it is the slice's own design, not that row's fix:

| Rows | Structural change that lands even if green |
|---|---|
| D1-D3 | toString enumerates through B1 (`next_in_preorder_global` is deleted) |
| D4, D5 (delete) | deleteContents on B1 and the newNode / newOffset helper |
| D10, D11 | step 6 goes through `ensure_pre_insertion_validity` (one validity check, not a lucky layer); step 1 is checked first |
| D12 | insertNode on `LiveRangeRef`; step 13 evaluated after step 12; the host's `with_range_mut` / `Range::new` carrier is deleted |
| D13 | every Range index, length and walk uses the exposed child list (B3) |
| D5 (extract), D6, D7, D9 | the one contents algorithm (§4.3) |
| D14 | the subtree flag is per pair, not per operation: the step 6.7 clone passes true, and every pair it produces keeps true for passes 3 and 4 (today both take the operation's `deep`: pass 3 at `node_methods/clone.rs:262`, pass 4's child clone at `:290`, so a `<template>` or a nested clonable host inside that shadow tree would still be cloned shallowly); the shallow-branch stamp (`:294-301`) goes with it |

## §1. Shape and verdict

**Edge-dense** (CLAUDE.md "Edge-dense work"): the work intersects boundary-point containment, MutationRecord
emission, live-range updates during the algorithm's own mutations, the clone-a-node pipeline (custom elements, form
state, template contents, shadow roots), and for RC-4 the parser / session path (§2). So: this umbrella, then one
per-PR plan per row below, each through **`/elidex-plan-review` before implementation** (rule, not judgment). A
per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| RC-0a | Pure-move test splits: `range/live.rs`, `selection.rs`, `range/tests.rs`, VM `tests_range.rs` (§7) | — | — | not required (pure move; `git diff --color-moved`) |
| RC-1 | Containment primitives; exposed child list for every Range index, length and walk (D13); live-range reference; toString, deleteContents, Selection.deleteFromDocument, insertNode (steps 1, 6, 13) moved onto them; extract's enumeration moved to the canonical set (interim) | RC-0a; **PR #578** (PM 207: `LiveRangeRef` is written against the post-#578 `engine_feature/` location, §4.2) | `#11-range-tostring-boundary-text`; proposed D4, D5 (delete half), D10, D11, D12, D13 | required |
| RC-2 | One "contents" algorithm for clone and extract; cloneContents; VM post-clone seam shared with cloneNode. Nodes are created with today's canonical owner rule (C3) | RC-1 (not gated on C0) | `#11-range-extract-partial-ancestor-clone`, `#11-range-comment-extract-clone`, `#11-range-full-impl` clone part; proposed D5 (extract half), D9 | required |
| RC-3 | surroundContents | RC-2 | `#11-range-full-impl` surround part | required |
| RC-0b | Pure-move split of `html_fragment.rs` inline tests (§7); lands **before N9** (PM 207, touch-time split: the first toucher splits; N9 rebases onto it) | — | — | not required |
| RC-4 | createContextualFragment through the session fragment-parse path; adds the `Fragment` value to N9's `ParserScriptingMode` and its semantics in both parser backends | RC-0b; **IBP-sandbox N9** (the enum); **C0** (`EcsDom::node_document`); independent of RC-1..3 (§5.4, §9) | `#11-range-full-impl` contextual part (the slot closes when RC-2, RC-3, RC-4 have all landed) | required |

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
- **B3 (units and the child list)**: offsets in CharacterData are UTF-16 code units. Every Range child index, node
  length and tree walk uses the **exposed** child list, which skips the ShadowRoot entity: `compare_points`'
  `child_index_in_ancestor`, `range/mod.rs` `child_index` and `node_length`, and the B1 enumeration, which replaces
  the raw `next_in_preorder_global` walk (D13 (a) and (b), RC-1). So a boundary offset, a node's index, the
  bridge's event index and the nodes in the range all come from the same list, and no Range algorithm reaches into a
  shadow tree through its host. **Cap policy**: Range's own derivations are **uncapped**, through
  `child_list_uncapped` (`navigation.rs:284`), following the existing `insert_node` step-13 precedent
  (`range/mutation.rs:575-580`, Codex PR418 R2: a capped count would set the boundary short). The bridge's event index
  stays `EcsDom::index_in_parent`, which returns `None` past `MAX_ANCESTOR_DEPTH` previous siblings; the tree
  primitives then fire no event (§0.2). Below 10,000 siblings the two agree. Above it, live-range updates are
  skipped, which is a pre-existing wide-DOM gap of the bridge, not of the Range algorithms (PM list).
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
  clone. The CE pass gets the created set as the algorithm's ordered **clone roots**, each with a scope (`Subtree` /
  `SelfOnly`), and runs today's walk per root (driver decision D-a′, §4.3 conditions 1-2). The form pass and the
  `ClonedFrom` marker are today's (`<input>` / `<textarea>` only, §4.3 condition 3); a moved original never carries
  the marker. One VM post-clone seam serves `cloneNode` (one `Subtree` root) and the Range methods.
  Template-content clones are outside the CE pass (rule in §4.3), as today.
- **C3 (node document: not this program's)**: the spec creates the fragment given the start node's node document
  ("clone the contents" / "extract" step 1, `webref body dom concept-range-clone`, `concept-range-extract`), and
  every clone gets its operation's document ("clone a single node" step 8). How each creation site gets its owner is
  the facet of `#11-ownerless-node-creation` owned by Layout (PM 207: owners come from the single entry
  `node_document_for_new_nodes`, C1 rev 2; reads use only C0's `node_document`). The program's rule is: RC-2 creates
  nodes with the owner rule the current canonical paths use (the canonical cloner for clones; today's
  fragment-creation call for the fragment) and adds **no new ownerless creation site** beyond today's. Every Range
  creation site is listed for C0 rev 2 in Appendix A. No Range code reads the node document through any reader other
  than the canonical one of its base: `EcsDom::owner_document` today, C0's `EcsDom::node_document` after C0.
- **E1 (throw before mutating)**: errors the spec raises before any mutation (extract / clone step 11 doctype; surround
  steps 1-2; insert steps 1 and 6) leave the tree, the records and the range untouched. elidex's own refusal of a
  ProcessingInstruction boundary (PI clones are unsupported, `node_methods/clone.rs:126-131`; PI has no producer,
  §0.2) is also raised before any mutation: the contents algorithm checks **both** boundaries up front, never midway
  through the moves.
- **E2 (`[CEReactions]`)**: seven methods are `[CEReactions]`: Range `deleteContents`, `extractContents`,
  `cloneContents`, `insertNode`, `surroundContents` (`webref idl dom Range`), `createContextualFragment` (`webref
  idl html Range`), and `Selection.deleteFromDocument` (`webref idl selection-api Selection`). Reactions enqueued by
  their mutations (disconnected / connected via the mutation-event leg, upgrades via C2) run when the method's
  `[CEReactions]` scope ends, in the order the spec steps enqueue them. The comparison base is that spec step order,
  not `appendChild`. Rule: each slice measures the timing for the methods it owns (the seven are split across RC-1
  to RC-4, §5), and a difference is a defect fixed in that slice.
- **E3 (engine→host error contract)**: an engine algorithm that can throw returns, on error, a `DomApiError {
  kind, message }` (`crates/script/elidex-script-session/src/types.rs:149-154`, the type `CloneNode` already
  returns) **plus the records produced before the error**; `Ok` carries the result and its records.
  `DomApiErrorKind` (`types.rs:195-219`, `#[non_exhaustive]`) gains `InvalidNodeTypeError` (surroundContents step 2).
  The host maps the error through the existing `dom_api_error_to_vm_error` (`vm/host/dom_bridge.rs:390`) and commits
  the records (M2); it decides nothing and adds no second mapper. `RangePointError` (`range/boundary.rs:12-19`, mapped
  separately in `vm/host/range_proto.rs:820-840`): **folded into `DomApiError`** in RC-1 unless RC-1's per-PR plan
  names a caller that needs a distinct shape (one issue, one way). The plan records the outcome.
- **P1 (parser behind the session)**: RC-4 parses only through `elidex-script-session` (no new
  `elidex-dom-api` → `elidex-html-parser` dependency).
- **P2 (Legacy mode)**: allowDeclarativeShadowRoots = false; with #577 a top-level `<template shadowrootmode>` is a plain
  template anyway, and nested ones are plain because the flag is off.
- **P3 (context read-only; no leaked context)**: the live start node or its parent element is never mutated (fragment
  plan I1, `docs/plans/2026-10-fragment-dsd-not-on-context.md` §2); a created `body` context is destroyed before
  return, in every outcome.
- **P4 (result)**: a new DocumentFragment whose node document is the start node's node document, read through
  `EcsDom::node_document` (C0; the one reader, PM 157); no MutationRecord.
- **X1 (layering)**: algorithms in `elidex-dom-api` (`range/`); `vm/host/range_proto_mutation.rs` does brand check,
  argument conversion, live-range-reference lookup, record commit, post-clone seam call, wrapper creation. No loop or
  walker in `vm/host/`.
- **X2 (ECS-native)**: no new component and no new entity-keyed side-store. `LiveRangeRef` is a handle into the
  existing `LiveRangeRegistry` map (Range objects are not entities). Clones use the existing ECS cloner. The clone
  roots are a value the engine returns for one call (not per-entity state, so not a component); the `ClonedFrom`
  marker stays as today, the form pass's only.

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
| X × C | the clone roots cross the engine→host boundary as a returned list of `(Entity, scope)`; the VM seam only iterates it and calls the passes, so it walks nothing itself (X1); the passes' walkers live in `elidex-custom-elements` and `elidex-form` |
| P × M | parsed nodes go into a new fragment: no record (P4) |
| P × L | createContextualFragment mutates nothing that exists, so no live-range update |
| P × C | parsed elements get their custom-element state from the parser's own derivation (as innerHTML), not from C2 |
| X × E | the engine returns `DomApiError` plus the records; the host only maps (through `dom_api_error_to_vm_error`) and commits (E3), so no error decision sits in `vm/host/` |
| X × P | RC-4's parse goes through `elidex-script-session` (P1); the `Fragment` mode value lives in the parser crates with N9's enum, and the VM native only marshals |
| E × P | createContextualFragment mutates nothing that exists, so any error leaves no records (P4). Its possible throws are a Trusted Types failure (step 2; no Trusted Types yet, §8) and the XML fragment parse of the fragment parsing algorithm steps' XML branch (unreachable while elidex has no XML parsing, §5.4 (b)); the HTML fragment parse does not throw |
| X × L | `LiveRangeRef` is obtained by the VM through HostData's existing split accessor (post-#578 location, §4.2) and passed to the engine; the engine reads and writes the registered range, the host never writes a boundary |

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
| WHATWG DOM §4.4 Interface Node | clone a node (step 3 "Run any cloning steps"); clone a single node (step 8 node document) | shallow / deep; shadow host; template; custom element | `clone_node_with_shadow_honor` (`node_methods/clone.rs:227`); step 6.7 subtree true (D14, RC-2); clone roots for the CE pass (§4.3, D-a′); node document of clones: Layout (C3) | ✓ | yes |
| WHATWG HTML §4.10.5 The input element / §4.10.11 The textarea element | cloning steps (reached via DOM §4.4 clone a node step 3) | input / textarea | `ClonedFrom` + `elidex_form::apply_clone_form_state` (input changes to marked entities, RC-2) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | cloning steps (contents cloned with the copy's template contents' node document) | subtree false / true | `replicate_template_contents` (unchanged); outside the CE pass (§4.3) | ✓ | yes |
| WHATWG HTML §4.13.3 Core concepts | look up a custom element definition (step 1: null registry → null) | — | CE pass scope over template contents (§4.3 condition 4) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | a document's custom element registry (null unless stated otherwise); effective global custom element registry (null for a non-global registry) | inert template document | CE pass scope over template contents (§4.3 condition 4) | ✓ | no |
| WHATWG DOM §4.9 Interface Element | create an element | `body` context for RC-4 | element creation helper (RC-4) | ✓ | no |
| WHATWG DOM §4.10 Interface CharacterData | replace data; substring data | Text / Comment / CDATASection | `apply_replace_data` (unchanged); substring in the contents algorithm | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | split a Text node | insert step 7 | `split_text_at_offset` (unchanged) | ✓ | yes |
| WHATWG HTML §8.5.7 The createContextualFragment() method | steps 1-7 | Element / Text-Comment with parent element / null / `html` in an HTML document | `create_contextual_fragment` (RC-4) | ✓ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | fragment parsing algorithm steps | Legacy mode; runScripts; XML document | session fragment parse (RC-4) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm | context = target; fragment node document; scripting mode | `parse_fragment_progressive` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm step 8 (Disabled when contextDocument's scripting is disabled) | Fragment / Disabled | N9's fragment entry derives Disabled; RC-4 passes Fragment (§5.4) | ✓ | no |
| WHATWG HTML §13.2.4.5 Other parsing state flags | parser scripting mode | Normal / Disabled / Inert / Fragment; Normal is unreachable on the fragment path (HTML fragment parsing algorithm step 1 asserts Inert or Fragment, and step 8 can only set Disabled) | N9's `ParserScriptingMode` (Normal, Disabled, Inert; F11); RC-4 adds `Fragment` | ✓ | no |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | script start tag steps 2-3 (Fragment: no parser document; Inert: already started = true) | Inert / Fragment | RC-4, in both parser backends (`elidex-html-parser-strict`, `elidex-html-parser`); RC-4 tests | ✓ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | create an element for a token steps 13, 15 (fragment parse: willExecuteScript false → DOM §4.9 create an element enqueues the upgrade) | definition null / non-null | question (d), §5.4 | ✗ | no |
| WHATWG HTML §4.12.1.1 Processing model | already started | Fragment mode leaves it unset | question (a), §5.4 | ✗ | no |
| Selection API §3 Selection interface | deleteFromDocument | rangeCount 0 / 1 | `Selection::delete_from_document` (`selection.rs:513`, RC-1) | ✗ (the step text is unconfirmed: `webref body selection-api dom-selection-deletefromdocument` finds no anchor, so the branch list is from the IDL and the code, not the steps) | yes |

Numbers and titles: `webref heading dom 5.2`, `5.5`, `4.2.3`, `4.3.2` (via `coverage-map`), `4.4`, `4.5`, `4.9`, `4.10`,
`4.11`; `webref heading html 8.5.4`, `8.5.7`, `13.4`, `4.10.5`, `4.10.11`, `4.12.3`, `4.13.3`, `13.2.6.1`,
`13.2.6.4.4`, `4.12.1.1`; `webref heading selection-api 3` (§3 Selection interface; `heading selection-api 3.1` /
`3.2` / `3.3` → "no headings under §3.x", so sub-numbers are unconfirmed and not cited; `body selection-api
dom-selection-deletefromdocument` → anchor not in the rendered chapter, so its step text is unconfirmed); `webref coverage-map dom 5.5 dom 5.2 dom 4.2.3 dom 4.3.2 dom 4.4
dom 4.9 dom 4.10 dom 4.11 html 8.5.7 html 8.5.4 html 13.4 html 13.2.4.5 html 4.12.1.1`. Dfns: `webref dfn dom
contained` / `'partially contained'` / `'get the common ancestor'` (§5.5), `'clone a node'` / `'clone a single node'`
(§4.4), `'replace data'` / `'substring data'` (§4.10), `'split a Text node'` (§4.11), `'ensure pre-insert validity'` /
`'replace all'` / `remove` / `append` (§4.2.3), `select` (§5.5), `'create an element'` (§4.9), `'custom element
registry'` (for=Document, §4.5), `'effective global custom element registry'` (§4.5); `webref dfn html
'fragment parsing algorithm steps'` (§8.5.4), `'parser scripting mode'` (§13.2.4.5), `'already started'`
(§4.12.1.1), `'create an element for a token'` (§13.2.6.1), `'look up a custom element definition'` (§4.13.3),
`'appropriate template contents owner document'` (§4.12.3); `webref dfn dom 'cloning steps'` (§4.4); `webref dfn
selection-api deleteFromDocument` (§3). Prose: `webref body dom concept-range-extract`, `concept-range-clone`, `dom-range-deletecontents`,
`concept-range-insert` (also holds surroundContents), `dom-range-stringifier`, `contained`, `concept-range-select`,
`get-the-common-ancestor`; `webref body html dom-range-createcontextualfragment`, `fragment-parsing-algorithm-steps`,
`html-fragment-parsing-algorithm` (step 8), `parser-scripting-mode`, `parsing-main-inhead` (script start tag
steps 2-3), `create-an-element-for-the-token` (steps 13, 15), `the-template-element` (cloning steps), `look-up-a-custom-element-definition`;
`webref body dom concept-node-clone` (clone a node steps 2, 3, 5, 6.7; clone a single node step 8),
`document-custom-element-registry`, `effective-global-custom-element-registry`; `webref body html
appropriate-template-contents-owner-document` (2026-11-01). Breadth: K = 3
(DOM, HTML, Selection API), M = 28. M ≥ 20 is the split signal, and this memo is the split: each per-PR plan carries only its own rows.

Step numbers are cited where `webref body` shows them today; per-PR plans and code comments cite steps **by name**
where the number has moved before (extract's partially-contained-child steps, §0.3 item 1).

## §4. Architecture

### §4.1 Layering map

| Concern | Home | Note |
|---|---|---|
| Containment, toString, delete, contents (clone/extract), insert, surround, context resolution | `elidex-dom-api` `range/` | engine-independent |
| Live-range reference | `elidex-dom-api` `range/live.rs` | existing registry, new handle |
| Record-producing primitives, fragment parse into a new fragment | `elidex-script-session` `mutation/` (`apply_*`); `split_text_at_offset` is in `elidex-dom-api` (`char_data/split_text.rs:105`) | existing primitives; `html_fragment.rs` gains one function in RC-4 |
| `ParserScriptingMode::Fragment` (NEW) (the enum itself is N9's) and its script start tag semantics | `elidex-html-parser-strict` (N9's enum, F11) and `elidex-html-parser` (html5ever backend) | RC-4 (§5.4) |
| Clone-a-node | `elidex-dom-api` `node_methods/clone.rs` | the canonical cloner; pass 2 keeps today's `ClonedFrom` gate (`<input>` / `<textarea>`); D14 is fixed here (RC-2). The node document of clones is Layout's (C3) |
| Created-set entry | `elidex-dom-api`: `CloneNode` returns its clone (one `Subtree` root); the Range contents entry returns the fragment and its clone roots in creation order (§4.3) | the VM seam iterates the roots it receives; it computes nothing (X1, X × C) |
| CE clone-creation pass; form-state pass | `elidex-custom-elements` (root + scope, needs only `Entity`), `elidex-form` (today's) | each walks in its own crate (RC-2, §4.3) |
| Error contract | `elidex-dom-api` engine returns `DomApiError` + records produced before the error (E3) | the host maps through `dom_api_error_to_vm_error` and commits only |
| Brand check, argument conversion, live-range-reference lookup, error mapping, record commit, post-clone seam call, wrapper | `vm/host/range_proto_mutation.rs` | marshalling only (X1); no `with_range_mut`, no transient `Range::new` carrier (RC-1 DoD) |

### §4.2 Live-range reference (RC-1)

`LiveRangeRegistry` already shares its map with the bridge through `Arc<Mutex<HashMap<RangeId, Range>>>`
(`range/live.rs:336-338`). RC-1 adds an owned handle, `LiveRangeRef` (NEW) = (that `Arc`, `RangeId`), obtained from the
registry, with a read (applying the dangling-collapse rule of `finalize_pending` to its own entry) and a boundary write.
The engine methods take `(&LiveRangeRef, &mut EcsDom)`. The VM obtains the handle while borrowing HostData, releases
that borrow, then passes `host.dom()`, so no new HostData split accessor is needed. RC-1 lands after PR #578 (PM
207) and is written against its layout: the `split_dom_*` projections, among them `split_dom_and_live_ranges`
(`host_data/mod.rs:939` at `ee6c7a1a`), move to `host_data/engine_feature/bind.rs`, and `host_data/mod.rs` drops to
729 lines (§0.2). RC-1 edits neither file; if its per-PR plan finds it must, it edits the post-#578 file. `delete_from_document` (`selection.rs:513`) takes the same path. `commit_range_after_mutation`
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

**Created set = clone roots (driver decision D-a′; replaces the R2 `ClonedFrom` widening and D-a).** Two passes
follow the clone, and they act on different sets, so they stay two mechanisms. The **form pass** implements the HTML
cloning steps of `<input>` / `<textarea>` (§3), so it acts only on those clones, found by today's `ClonedFrom`
marker. The **CE pass** implements the custom-element half of "create an element" for every element created by
cloning ("clone a single node" step 2.4, `webref body dom concept-node-clone`), so it needs every created element,
in creation order, and nothing else. Conditions, each a requirement on RC-2:

1. **Clone roots.** The contents algorithm returns, besides the fragment and the records, an ordered list of clone
   roots in creation order (the order of §0.3 item 17). Each root has a scope: `Subtree` for a contained child's
   deep clone (clone mode, step 14.1); `SelfOnly` for the clone of a partially contained element child (clone steps
   13.1 and 16.1, and the matching steps of `concept-range-extract`). A SelfOnly clone's later descendants are moved originals (extract, never listed) or the clone
   roots of its recursive subrange (each listed on its own). CharacterData clones are not elements and are not
   listed. `cloneNode` passes one `Subtree` root, its clone, so its CE behaviour is identical to today: the same
   walk, the same order, the same cap, over the same root.
2. **CE pass per root.** `apply_clone_creation_ce_semantics` takes a root and a scope:
   `apply_clone_creation_ce_semantics(dom, registry, root, scope: CloneScope) -> Vec<Entity>`, with
   `CloneScope { Subtree, SelfOnly }` defined in `elidex-custom-elements` (it needs only `Entity`;
   `elidex-dom-api` already depends on that crate for `CEState`, so the Range engine can name it, and no crate
   cycle forms). `Subtree` runs today's `for_each_shadow_inclusive_descendant` walk, unchanged in order and cap.
   `SelfOnly` visits the root element, then today's walk over the root's shadow root if it has one, never its
   light children (D-a″, §0.3 item 17). A scope argument is chosen over a sibling function so that both visit sets
   share one walk in one function. Its contract "MUST only be called on a freshly cloned subtree" (`upgrade.rs:121-123`) becomes "on a
   root this call created, with the scope the cloner gave it". The VM seam calls it once per root, in list order,
   and appends the candidates to the reaction queue in that order (`custom_elements/upgrade.rs:142`, `:151-153`).
3. **Form pass and marker: today's.** Pass 2 (`mark_cloned_form_control`, `node_methods/clone.rs:368-387`) attaches
   `ClonedFrom` only to `<input>` / `<textarea>` clones, "so a large `cloneNode(true)` does not spray a marker onto
   every cloned node" (`:373-374`). `elidex_form::apply_clone_form_state` reads and removes every marker over its
   root's template-inclusive descendants, unconditionally (`elidex-form/src/clone.rs:44-63`); a left-behind marker is
   harmless to it (`node_methods/clone.rs:60-64`), and every production call of the canonical cloner is followed by
   it (today `CloneNode` only, §0.2). For the Range methods the seam calls it once on the fragment: every clone of
   the call is under the fragment, and extract's moved originals never carry the marker, so the form pass is already
   correct under extract. R2 Ax3-MIN9 (a census ratchet for the widened marker) is moot for the CE pass, which reads
   no marker; the remaining form-marker contract is today's.
4. **CE-pass scope over template contents.** The CE pass visits each `Subtree` root's shadow-inclusive
   descendants and **not** template-content clones (today's walk, `for_each_shadow_inclusive_descendant`,
   `upgrade.rs:131`, does not reach template contents). Spec basis: template cloning steps step 2 clone the
   contents "with document set to copy's template contents's node document" (`webref body html
   the-template-element`), the inert template document "whose browsing context is null" (appropriate template
   contents owner document step 1.1.1); look up a custom element definition step 1 returns null for a null
   registry (`webref body html look-up-a-custom-element-definition`). The inert document's registry **is null**:
   "creating a document" sets none, and a document's custom element registry is null unless stated otherwise (DOM
   §4.5, `webref body dom document-custom-element-registry`). The clone-a-single-node registry steps 2.1-2.3
   (`webref body dom concept-node-clone`) then give the content clone a null registry. A null registry stays null,
   and a global one becomes the inert document's registry's effective global custom element registry, which is null
   because that registry is null and so not global (DOM §4.5, `webref body dom
   effective-global-custom-element-registry`). The rule keeps `cloneNode` behaviour unchanged; a test pins it
   (a cloned `<template>` whose contents hold a defined custom element enqueues no upgrade for the content clone).
   C1 (`#11-template-contents-owner-document`) changes those contents' node document; the rule does not depend on it.

The seam (NEW, RC-2) is one VM function used by `native_node_clone_node` and the Range natives: it receives the
clone roots and the form-pass root the engine returned (for `cloneNode`: one `Subtree` root, which is also the
form-pass root), runs the CE pass per root in order (condition 2), then the form pass once (condition 3). This is
today's shim order (`node_methods_extras.rs:221`, `:230`), now inside the one seam. `apply_clone_form_state` is
unchanged.

deleteContents stays its own algorithm (the spec's is separate and clones nothing); it shares B1 and the newNode /
newOffset helper with extract.

### §4.4 ECS-native check

| Spec / OO notion | elidex form |
|---|---|
| live range object updated by mutation observers inside the engine | existing `LiveRangeRegistry` map + `LiveRangeBridge` driven by `MutationDispatcher` events; `LiveRangeRef` is a handle, not a new registry |
| "clone a node" with per-class cloning steps | ECS cloner + post-pass worklist over (src, dst) pairs (existing); the form cloning steps keep the existing `ClonedFrom` marker on `<input>` / `<textarea>` clones; the CE pass gets the call's clone roots as a returned value (one call's output, not per-entity state) |
| subrange objects | plain values on the stack (L2) |
| `[CEReactions]` element queues | existing reaction queue fed by the mutation-event leg and C2 |

No component, system or side-store is added. The existing `ClonedFrom` marker is unchanged (§4.3 condition 3).

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
  iteration, newNode / newOffset. Exposed child list (D13, B3): `child_index_in_ancestor`, `child_index` and
  `node_length` move onto the uncapped exposed list (`child_list_uncapped`), and the B1 enumeration replaces the raw
  `next_in_preorder_global` walk. `LiveRangeRef` (§4.2). toString rewritten on B1 (D1-D3). deleteContents on B1 +
  newNode / newOffset written at step 8 (D4, D5). insertNode on `LiveRangeRef`: step 1 (its own HierarchyRequestError
  check) and step 6 (`ensure_pre_insertion_validity`) before any mutation (D10, D11, per the §0.4 rule), step 13
  evaluated after step 12 (D12). extractContents' flat moves switch from `contained_top_level_nodes` to B1's
  nodesToRemove set (interim; RC-2 replaces the body), so `contained_top_level_nodes` and `next_in_preorder_global`
  are deleted. `delete_from_document` (`selection.rs:513`) on `LiveRangeRef`. Error contract E3 for insertNode
  (`DomApiErrorKind` gains nothing here; HierarchyRequestError exists), and the `RangePointError` fold (E3).
- **Order**: after RC-0a and PR #578 (§4.2).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`, `crates/dom/elidex-dom-api/src/range/live.rs`, `crates/dom/elidex-dom-api/src/range/containment.rs` (NEW),
  `crates/dom/elidex-dom-api/src/selection.rs`, `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto.rs` (toString native only if its signature changes), tests.
- **Tests**: unit: toString with Element start (children before `start_offset` excluded) and Element end
  (descendants before `end_offset` included), CDATASection (Rust-constructed); delete with Element end container
  (`<div><p>ab</p><span>X Y</span></div>`, range `(ab, 1)`-`(span, 1)`: X removed); **extract with the same Element
  end container** (X moved into the fragment; D4's extract half, cross-checked against RC-2's rewritten extract,
  which must keep the test green); delete final boundary when start is not an inclusive ancestor of end; another
  live range at the deletion point; D13: `h.attachShadow({mode:'open'}); h.append(a, b)`, range `(h, 1)`-`(h, 2)`:
  `a` not contained, `comparePoint(h, 1)` / `isPointInRange` on the exposed index, deleteContents removes `b` only;
  D13 (b): a range spanning a shadow host (`(parent, 0)`-`(parent, 1)` around `h`, with text in `h`'s shadow tree):
  toString omits the shadow text, and deleteContents / extractContents leave the shadow tree, and the `ShadowRoot`
  link, untouched (negative control).
  VM: the same through JS; `Selection.deleteFromDocument` final boundary; `r.selectNode(x); r.insertNode(x)` end
  boundary; the D10/D11 cases of §0.4. MO: record order unchanged for existing `range_ops.rs` cases; new D4 cases
  emit the removal (delete) and move (extract) records for X.
- **DoD**: negative controls for D1, D2, D3, D4 (delete **and** extract), D5 (delete + Selection), D10, D11, D12,
  D13, each red-first or under its green-branch rule (§0.4), the outcome recorded in the PR body;
  `git grep -n 'contained_top_level_nodes\|next_in_preorder_global\|commit_range_after_mutation' -- crates`
  returns nothing; step 13 is not left in the host: `git grep -n 'with_range_mut\|Range::new' --
  crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` returns nothing (today 4 hits: `:50`, `:75`, `:158`,
  `:193`); `git grep -n 'child_index_in_ancestor\|dom\.get_first_child\|dom\.get_next_sibling' --
  crates/dom/elidex-dom-api/src/range` returns nothing (raw sibling access gone from Range; today 4 hits,
  `range/mod.rs:558`, `:564`, `:574`, `:579`); E2 timing measured for
  deleteContents, insertNode, `Selection.deleteFromDocument`; RC-1 hygiene rows of §6 done.

### §5.2 RC-2 — one contents algorithm for clone and extract

- **Order**: after RC-1. Not gated on C0: the node document of the nodes it creates is Layout's (C3, §8), so the
  silent-wrong fixes D5 (extract), D6, D7 and D9 do not wait for C0 or the P1 switch-back (PM 201 / 202).
- **Scope**: §4.3. cloneContents implemented; extractContents replaced by the same function. Same-container and
  partial CharacterData for Text, Comment and CDATASection via `get_char_data` and canonical clone + set data (D7);
  partially contained element children cloned shallowly with recursive subranges (D6); doctype throw at step 11
  (D9, `HierarchyRequestError`, before any mutation); PI boundary refused up front (E1); every clone through the
  canonical cloner, and the fragment through today's creation call, so the owner rule is the current canonical one
  and no ownerless creation site is added (C3; the sites are in Appendix A); VM post-clone seam shared with
  `cloneNode`, with the clone roots of §4.3 (D-a′); D14 fixed in the canonical cloner (§0.4). ProcessingInstruction has no producer (§0.2,
  unreachable from script); `clone.rs:126` stays an unslotted stub here and gets a proposed slot (§8). The Attribute
  stub (`node_methods/clone.rs:120-125`) is in `CloneNode`'s `invoke` (`node_methods/clone.rs:91`), which RC-2 does not plan to edit (the contents
  algorithm calls `clone_node_with_shadow_honor`, and a range never holds an Attr). The per-PR plan checks this by
  its diff: if RC-2 edits that `match`, the Attribute clone is folded into RC-2; otherwise it is the proposed
  `#11-attr-clone-node` (§8).
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW), `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`,
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs`, `crates/dom/elidex-custom-elements/src/upgrade.rs`,
  `crates/core/elidex-ecs/src/dom/tree_clone.rs` (doc row only),
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, `crates/script/elidex-js/src/vm/host/node_methods_extras.rs`, `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs`,
  tests (`crates/dom/elidex-form/src/clone.rs` unchanged unless the per-PR plan finds otherwise).
- **Tests**: unit: extract and clone over `<p>a<b>bc<i>d</i>e</b>f</p>` with start inside `bc` and end inside `f`
  (fragment = `c`-text, shallow `<b>` holding `<i>d</i>` and `e`, then `f`-head); same for clone with the tree
  unchanged; Comment same-container and cross-container boundaries; doctype throw with tree unchanged; clone of a
  clonable shadow host and of a `<template>` (structure only: node documents are not asserted, they are Layout's,
  §8); RC-1's D4 extract case still green. VM: a custom element under a
  partially contained `<b>` keeps its identity and is not re-upgraded after extract (C2); a cloned custom element
  gets its upgrade reaction (C1); cloneContents over two contained custom-element subtrees enqueues upgrades in
  the spec's creation order (§0.3 item 17); no upgrade for a template-content clone (§4.3 condition 4); D14:
  `host.cloneNode(false)` on a host with a clonable shadow root whose child has a child: the clone's shadow tree
  holds a deep copy (negative control; same for a `<template>` with contents inside that shadow tree); `<input>` live value
  copied into a cloned contained subtree. MO: extract records in spec order (start replace data, recursive subrange
  removals on the partial child, contained moves, end replace data); clone produces none.
- **DoD**: negative controls for D6, D7, D9, D5 (extract), D14, each red-first or under the green-branch rule (§0.4); the
  cloneContents stub test (`tests_range.rs:441-452`) replaced by behaviour tests; no new ownerless creation site:
  `git grep -n -E 'create_(text|document_fragment)[a-z_]*\(' -- crates/dom/elidex-dom-api/src/range ':!*tests*'`
  lists only the fragment creation (today, besides `range/live.rs`'s inline tests that RC-0a moves to a test file,
  4 hits: `range/mutation.rs:266`, `:303`, `:354`, `:392`; the three Text clones
  move to the canonical cloner), and Appendix A is re-run at RC-2's head and sent to Layout; one post-clone seam (`git grep -n
  'enqueue_subtree_upgrade_reactions\|apply_clone_form_state' -- crates/script/elidex-js/src/vm/host` shows one
  call site each, inside the seam); the production-cloner census of §4.3 condition 3 re-run (today's form-marker contract) (`git grep -n
  'clone_node_with_shadow_honor\|"cloneNode"' -- crates`), every caller followed by the seam; E2 timing measured for
  extractContents and cloneContents; RC-2 hygiene rows of §6 done.

### §5.3 RC-3 — surroundContents

- **Scope**: surroundContents steps 1-7 on `LiveRangeRef`: step 1 (non-Text partially contained → InvalidStateError)
  and step 2 (Document / DocumentType / DocumentFragment → InvalidNodeTypeError; `DomApiErrorKind` gains the variant,
  E3) before any mutation; step 3 = RC-2's extract; step 4 replace all through the session primitive; step 5 =
  RC-1's insert, unchanged; step 6 append; step 7 select (`range/boundary.rs:125`). Late throws (insert step 1 when
  the post-extract start is a Comment; append into a CharacterData `newParent`, the spec's "historical reasons"
  note) leave the mutations done and return their records with the error (E3), which the host commits (M2).
- **Seam**: RC-3 needs no post-clone seam call. Surround's extract creates CharacterData clones only: step 1 rejects
  any non-Text partially contained node, so a partially contained node is a Text; the same-container case clones the
  one CharacterData node; contained children are moved, not cloned. Neither pass acts on a non-element (the CE pass
  needs `CustomElementState`, the form pass an `<input>` / `<textarea>` source), so there is nothing for the seam to
  do.
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW in RC-2), `crates/dom/elidex-dom-api/src/range/mutation.rs`,
  `crates/script/elidex-script-session/src/types.rs` (one enum variant), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each error branch with tree / range / records checked; surround with `newParent` already having
  children (replace-all records); surround around a Text-only partial selection; range after surround equals
  "select newParent"; late-throw cases with their records delivered.
- **DoD**: the surroundContents stub removed; E2 timing measured for surroundContents; RC-3 hygiene rows of §6 done.

### §5.4 RC-4 — createContextualFragment

- **Order** (PM 207): N9 lands the `ParserScriptingMode` enum (F11: `Normal | Disabled | Inert`, `ibp-sandbox.md:510`),
  then RC-4 adds the `Fragment` value. The Layout order is N1 → C0 → C1 → T3 → N9, so C0 (and C1) have landed
  before RC-4. **If C0 is late, tell the PM before starting RC-4.** RC-0b lands before N9 (§7).
- **Scope**: `create_contextual_fragment` (dom-api, `range/mutation.rs:637`) resolves the context per HTML §8.5.7
  steps 3-6 (Element; Text / Comment → parent element; else null; null or HTML-namespace `html` in an HTML document
  → a new `body` created in the start node's node document), then calls the session's fragment entry (N9's one
  fragment entry, which derives Disabled through F10 with `NodeDocumentOf(context)`,
  `docs/plans/2026-09-ibp-sandbox-input.md:66`) with scripting mode `Fragment` and fragment parser mode Legacy (no
  declarative shadow roots), and places the roots into a new DocumentFragment (P4). **Node document**: RC-4 reads it
  only through `EcsDom::node_document` (C0). That is the engine's one reader (PM 157: N4's `effective_node_document`
  is replaced by `node_document` in C0's plan-review), so RC-4 adds none. RC-4's two creation sites (the `body`
  context and the result fragment) take their owner from the single creation entry Layout provides
  (`node_document_for_new_nodes`, C1 rev 2, PM 207). They are listed in Appendix A. The created `body` is destroyed
  before return on every path (P3). Trusted Types: elidex has none (innerHTML passes the string through,
  `dom_inner_html.rs:146`), so step 2 is identity until it exists; the gap is program-wide, not this program's (§8
  proposed slot).
- **The `Fragment` value** (N9's D4 leaves it to this slot, §0.2): RC-4 adds it to `ParserScriptingMode` and gives it
  its semantics in **both** parser backends. HTML §13.2.6.4.4 "in head" script start tag step 2: in Fragment mode the
  script element gets no parser document; step 3: only Inert sets "already started" (`webref body html
  parsing-main-inhead`). Backends: `elidex-html-parser-strict` (N9's enum) and the html5ever-backed
  `elidex-html-parser` (`crates/dom/elidex-html-parser/Cargo.toml:19`, which today maps only `scripting_disabled`,
  `src/lib.rs:114-117`).
- **Overlap with the fragment plan** (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574; PR 1 landed as #577
  `ee6c7a1a`): RC-4 relies on its I1 (the parse never mutates the context) and I2 (a top-level declarative template is
  plain); with Legacy mode the flag is off, so templates are plain at every depth (P2). The context rules differ from
  innerHTML's (target, or the shadow host) and outerHTML's (parent; DocumentFragment parent falls back to "div",
  `html_fragment.rs:138-142`, `:156-162`): RC-4's context is always an Element (the created `body` covers null), so no fallback is
  used. No change to the fragment plan's code.
- **Questions the per-PR plan must answer first (scout)**:
  (a) **Script run**: does elidex run "prepare the script element" (HTML §4.12.1.1) when a fragment-parsed
  `<script>` is later inserted? If no,
  createContextualFragment's scripts never run where the spec runs them; conditional slot
  `#11-fragment-script-run-on-insert` (proposed, §8).
  (b) Does elidex have XML documents (fragment parsing algorithm steps step 1)? If not, the branch is unreachable and
  stated so. Its owner is `#11-domparser-xml-real-parsing` (the DOMParser native HTML-parses every XML MIME type
  today, `crates/script/elidex-js/src/vm/host/dom_parser.rs:253-262`).
  (c) **Scripting state**: consume N9's F10 `NodeDocumentOf(context)` derivation (HTML §13.4 HTML fragment parsing
  algorithm step 8: "If contextDocument's scripting is disabled, then set scriptingMode to Disabled"); RC-4 adds no
  `scripting_disabled` argument (today `dom_inner_html.rs:421`, `:476` pass `false`; N9 drops the field).
  (e) **html5ever**: can the html5ever backend express Fragment mode (no parser document for the script, already
  started left unset)? If it cannot, the per-PR plan names the defect (what a tolerant-path fragment parse does
  differently from the spec), and the PM decides its slot. RC-4 does not hide it.
  (d) Are custom-element upgrade reactions for parsed elements enqueued at element creation (HTML §13.2.6.1 "create
  an element for a token" step 13: willExecuteScript is false for a fragment parse; step 15 → DOM §4.9 "create an
  element" enqueues the upgrade reaction) or only at insertion, in innerHTML today? RC-4 mirrors innerHTML; a
  difference is a parser-path defect, conditional slot `#11-fragment-parse-ce-upgrade-timing` (proposed, §8).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs` (or its successor module), `crates/script/elidex-script-session/src/mutation/html_fragment.rs`
  (+ re-export in `mutation/mod.rs`, 994 lines at `ee6c7a1a`; re-measured at the per-PR plan, §7),
  `crates/dom/elidex-html-parser-strict/` (the enum value and the script start tag steps, in the file N9 puts the enum
  in), `crates/dom/elidex-html-parser/` (the html5ever mapping), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests.
- **Tests**: each context branch (Element; Text and Comment with parent element; Text under a DocumentFragment → body;
  Document start → body; `html` element → body; SVG-namespace `html`-named element is not replaced); `<td>` parsed with
  a `<tr>` context vs body context; nested `<template shadowrootmode>` stays plain; fragment `ownerDocument`; context
  unchanged and no leaked `body` (`world().len()` before and after); no MutationRecord; a parsed `<script>` is not
  "already started" and has no parser document (Fragment mode), in each parser backend.
- **DoD**: the createContextualFragment stub test (`tests_range.rs:454-463`) replaced; questions (a)-(e) answered in the
  per-PR plan; E2 timing measured for createContextualFragment; RC-4 hygiene rows of §6 done; `#11-range-full-impl`
  closed when RC-2, RC-3 and RC-4 have landed.

## §6. Hygiene, by owning slice

Non-citation hygiene only. Citation correctness is the per-PR rule below; the umbrella keeps no per-line citation list.

| Site | Fix | Slice |
|---|---|---|
| `range/mutation.rs:598` stale slot `#11-range-clone-and-surround-contents` (closed, merged into `#11-range-full-impl`) | deleted with the stub | RC-2 |
| `range/mutation.rs:595` "`clone_node_deep` already exists on EcsDom" | deleted with the stub | RC-2 |
| `range/mutation.rs:632` stale slot `#11-range-create-contextual-fragment` | N9 lands first (PM 207) and re-points this line (`docs/plans/2026-09-ibp-sandbox.md:183`, `:538`); RC-4 deletes the re-pointed text with the stub | N9, then RC-4 |
| `range/mutation.rs:622-623`, `:633` "`is_html_namespace` forward-stub" | deleted; implemented in #256 (§0.2) | RC-4 |
| `range/mutation.rs:252-263`, `:276-277` deferral notes for `#11-range-comment-extract-clone` and the B1.2d-ii §11 carve | deleted with the old body | RC-2 |
| `range/mutation.rs:142-146` delete doc "partial-element-ancestor deep-clone … B1.2d-ii plan §11 carve" | deleted | RC-1 |
| `range_proto_mutation.rs:1-16` module doc ("3 stubs … NotSupportedError pending `#11-range-full-impl`") | rewritten as each stub goes | RC-2 (clone), RC-3 (surround), RC-4 (contextual) |
| `range_proto_mutation.rs:210-212`, `:226-228`, `:247-249` stub docs citing `#11-range-full-impl` | deleted with each stub | RC-2 / RC-3 / RC-4 |
| `node_methods/clone.rs:58-64` "The sole production caller is the VM `cloneNode` shim" | rewritten to name the one post-clone seam; the marker contract itself is today's (§4.3 condition 3) | RC-2 |
| `node_methods/clone.rs:288-289` "shadow children clone with the operation's own `subtree` flag" | rewritten with the D14 fix (step 6.7: subtree true) | RC-2 |
| `selection.rs:690` `#[allow(dead_code)]` on the test helper `elem` ("kept for future cross-doc / no-association tests") | connect it (a no-association test RC-1 needs anyway) or delete it | RC-1 |

**Citation rule (each per-PR plan, on the files it edits).** Every § number, § ↔ title pair and step number in those
files is fixed or verified in that PR. The method is the 4-pattern sweep:

1. Establish the truth first: `webref dfn` / `heading` / `body` for each concept the files cite.
2. Grep with the spec-name prefix (e.g. `git grep -n -P 'DOM §4\.10(?![.\d])' -- <files>`).
3. Grep without it (`'§4\.10(?![.\d])'`).
4. Grep by § number for every section the files cite, and probe for sections that do not exist (e.g. `webref heading dom
   5.10` → none). For step numbers, run `git grep -n -E 'step [0-9]+(\.[0-9]+)?' -- <file>` and compare with `webref body`.

The per-PR plan records each pattern's hit count and the disposition of every hit. A number webref cannot confirm is
replaced by the member or algorithm name, not kept. Step numbers that have moved before (extract's
partially-contained-child steps, §0.3 item 1) are cited by name.

## §7. 1000-line audit (CLAUDE.md "1000-line debt")

Measured with `wc -l` at `ee6c7a1a`:

| File | Lines | Touched by | Action |
|---|---|---|---|
| `crates/dom/elidex-dom-api/src/range/live.rs` | 1021 | RC-1 (`LiveRangeRef`) | **RC-0a**: inline tests (`:498-1021`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/selection.rs` | 1036 | RC-1 | **RC-0a**: inline tests (`:676-1036`) to a sibling test file |
| `crates/dom/elidex-dom-api/src/range/tests.rs` | 806 | RC-1, RC-2, RC-3 | **RC-0a** (projected past 1000): live-range-update tests (`adjust_ranges_*`, `:332-684`) vs method tests |
| `crates/script/elidex-js/src/vm/tests/tests_range.rs` | 929 | RC-1..RC-4 | **RC-0a** (projected): split on its existing section headers (`:32`, `:69`, `:137`, `:184`, `:437`, `:464`, `:481`, `:539`, `:601`) |
| `crates/script/elidex-script-session/src/mutation/html_fragment.rs` | 858 (tests from `:277`) | RC-4; also N9 (F11's fragment derivation, `ibp-sandbox.md:510-511`) | **RC-0b** (projected): inline tests to `mutation/tests_html_fragment.rs` (NEW), as `mutation/tests_replace_all.rs` already is. Lands **before N9** (PM 207: the first toucher splits; N9 rebases onto it) |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 2024 at `ee6c7a1a`; 729 after PR #578 | not touched (§4.2) | none: PR #578 is that split, and RC-1 lands after it. If a per-PR plan must touch `HostData`, it edits the post-#578 file (`mod.rs` or `engine_feature/bind.rs`) |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | RC-4 (+1-2 re-export); also edited by C0 (D-5's fragment arm, C0 memo O-4 (b)) | **re-measured at each per-PR plan** that touches it: if C0 (or anything else) has taken it to ≥ 1000, a standalone pure-move split comes first (CLAUDE.md touch-time split); at `ee6c7a1a` none |
| `crates/dom/elidex-dom-api/src/range/mutation.rs` | 640 | RC-1..RC-4 | none as prereq; new code goes into `containment.rs` / `contents.rs` (NEW) so it does not grow past ~800 |
| `range/mod.rs` 587, `range/boundary.rs` 299, `range_proto_mutation.rs` 262, `range_proto.rs` 906, `tests_mutation_observer/range_ops.rs` 586, `node_methods/clone.rs` 458, `elidex-custom-elements/src/upgrade.rs` 526, `node_methods_extras.rs` 293, `custom_elements/upgrade.rs` 487, `elidex-form/src/clone.rs` 304, `child_node/mod.rs` 459, `dom_bridge.rs` 696 | under 1000 | various | each per-PR plan re-measures; `range_proto.rs` (906) is touched only if toString's native changes |

RC-0a and RC-0b are pure moves (no text change beyond module declarations), reviewed with `git diff --color-moved`.

## §8. Out of scope, and slots

- **`#11-shadow-tree-live-range-tracking`** (open, unchanged): shadow-tree live-range boundary updates. The program
  keeps the shallow light-tree contract of `range/live.rs:38-55`. (D13 is a different defect: Range's own light-tree
  indices and walks on a host, not boundaries inside the shadow tree.)
- **`#11-range-client-rects`** (open, owner Layout): `getBoundingClientRect` / `getClientRects`, a consumer of the
  forced style/layout flush umbrella. Not touched.
- **`#11-insertadjacenthtml-context-validity`** (open; re-eval 2026-08-26, **overdue**,
  `project_open-defer-slots.md:709`): its trigger lists "Range work touching `apply_insert_adjacent_html`"; RC-4
  does not touch it, so the trigger does not fire. Its defect text also names `Range.createContextualFragment` in the
  parser-path family. That entry is **moot**: createContextualFragment parses into a new fragment and inserts
  nothing, so there is no pre-insert validity step to skip (HTML §8.5.7 steps 1-7). Proposed disposition (PM):
  narrow the family to innerHTML / outerHTML / setHTMLUnsafe / insertAdjacentHTML, extend-with-cause to 2026-12-01
  (cause: no parser-path insertion-validity pass is planned).
- **Selection.containsNode**: uses the Selection spec's own containment (`node_fully_contained`,
  `selection.rs:652-670`), not DOM "contained"; B1 does not replace it.
- **`#11-selection-document-membership`** (C0 R-6 (c), informational): its site `selection.rs:614-619` is in RC-1's
  file. RC-1 does not change it.
- **IBP-sandbox N9** (Layout): introduces `ParserScriptingMode` (Normal, Disabled, Inert), the scripting-disabled
  derivation and F11 at every fragment-parse caller (§0.2). RC-4 adds `Fragment` after N9 (PM 207).
- **Node document of Range-created nodes** (C3's write rule; R2's D8; shallow-clone stamping): a facet of
  `#11-ownerless-node-creation` (registered, PM 207), owned by **Layout** (C0 / C1). One mechanism: every creation
  site takes its owner from `node_document_for_new_nodes` (C1 rev 2), and reads use only `node_document` (C0).
  Appendix A is this program's input to C0 rev 2. Template contents' owner document is C1's
  (`#11-template-contents-owner-document`).
- **XML fragment parsing** (§5.4 (b)): `#11-domparser-xml-real-parsing` (`vm/host/dom_parser.rs:253-262`).

**Proposed slots outside the program's own fixes** (for the PM to register, fold or decline; owner left to the PM):

| Proposed ID | Defect | Why deferred | Trigger | Re-eval |
|---|---|---|---|---|
| `#11-trusted-types` | elidex has no Trusted Types; every sink that takes `(TrustedHTML or DOMString)` (createContextualFragment step 2, innerHTML, …) treats the string as is | program-wide gap (one mechanism for every sink), not a Range defect; until it exists, createContextualFragment step 2 is identity, like innerHTML | a Trusted Types / CSP `require-trusted-types-for` program | 2026-12-01 |
| `#11-processing-instruction-clone` | `CloneNode` refuses ProcessingInstruction with NotSupportedError (`node_methods/clone.rs:126-131`), an unslotted stub | unreachable from script (no PI producer, §0.2); the Range contents algorithm refuses a PI boundary before any mutation (E1) | a PI producer (XML parsing, `createProcessingInstruction`) | 2026-12-01 |
| `#11-attr-clone-node` | `attr.cloneNode()` throws NotSupportedError (`node_methods/clone.rs:120-125`); reachable from script through `document.createAttribute` (`char_data/attr.rs:22`) | not a Range defect: a range never holds an Attr, and RC-2 does not plan to edit `CloneNode`'s `invoke` (`node_methods/clone.rs:91`) (§5.2; if it does, the fix folds into RC-2) | an Attr-node program, or RC-2's diff touching that `match` | 2026-12-01 |
| `#11-fragment-script-run-on-insert` (conditional, §5.4 (a)) | a fragment-parsed `<script>` (Fragment mode) does not run when inserted, where the spec runs "prepare the script element" | only if RC-4's question (a) finds it; the defect is in dynamic script execution, not in Range | RC-4's per-PR plan answering (a) "no" | 2026-12-01 |
| `#11-fragment-parse-ce-upgrade-timing` (conditional, §5.4 (d)) | parsed custom elements get their upgrade reaction at insertion instead of at creation (HTML §13.2.6.1 steps 13, 15) | only if RC-4's question (d) finds a difference; a parser-path defect shared with innerHTML | RC-4's per-PR plan answering (d) | 2026-12-01 |

The two conditional slots are registered only if their trigger finds the defect. The script-run slot's owner
(dynamic script execution) is for the PM to assign. A `[CEReactions]` timing difference has no slot: every one of
the seven methods belongs to a slice, which fixes it (E2).

## §9. Order, suspension points, cross-lane dependencies, definition of done

**Order.** RC-0a → RC-1 → RC-2 → RC-3; RC-0b → (N9) → RC-4 (RC-4 is independent of RC-1..3). Cross-lane (PM 207):

- **Suspension points (PM decision 201).** Each RC is a suspension point: after any merged slice, L3 may switch to P1
  when C0 lands ("L3 switches to P1 at a slice boundary once C0 lands", PM 201). No slice is left half-landed; the
  program resumes at the next RC.
- **RC-1 after PR #578.** `LiveRangeRef` is written against the post-#578 `engine_feature/` layout (§4.2).
- **RC-2 is not gated on C0.** The node document of the nodes it creates is Layout's (§8); RC-2 adds no ownerless
  creation site (C3), and Appendix A, re-run at RC-2's head, goes to Layout.
- **`<template>` clone tests and C1.** RC-2's template tests assert structure, not node documents. The CE-pass rule
  (§4.3 condition 4) does not depend on C1.
- **RC-0b before N9.** RC-0b splits `html_fragment.rs`'s tests first (touch-time split: the first toucher splits);
  N9 rebases onto it.
- **RC-4 after N9 and C0.** N9 lands the `ParserScriptingMode` enum; RC-4 adds `Fragment` (§5.4). Under the Layout
  order N1 → C0 → C1 → T3 → N9, C0 and C1 have landed before RC-4; if C0 is late, the driver tells the PM before
  starting RC-4. The `range/mutation.rs:632` line: N9 re-points it, RC-4 deletes it (§6).
- **`mutation/mod.rs`** (994 lines, C0 edits it): re-measured at each per-PR plan (§7).

**Definition of done.**

- RC-0a, RC-1 (after #578), RC-2, RC-3 merged in that order; RC-0b before N9; RC-4 after N9 and C0.
- Every §0.4 silent-wrong row in the program has its negative control, red before and green after, recorded in its
  PR body, or its green-branch regression pin (§0.4).
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
- Lifecycle of the four pre-existing slots (`#11-range-full-impl` and the three re-homed ones): disposition
  **promote-to-PR** (Control D, `feedback_defer_lifecycle_policy.md:75`), with the program as the PR set, and a new
  re-eval date **2026-12-01** (the same as the proposed slots), or the home slice's landing if earlier.
- `#11-range-full-impl` re-eval (due 2026-11-01, venue N9's plan-memo, `docs/plans/2026-09-ibp-sandbox.md:763`): the
  re-eval outcome is this umbrella (promote-to-PR, above). The slot is homed in the program (RC-2 clone, RC-3
  surround, RC-4 contextual, the last adding the Fragment mode to N9's enum) and closes when all three land.
- Roadmap row (`m4-12-platform-gap-roadmap.md:1252`), replacement for its description and timing cells: "Range
  cloneContents + surroundContents (DOM §5.5) + createContextualFragment (HTML §8.5.7) through the Range contents
  program (`docs/plans/2026-11-range-contents-program.md`: RC-2 / RC-3 / RC-4). createContextualFragment parses
  through `elidex-script-session` (no `elidex-dom-api` → `elidex-html-parser` dependency, P1) and adds the Fragment
  parser scripting mode to IBP-sandbox N9's `ParserScriptingMode`." Timing: "program order (§9)" in place of "PR7 前"
  (D-26 PR7 landed in #458, `315ba316`). The current text "+ elidex-html-parser dep on dom-api" contradicts P1, and
  its "§4.4" / "DOM Parsing §3.2" cites are stale (`webref heading dom 4.4` = Interface Node;
  createContextualFragment is HTML §8.5.7).

## Open questions — for the PM

Cross-lane questions Q-X1 to Q-X4 of Rev 2 are settled by PM decision 207 and folded in above (§5.4, §8, §9). R5 is
withdrawn: the driver re-decided the created-set carrier (D-a′, §4.3). Rev 3's X4 is now D14 (§0.4), not a PM
question.

**Ratifications** (changes to PM decision 202's cut):

- **R1** Slice renumbering: PM 202's slices 3 / 1 / 2 / 4 become RC-1 / RC-2 / RC-3 / RC-4, plus RC-0a / RC-0b (§0.1),
  with the ledger home texts of §9.
- **R2** The reorder (toString first, with the containment primitives; §0.1).
- **R3** The added mechanism: `LiveRangeRef` (§4.2).
- **R4** D10 / D11 fixed in RC-1, not RC-3 (§0.4).

**Cross-lane** (Layout owns N9, C0, C1):

- **X1** Appendix A (the Range node-creation sites) goes to Layout for C0 rev 2 (driver sends).
- **X2** RC-4's two creation sites take their owner from `node_document_for_new_nodes` (C1 rev 2). Please confirm the
  entry exists under that name before RC-4 starts.
- **X3** (observation for Layout) `docs/plans/2026-09-ibp-sandbox-input.md:66` cites HTML §13.4 "steps 5, 10" for the
  mode derivation. Step 5 (contextDocument) is right. Step 10 is misattributed: it sets the tokenizer state. The
  scripting-disabled step, step 8, is missing (`webref body html html-fragment-parsing-algorithm`).
- **X5** (observation, outside the program) The live-range bridge fires no event past 10,000 previous siblings, so
  live ranges miss those updates (B3, §0.2). This is pre-existing. Is it a slot?

**Proposed slots** (register, fold or decline; Why / trigger / re-eval in §0.4 and §8):

- From §0.4, closed by the program's own slices: `#11-range-contained-enumeration-end-element` (D4),
  `#11-range-delete-extract-new-boundary` (D5), `#11-range-insert-node-step13-timing` (D12),
  `#11-range-compare-points-shadow-index` (D13, now raw-sibling tree access), `#11-range-insert-node-validity` (D10,
  D11), `#11-range-contents-doctype-check` (D9), `#11-clone-shadow-children-subtree` (D14, pre-existing, closed by
  RC-2). Folded IDs need "Not registered by design" homing rows.
- Withdrawn: `#11-range-contents-node-document` (R2's D8, now a facet of `#11-ownerless-node-creation`). It needs a
  "Not registered by design" homing row while the memo names it.
- From §8, outside the program: `#11-trusted-types`, `#11-processing-instruction-clone`, `#11-attr-clone-node`;
  conditional `#11-fragment-script-run-on-insert` (owner: dynamic script execution, to assign),
  `#11-fragment-parse-ce-upgrade-timing`. R2's "(E2 rule)" row is withdrawn (E2).
- Ledger: the lifecycle of the four pre-existing slots (promote-to-PR, 2026-12-01, §9); narrow and re-date the overdue
  `#11-insertadjacenthtml-context-validity` (§8); record the roadmap row text (§9).

**For the per-PR plans** (not PM questions): RC-1: the `RangePointError` fold (E3); D10 / D11 / D12 / D13
measurements. RC-2: the Attribute-stub check (§5.2). RC-4: §5.4 (a)-(e).

## Appendix A. Range node-creation sites for C0 rev 2

Every site in the Range code, and in the planned RC-2 / RC-4 algorithms, that creates a node, at `ee6c7a1a`.
"Ownerless" means the node gets no `AssociatedDocument` at creation. Re-run:
`git grep -n -E 'create_[a-z_]*\(|clone_(node|subtree)[a-z_]*\(|split_text_at_offset\(' -- crates/dom/elidex-dom-api/src/range crates/dom/elidex-dom-api/src/selection.rs crates/script/elidex-js/src/vm/host/range_proto_mutation.rs crates/script/elidex-js/src/vm/host/range_proto.rs`
(test modules and `create_element_wrapper` excluded by hand: `range/live.rs:498-1021`, `selection.rs:676-1036`).

| # | Site | Creates | Owner today | Fate in the program |
|---|---|---|---|---|
| A1 | `range/mutation.rs:266` (extract) | the result DocumentFragment | **ownerless**: `create_document_fragment()` → owner `None` (`crates/core/elidex-ecs/src/dom/mod.rs:519-521`) | replaced by A6 (RC-2) |
| A2 | `range/mutation.rs:303` (extract, same Text container) | Text clone | **ownerless**: `create_text` → owner `None` (`dom/mod.rs:492-494`) | replaced by A7 (RC-2) |
| A3 | `range/mutation.rs:354` (extract, partial start Text) | Text clone | **ownerless** (same) | replaced by A7 (RC-2) |
| A4 | `range/mutation.rs:392` (extract, partial end Text) | Text clone | **ownerless** (same) | replaced by A7 (RC-2) |
| A5 | `range/mutation.rs:505` (insertNode step 7) | the split Text tail, via `split_text_at_offset` | `create_text_with_owner(tail, owner_document(text))` (`char_data/split_text.rs:148-149`): owned when the Text resolves an owner, including through `owner_document`'s tree-position fallback, which C0 D-1 deletes | unchanged (RC-1 keeps the primitive) |
| A6 | RC-2 contents algorithm, "clone the contents" / "extract" step 1 | the result DocumentFragment | (planned) today's creation call, so ownerless as A1 | owner rule: Layout (§8) |
| A7 | RC-2, CharacterData clones (same-container and partial start / end cases) | shallow canonical clone + set data | (planned) `clone_node_with_shadow_honor(deep = false)` → `clone_node_shallow` stamps none (`node_methods/clone.rs:320-326`): **ownerless** | owner rule: Layout |
| A8 | RC-2, "a clone of" the first / last partially contained child | shallow canonical clone of an element | (planned) as A7: **ownerless**. If the child is a clonable shadow host, the replicated shadow root and its children are stamped from `owner_document(src)` (`node_methods/clone.rs:239-243`, `:282-285`, `:294-301`) | owner rule: Layout |
| A9 | RC-2 clone mode, contained children | deep canonical clone | (planned) `clone_subtree` stamps the root and every descendant from `owner_document(src)` (`crates/core/elidex-ecs/src/dom/tree_clone.rs:204-232`); **ownerless** when the source resolves no owner (`:223-227`) | owner rule: Layout |
| A10 | RC-2, `<template>` clones under A8 / A9 | template content fragment and its clones | `attach_template_contents(dst, clone_doc)` (`node_methods/clone.rs:422`), `clone_doc` = `owner_document(src)` | C1 (`#11-template-contents-owner-document`) |
| A11 | RC-4, HTML §8.5.7 step 6 | the `body` context element | (planned) created in the start node's node document | `node_document_for_new_nodes` (C1 rev 2, PM 207) |
| A12 | RC-4 result | the DocumentFragment holding the parsed roots | (planned) node document = the start node's (P4) | `node_document_for_new_nodes` |

Not Range sites: the nodes the fragment parse creates (parser path, `html_fragment.rs` / the parser crates); RC-3
creates no node (`newParent` is the caller's); `selection.rs:687`, `:692` are test helpers.
