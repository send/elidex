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

**Rev 4** (2026-11-01) — plan-review R3 (0 CRIT / 9 IMP / 28 MIN), PM decisions 215 and 216, and **driver decision
D-g**. Reaction order: clone creation becomes a `MutationEvent` (`CloneCreated`), emitted once by the canonical
cloner's entry; the existing `CustomElementReactionConsumer` turns it into an in-stream placeholder in the reaction
queue; the VM native resolves every placeholder before it returns, so no script runs first. `cloneNode` takes the
same path, and the separate post-clone CE seam is retired (§4.3; the check against the code is §0.3 item 18). The form
pass runs once per clone root. Measured: elidex drains reactions at checkpoints, not when a `[CEReactions]` method
returns (§0.2, E2, PM list). RC-1 adds `WrongDocumentError` and `InvalidNodeTypeError` to `DomApiErrorKind` with their
mapper arms. RC-4's dependencies are listed without a Layout order (PM 215 Q-Y1, PM 216). RC-4's script-state test is
removed because elidex has no such state; a slot is proposed instead. D14 lands before C1, and C1 rebases (PM 215 Q-Y2,
PM 216). RC-0a also splits `node_methods/tests/clone.rs`. D14 is measured: an existing test pins the wrong result. The
§6 sweep patterns are re-derived, and "clone a single node" is cited by its own anchor. R1-R4 are ratified by PM
decision 215.

**Rev 5** (2026-11-01) — plan-review R4 (0 CRIT / 7 IMP / 27 MIN), PM decisions 222-224, and **driver decision D-g′**
(the last revision before the PR and `/external-converge`, PM 222). Clone creation stays the `MutationEvent::CloneCreated` (NEW)
event, but its two consumers now act **at dispatch time**: `CustomElementReactionConsumer` runs the CE clone pass and
enqueues the upgrades in place, and `FormControlReconciler` runs the form pass. Rev 4's placeholder, resolver, VM
seam, fail-safe arm and seam census are removed (§4.3; the checks against the code are §0.3 items 26-29). This
**reverses #466's I2 and I3**, a design change to a ratified surface for `/external-converge` R1 (§4.3 "#466
reversal"). E2's base is now the spec's element-queue model; the flat-FIFO grouping difference is the registered
`#11-ce-element-queue-grouping` (§9), and the drain point is the proposed `#11-ce-reactions-method-scope-drain`, with its
full proposal in §9 (PM 224). RC-1 folds every host-side Range DOMException decision into the engine (E3). New defect
D15 (`document.cloneNode(true)` keeps the global registry association, RC-2). RC-4 reads the realm document through
the J12 route and gets new nodes' document from C0's `EcsDom` entry (PM 223). §6's step grep catches hyphenated forms.

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

The renumbering, the reorder, the added mechanism and the D10 / D11 move (R1-R4) were ratified by PM decision 215.
R5 (the created-set carrier) is withdrawn. The carrier is now the clone-created event, handled at dispatch time
(driver decision D-g′, §4.3). It replaces Rev 4's D-g placeholder and Rev 3's D-a′ returned list.

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
- Reaction queue and drain (D-g′):
  - `Vm::bind` installs the `ConsumerDispatcher` (`crates/script/elidex-js/src/vm/vm_api.rs:292-316`). Its
    `dispatch` passes every `MutationEvent` to every consumer in field order, `form_control` fourth and
    `custom_elements` last (`crates/script/elidex-js/src/vm/consumer_dispatcher.rs:148-158`).
  - Consumers get `&mut EcsDom` (`MutationDispatcher::dispatch`, `crates/core/elidex-ecs/src/dom/mutation_event.rs:351`,
    "consumers that derive secondary ECS state from the event … need component-write access at dispatch time"). The
    re-entry contract forbids only `EcsDom` mutation primitives during dispatch (`mutation_event.rs:25-31`).
    Precedent for component writes at dispatch: `FormControlReconciler::handle_insert` walks the inserted subtree and
    calls `create_form_control_state` (`crates/dom/elidex-form/src/reconciler.rs:84-95`), which reads the tree and
    writes only components (`elidex-form/src/init.rs:63-96`, with its helpers `finalize_control` `:128` and
    `apply_element_state_flags` `:150`).
  - The queue is one flat FIFO: the consumer `extend`s it (`consumer.rs:120-121`), and `flush_ce_reactions` drains it
    in waves, each wave the whole queue in order, with reactions enqueued during a wave going to the next wave
    (`vm/host/custom_elements/flush.rs:47-71`). The spec keeps one reaction queue per element and invokes all of an
    element's reactions at the position where the element first entered the element queue (`webref body html
    custom-element-reactions-stack`, `enqueue-an-element-on-the-appropriate-element-queue`,
    `invoke-custom-element-reactions` step 1.3). The two orders differ whenever one element's reactions are interleaved
    with another's (§9, `#11-ce-element-queue-grouping`).
  - `CustomElementReactionConsumer::handle` matches `Insert` / `Remove` / `AttributeChange`, and `_ => {}` drops the
    rest (`crates/dom/elidex-custom-elements/src/consumer.rs:75-96`). It shares the registry and the queue
    (`ce_reaction_queue`, `crates/script/elidex-js/src/vm/host_data/mod.rs:558-567`) through `Arc<Mutex<…>>`
    (`consumer.rs:56-70`), and it enqueues synchronously during dispatch (`:115-122`, `:131-135`).
  - Other crates fire events through `pub` helpers on `EcsDom`, for example `fire_split_text` /
    `fire_normalize_merge` (`crates/core/elidex-ecs/src/dom/mod.rs:231`, `:258`), which `elidex-dom-api` calls
    (`char_data/split_text.rs:198`, `node_methods/core.rs:138`).
  - The queue drains only in `flush_ce_reactions` (`vm/host/custom_elements/flush.rs:39-97`). Its callers are the
    `eval` tail (`vm/interpreter.rs:54`), the engine's post-call settle (`engine.rs:165`), timers
    (`vm/natives_timer.rs:281`) and `customElements.define` (`custom_elements/define.rs:204`): `git grep -n
    'flush_ce_reactions(' -- crates/script/elidex-js/src`. **No `[CEReactions]` native drains** at its own return,
    whereas HTML §4.13.6 `[CEReactions]` steps 1-4 invoke the element queue before the method returns
    (`webref body html cereactions`). So from the moment a native enqueues until it returns, no script runs and no
    reaction is invoked.
- MutationEvent and shadow roots: the tree primitives fire no `Insert` when `node` or `parent` is a ShadowRoot
  (`crates/core/elidex-ecs/src/dom/tree/mutation.rs:289-291`, in `fire_after_insert`). No consumer hears those events,
  the live-range bridge included.
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
  single node" step 8 "Set copy's node document to document" for every copy (`webref body dom clone-a-single-node`).
- D14 is pinned by an existing test. `clone_node_shallow_replicates_own_clonable_shadow_shallowly`
  (`crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs:792-827`) asserts that the shadow child of a
  `cloneNode(false)` clone has no children (`:823-826`). `clone_node_shallow_shadow_children_carry_owner_document`
  (`:902`) pins the hand stamp of `:294-301`.
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
  `:195-219`, `#[non_exhaustive]`) has neither `InvalidNodeTypeError` (surroundContents step 2 throws it) nor
  `WrongDocumentError`. The VM maps it in one place, `dom_api_error_to_vm_error`
  (`crates/script/elidex-js/src/vm/host/dom_bridge.rs:390-429`, a private `fn`). A kind without an arm there falls to
  `_ => VmError::internal` (`:427`).
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
    (PM 207). R-6 (c) `#11-selection-document-membership` is a **proposal in C0's memo, not a registered slot**; it
    names `selection.rs:614-619` (informational, §8).
  - PM 157: N4's planned `EcsDom::effective_node_document` (NEW) (in N4's plan, not in the code; `ibp-sandbox.md:228`) is replaced by C0's `node_document` in C0's
    plan-review, so the engine has one node-document reader. This memo states no Layout order (PM 215 Q-Y1); RC-4's
    dependencies are listed in §5.4.
  - PR #578 (`vm-host-data-split`, open, PM 203) moves `HostData::split_dom_*` projections, among them
    `split_dom_and_live_ranges` (at `ee6c7a1a`: `crates/script/elidex-js/src/vm/host_data/mod.rs:939`), into
    `host_data/engine_feature/bind.rs`, and leaves `host_data/mod.rs` at 729 lines (`gh pr view 578 -R send/elidex`;
    open, head `b8e9add1`). The moved accessor keeps its signature, `(&EcsDom, &mut LiveRangeRegistry)`: a shared
    `&EcsDom` (`gh pr diff 578 -R send/elidex`, the `+` side of `fn split_dom_and_live_ranges`). The registry is
    also reachable as the `pub(crate)` field `live_range_registry` (for example `ctx.host().live_range_registry.register(…)`,
    `vm/host/range_proto.rs:288`).

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
    stays the form pass's alone, and the CE pass gets clone roots (D-a′; Rev 4 replaces the carrier with D-g, §4.3).
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

Rev 3, the check of driver decision D-a′ against the code at `ee6c7a1a`. The order analysis carries into D-g
unchanged: the clone roots and their scopes are the same, and Rev 4 fires them as events instead of returning them.

17. **D-a′ order, clone mode: matches, except one place.** "Clone the contents" creates, in order: the fragment
    (not an element); the first partially contained child's clone (step 13.1), then its subrange's clones
    (step 13.4); each contained child's deep clone (step 14.1); the last partially contained child's clone, then its
    subrange's (steps 16.1, 16.4); CharacterData clones at steps 4, 12 and 15 are not elements (`webref body dom
    concept-range-clone`). Clone roots listed in that order, recursing into each subrange where the spec does, give
    the spec's order of roots. Within a `Subtree` root, today's walk (`walkers.rs:54-80`: visit the node, then its
    light children's subtrees in order, then its shadow children) is the order of "clone a node": the copy
    (step 2), the children in tree order, each recursively (step 5), then the shadow children (step 6.7). Template
    cloning steps (step 3) create the content clones, which the walk skips, as today (§4.3 condition 6).
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

Rev 4, the check of driver decision D-g against the code at `ee6c7a1a`, and premises in the R3 dispositions that the
code or the spec corrects:

18. **D-g is feasible. Both conditions hold.** (Rev 5: the placeholder and the resolution step are superseded by
    D-g′, items 26-29. The wiring bullet and the order example stand: under D-g′ the `Upgrade` itself, not a
    placeholder, sits at the event's position.)
    - **The wiring can carry the event.** The one installed dispatcher passes every `MutationEvent` to
      `CustomElementReactionConsumer::handle` (§0.2). A new variant reaches it once its `match` gains an arm, and
      `elidex-dom-api` can fire it through a `pub` `EcsDom` helper, as `fire_split_text` already does (§0.2). The
      consumer already holds the queue, so the placeholder goes into the same `VecDeque` as `Disconnected` /
      `Connected`, at the position the event fired.
    - **Resolution precedes script.** The queue drains only at checkpoints, never inside a native (§0.2). A native runs
      argument conversion, then the engine call (Rust only; the dispatcher's consumers are Rust), then its seam, then
      wrapper creation. So a resolution step in the seam runs after the last placeholder of the call is enqueued and
      before any script or reaction runs. `customElements.define` cannot run in that window, so the registry the seam
      reads is the one the clone saw.
    - **Order example.** Extract step 17.1 creates the partially contained child's clone, which fires the event
      (placeholder). Step 17.4 extracts the subrange, and its moves fire `Remove`, which enqueues `Disconnected`.
      The queue therefore holds x-clone's placeholder, which resolves to its `Upgrade`, before x-m's `Disconnected`.
      That is the spec order (`webref body dom concept-range-extract`, steps 17.1-17.4). Today the CE pass runs after
      the algorithm, so every `Upgrade` lands after every `Disconnected`.
19. **"Where `[CEReactions]` natives drain"**: none drains (§0.2). D-g needed "no script before resolution", which held
    for the same reason; D-g′ needs nothing of the drain, because it resolves nothing later. The drain point itself
    differs from HTML §4.13.6 for every `[CEReactions]` method in the engine, `appendChild` included, so E2 is restated
    (§2) and the drain point is the proposed `#11-ce-reactions-method-scope-drain` (§9).
20. R3 Ax3-MIN "extract uses `commit_range_after_mutation` (`range_proto_mutation.rs:124`)": the call is at `:125`
    (`git grep -n 'commit_range_after_mutation' -- crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`
    → `:12` (module doc), `:34` (definition), `:99` (delete), `:125` (extract)).
21. R3 Ax2-MIN5 "the bridge skips ShadowRoot-parented mutations (`tree/mutation.rs:289-291`)": that line is the
    engine's fire site, not the bridge, so **no** consumer hears those inserts (§0.2). L1's light-tree scope (§2) follows
    from it either way.
22. R3 Ax4-IMP "the lookahead must exclude only a following digit": `(?!\d)` also matches `§4.10` inside `§4.10.5`.
    The pattern in §6 uses `(?!\.?\d)`, which excludes a following digit or `.digit`, so it still catches a
    sentence-final `§4.10.` and does not catch `§4.10.5` or `§4.100`. It was checked on a fixture holding those four
    forms.
23. Rev 3 cited `webref body dom concept-node-clone` for the steps of "clone a single node" (2.1-2.4, 8). That anchor's
    output does not end at "clone a node" step 7 (Rev 4 said so; R4 Ax4 found it wrong): it runs on into "clone a
    single node" and the algorithms after it. The cite was imprecise for another reason: it names the "clone a node"
    algorithm, so a reader looking for step 2.4 or step 8 lands on the wrong algorithm's numbering. "Clone a single
    node" has its own anchor, `webref body dom clone-a-single-node`, which shows steps 2.1-2.4 and 8 under that
    algorithm's name. Rev 4 cites it, and Rev 5 keeps the cite.
24. D14's basis is **measured**, not predicted: an existing unit test asserts the wrong result (§0.2). RC-2 inverts that
    test.
25. D-g's "`cloneNode` … one Subtree root event": the event's scope follows the operation's `subtree` flag (§4.3
    condition 1), so `cloneNode(false)` fires `SelfOnly`. For a shallow clone the two scopes visit the same set,
    because the copy has no light children: the root, then its shadow tree. That is today's walk on that root
    (`walkers.rs:54-80`), so `cloneNode`'s CE behaviour is today's for both flag values, apart from D14 (§4.3
    condition 1).

Rev 5, the check of driver decision D-g′ against the code at `ee6c7a1a` (the four checks the driver asked for; all
four hold), and premises in the R4 dispositions and the Rev 5 brief that the code or the memos correct:

26. **(a) A SelfOnly root has no light children when its event fires.** The event fires inside
    `clone_node_with_shadow_honor`, after its worklist and before `Some(cloned)` (`node_methods/clone.rs:227-305`).
    With `deep = false` that function creates no light child of the root. `clone_recording` takes the shallow cloner
    (`:313-327`). The worklist adds only template content clones, under the content fragment (`replicate_template_contents`,
    `:262`), and shadow children, under the replicated ShadowRoot (`:286-303`, `append_child(cloned_shadow, …)`). The
    contents algorithm does not exist at `ee6c7a1a` (today's extract builds its Text clones with `create_text` and
    calls no cloner, §0.2), so the other half of the check is the spec order RC-2 implements. "A clone of
    firstPartiallyContainedChild" is step 13.1 (clone) / 17.1 (extract). The subfragment is computed at step 13.4 /
    17.4 and appended to the clone at step 13.5 / 17.5 (`webref body dom concept-range-clone`, `concept-range-extract`).
    Steps 16.x / 20.x mirror this for the last partially contained child. Every append into a SelfOnly root is
    therefore after the entry, and so after its event. The visit set at the event is the root plus its shadow tree,
    and D-a″ holds by construction. A Subtree root's event fires after the whole worklist, so its whole subtree,
    shadow trees and template contents included, exists.
27. **(b) Both passes write components and read the tree; neither calls an `EcsDom` mutation primitive.**
    - `apply_clone_creation_ce_semantics` (`elidex-custom-elements/src/upgrade.rs:125-156`): one
      `for_each_shadow_inclusive_descendant` walk (read), `prepare_upgrade` (a pure read, `:55-99`), then
      `world_mut().get::<&mut CustomElementState>` to clear `is_value` (`:149-151`), a component write.
    - `apply_clone_form_state` (`elidex-form/src/clone.rs:44-63`): `collect_template_inclusive_descendants` (read,
      `init.rs:111-126`), `world_mut().remove_one::<ClonedFrom>` (`:62`), and `copy_form_state` (`:74-152`). That calls
      `create_form_control_state`, the same function `FormControlReconciler::handle_insert` already calls at dispatch
      (`reconciler.rs:84-95`), then writes `FormControlState` and `ElementState`.
    - So both are allowed during dispatch under the re-entry contract (`mutation_event.rs:25-31`). The consumers get
      `&mut EcsDom` (§0.2), and the CE arm needs it for the `is_value` write. The existing `handle_insert` takes
      `&EcsDom` only because it writes nothing.
28. **(c) No new lock; the lock order is unchanged.** The CE arm locks the registry, runs the pass, drops the guard,
    then locks the queue and extends it. That is `handle_insert`'s shape (`consumer.rs:98-123`) and the lock-disjoint
    convention of `handle_attribute_change` (`:146-148`). The form arm takes no lock. Lock-holding paths: today's seam
    took the same two locks in the same order, registry then queue (`vm/host/custom_elements/upgrade.rs:135-157`).
    `flush_ce_reactions` drains the queue under its lock and invokes after releasing it (`flush.rs:58-70`). So a
    constructor that calls `cloneNode` re-enters the dispatch path with no guard held, as an `appendChild` from a
    constructor does today through `handle_insert`. One difference: the consumer treats a poisoned mutex as fatal
    (`expect`, `consumer.rs:116`), whereas today's seam recovers (`unwrap_or_else(PoisonError::into_inner)`,
    `upgrade.rs:138`). Every insertion already takes the `expect` path, so this adds no new failure mode.
    RC-1's ranges mutex (L4) is not held across a cloner call either, because the entry fires events (L4, §2).
29. **(d) `cloneNode` behaves as today.**
    - **CE: same set, same order, earlier enqueue, same queue position.** Today the post-pass runs after
      `invoke_dom_api` returns (`node_methods_extras.rs:211-221`). Between the entry's return and that point the
      native runs the tail of `CloneNode`'s `invoke` (`get_or_create_wrapper`, `node_methods/clone.rs:160-164`; identity map
      only), then `invoke_dom_api`'s `drain_notify_records` (`vm/host/dom_bridge.rs:32-40`, called at `:514`). That feeds
      `queue_mutation_record` only, the MutationObserver queue (`vm/host/mutation_observer.rs:459`); the CE record
      leg is `deliver_mutation_records`, an embedder entry not called by natives. Then `create_element_wrapper` and
      `install_document_methods_if_cloned_doc` (`node_methods_extras.rs:249-265`), which are VM object operations.
      None of these mutates the tree, fires an event or pushes onto `ce_reaction_queue` (`git grep -l
      ce_reaction_queue -- crates/script/elidex-js/src` lists no file on that path). None reads `is_value`
      either: the VM writes it only in `custom_elements/creation.rs:233` (`git grep -n is_value -- crates/script/elidex-js/src
      ':!*tests*'`), so clearing it earlier is unobservable. So the walk sees the same tree, gives the same
      candidates in the same order, and they land at the same queue position.
    - **Form: unchanged.** The same function on the same root over the same tree, still before any insertion, so I3's
      detached-read contract holds (§4.3 "#466 reversal"). The only change is that the form pass now runs before the
      CE pass (dispatch field order, §0.2), where the shim ran CE first. The two touch disjoint components (CE:
      `CustomElementState`; form: `ClonedFrom`, `FormControlState`, `ElementState`), and neither reads the other's,
      so the swap is unobservable.
    - **Unbound / no dispatcher.** With no dispatcher installed the event reaches no consumer. Today's passes run only
      in the VM shim behind `host_if_bound()` (`node_methods_extras.rs:229`, and `ctx.host_if_bound()` inside `enqueue_subtree_upgrade_reactions`), and `Vm::bind` is what installs the
      dispatcher, so the coverage is the same.
30. Premise errors in the Rev 5 brief and the R4 dispositions:
    - "J12 … `vm/host/settings_route.rs`, NEW in N9": the route and the file are **N4's**. J12 is "from N4"
      (`docs/plans/2026-09-ibp-sandbox.md:399`), and the input file assigns `settings_route.rs` (NEW file) to N4
      (`docs/plans/2026-09-ibp-sandbox-input.md:63`). F10's description names the file (`ibp-sandbox.md:495`). RC-4
      needs N9, which comes after N4, so the dependency is unchanged. The memo cites the route as N4's.
    - "find where the contents algorithm appends subrange results": there is no such code at `ee6c7a1a` (item 26).
    - `#11-ce-element-queue-grouping` "the PM registers it" (PM 222): the ledger records it as registered on
      2026-11-01 under PM 223 (`project_open-defer-slots.md`, the slot's own entry). The memo cites it as registered,
      PM 223.
    - Ax1-MIN "`require_attached_node` (`range_proto.rs:520-538`)", "`:526-536`", "`validate_boundary_node_and_offset`
      `:502-510`": the functions are at `:519-539` and `:488-513`. The throws are at `:531` and `:505`. The
      dispositions' ranges sit inside the functions, so the findings stand.
31. **#466's surface (PM 222).** D-g′ changes a surface that #466 ratified. Its reasons are in §4.3 "#466 reversal",
    and it is reviewed in `/external-converge` R1.

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
| D14 | "Clone a node" step 6.7 clones a clonable shadow root's children with the operation's `subtree` flag, not true: `h.attachShadow({mode:'open', clonable:true}).append(x)` with `x` holding a child; `h.cloneNode(false).shadowRoot.firstChild` has no children (spec: a deep copy of `x`). RC-2's shallow clone of a partially contained shadow host hits the same path | silent-wrong | **measured** (`node_methods/tests/clone.rs:792-827` asserts the wrong result, §0.2) | `node_methods/clone.rs:288-290` (`clone_recording(dom, child, deep, …)`); `webref body dom concept-node-clone` step 6.7 "subtree set to true" | RC-2 (in the canonical cloner, so `cloneNode` and the Range methods get it at once) | `#11-clone-shadow-children-subtree` (**registered**, PM 212) |
| D9 | extractContents does not throw HierarchyRequestError for a contained doctype (step 11); e.g. `r.selectNodeContents(document); r.extractContents()` moves the doctype | silent-wrong | predicted | no doctype check in `mutation.rs:265-402` | RC-2 | new: proposed `#11-range-contents-doctype-check` |
| D15 | `document.cloneNode(true)` keeps every element clone's registry association. "Clone a single node" step 2.3 replaces a global registry with document's custom element registry's effective global custom element registry, where document is the copy (step 7). The copy is made by "create a document", which sets no registry, so it is null, and so is its effective global one. So the spec gives each element clone a null registry and no upgrade. `propagate_ce_identity` copies `registry` unchanged (`node_methods/clone.rs:344-366`); the default association is `Document` (`elidex-custom-elements/src/state.rs:70-78`), and `prepare_upgrade` skips only `Null` (`upgrade.rs:74-76`). So a defined custom element inside a cloned document is predicted to get an `Upgrade` | silent-wrong | predicted | `webref body dom clone-a-single-node` steps 2.1-2.4, 7; `create-a-document`; `document-custom-element-registry` | RC-2 (in the canonical cloner, which RC-2 edits; when the source is a Document the operation document is the copy, the `shadow_doc` branch at `node_methods/clone.rs:239-243`, so the rewrite is one branch in `propagate_ce_identity`; elidex has no scoped registries, so step 5.3's scoped-registry copy cannot apply) | new: proposed `#11-clone-document-registry-rewrite` |
| — | extractContents fragment / Text clones have no node document (R2's D8) | silent-wrong | predicted | `mutation.rs:266`, `:303`; Appendix A | **out of the program**: Layout, facet of `#11-ownerless-node-creation` (PM 207, §8) | R2's proposed `#11-range-contents-node-document` withdrawn |
| — | cloneContents unimplemented | visible (throws) | measured (`tests_range.rs:441-452`) | §0.2 stubs | RC-2 | `#11-range-full-impl` (clone part) |
| — | surroundContents unimplemented | visible | predicted (no stub test: `git grep -n surroundContents -- crates/script/elidex-js/src/vm/tests/tests_range.rs` = 0) | §0.2 (`range_proto_mutation.rs:213-262`) | RC-3 | `#11-range-full-impl` (surround part) |
| — | createContextualFragment unimplemented | visible | measured (`tests_range.rs:454-463`) | §0.2 | RC-4 | `#11-range-full-impl` (contextual part) |

**Classification and ledger handling.** Every row is **pre-existing** at `ee6c7a1a` (none is introduced by this
program), and every proposed ID is **closed by the program's own home slice**. D14's slot is registered (PM 212);
the other new IDs are still proposed. Slot IDs marked "proposed" are for the
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
| D15 | none beyond the row's fix: if green, the regression pin stays and no branch is added |
| D14 | the subtree flag is per pair, not per operation: the step 6.7 clone passes true, and every pair it produces keeps true for passes 3 and 4 (today both take the operation's `deep`: pass 3 at `node_methods/clone.rs:262`, pass 4's child clone at `:290`, so a `<template>` or a nested clonable host inside that shadow tree would still be cloned shallowly); the shallow-branch stamp (`:294-301`) goes with it. D14 is measured, so its row has no green branch: the existing test (`tests/clone.rs:792-827`) is inverted |

## §1. Shape and verdict

**Edge-dense** (CLAUDE.md "Edge-dense work"): the work intersects boundary-point containment, MutationRecord
emission, live-range updates during the algorithm's own mutations, the clone-a-node pipeline (custom elements, form
state, template contents, shadow roots), and for RC-4 the parser / session path (§2). So: this umbrella, then one
per-PR plan per row below, each through **`/elidex-plan-review` before implementation** (rule, not judgment). A
per-PR plan that passes plan-review under this umbrella is a terminal unit.

| PR | Scope | Depends on | Closes | Plan-review |
|---|---|---|---|---|
| RC-0a | Pure-move test splits: `range/live.rs`, `selection.rs`, `range/tests.rs`, VM `tests_range.rs`, `node_methods/tests/clone.rs` (§7) | — | — | not required (pure move; `git diff --color-moved`) |
| RC-1 | Containment primitives; exposed child list for every Range index, length and walk (D13); live-range reference; toString, deleteContents, Selection.deleteFromDocument, insertNode (steps 1, 6, 13) moved onto them; extract's enumeration and boundary write moved to the canonical set and `LiveRangeRef` (interim); `DomApiErrorKind` gains `WrongDocumentError` / `InvalidNodeTypeError` with mapper arms; `RangePointError` and the host-side Range DOMException decisions of `range_proto.rs` fold into `DomApiError` (E3) | RC-0a; **PR #578** (PM 207, §4.2) | `#11-range-tostring-boundary-text`; proposed D4, D5 (delete half), D10, D11, D12, D13 | required |
| RC-2 | One "contents" algorithm for clone and extract; cloneContents; the clone-created event and its two dispatch-time consumers, shared with cloneNode (D-g′, §4.3); D14; D15. Nodes are created with today's canonical owner rule (C3) | RC-1 (not gated on C0); lands **before C1**, which rebases on D14 (PM 215 Q-Y2, PM 216; §9); shares the `MutationEvent` census with C0's `Adopt` (§9) | `#11-range-extract-partial-ancestor-clone`, `#11-range-comment-extract-clone`, `#11-clone-shadow-children-subtree`, `#11-range-full-impl` clone part; proposed D5 (extract half), D9, D15 | required |
| RC-3 | surroundContents | RC-2 | `#11-range-full-impl` surround part | required |
| RC-0b | Pure-move split of `html_fragment.rs` inline tests (§7); lands **before N9** (PM 207, touch-time split: the first toucher splits; N9 rebases onto it) | — | — | not required |
| RC-4 | createContextualFragment through the session fragment-parse path; adds the `Fragment` value to N9's `ParserScriptingMode` and maps it in both parser backends (the script-element half is unobservable in elidex, §5.4) | RC-0b; three landings, no order claimed (PM 215 Q-Y1, PM 216): **IBP-sandbox N9** (the enum, F11; N4's J12 route before it), **C0** (`EcsDom::node_document`, and C0's `EcsDom` entry for new nodes' document, PM 223), **C1** (kept per PM 216; its reason moved to C0, so it is a PM question, §5.4); independent of RC-1..3 (§5.4, §9) | `#11-range-full-impl` contextual part (the slot closes when RC-2, RC-3, RC-4 have all landed) | required |

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
  snapshot-then-overwrite. **Scope: the light tree.** The engine fires no `Insert` for a ShadowRoot-parented mutation
  (`tree/mutation.rs:289-291`, §0.2), so boundaries inside a shadow tree are not tracked. That gap is
  `#11-shadow-tree-live-range-tracking` (§8), not this program's. No Range algorithm here mutates a shadow tree (B3).
- **L2 (subranges)**: the "new live range" of extract / clone steps 17.3 / 20.3 (13.3 / 16.3 for clone) is a plain,
  unregistered value. Justification: after its own step 3 the recursion reads only the captured originals, and its
  only write (extract step 15 on the subrange) has no reader.
- **L3 (other live ranges)**: change only through the hooks the M1 primitives fire (replace data, remove, insert,
  split). No algorithm adjusts another range directly.
- **L4 (lock)**: the ranges mutex (`range/live.rs` "Lock invariant") is never held across a DOM mutation or a call of
  the canonical cloner entry (which fires `CloneCreated`, C2); `LiveRangeRef` locks per read or write.
- **C1 (canonical clone)**: every clone made by clone/extract is "clone a node" through the canonical entry
  (`clone_node_with_shadow_honor`; no recording variant, §4.3), so custom-element identity, template
  contents, clonable shadow roots and the form-control marker follow the same rules as `cloneNode`. A CharacterData
  clone is a shallow canonical clone, then "set data" with the substring.
- **C2 (post-clone passes on created nodes only, in spec order)**: the registry-dependent custom-element pass and the
  form-state pass visit exactly the entities created by cloning in this call, never the moved originals that extract
  places under a clone. Every clone root fires one **`MutationEvent::CloneCreated { root, scope }`** (NEW) just before
  the canonical cloner's entry returns (driver decision D-g′, §4.3). `scope` is `Subtree` or `SelfOnly`, from the
  operation's `subtree` flag. Both passes run **at dispatch time**, inside the clone. `CustomElementReactionConsumer`
  runs the CE pass (D-a″ visit sets, today's walk per root) and pushes the `Upgrade`s at once, so each upgrade sits at
  its creation point among the `Disconnected` / `Connected` reactions of the same call (§0.3 items 18, 26).
  `FormControlReconciler` runs the **form pass**, today's function and marker (`<input>` / `<textarea>` only, §4.3
  condition 5), once per clone root, with the cap counted from that root. That is `cloneNode`'s contract today, now
  applied to each root. A SelfOnly root has no light children when its event fires (§0.3 item 26), so neither pass
  can reach a moved original. `cloneNode` and the Range methods share this one path, and template-content clones stay
  outside the CE pass (rule in §4.3), as today.
- **C3 (node document: not this program's)**: the spec creates the fragment given the start node's node document
  ("clone the contents" / "extract" step 1, `webref body dom concept-range-clone`, `concept-range-extract`), and
  every clone gets its operation's document ("clone a single node" step 8). How each creation site gets its owner is
  the facet of `#11-ownerless-node-creation` owned by Layout (PM 207: owners come from C0's `EcsDom` entry for new
  nodes' document, PM 223; reads use only C0's `node_document`). The program's rule is: RC-2 creates
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
  their mutations (disconnected / connected via the mutation-event leg, upgrades via C2) are enqueued **at the spec
  step that enqueues them**. The spec's base is the element-queue model: each element has its own reaction queue,
  and all of an element's reactions run at the position where it first entered the element queue (§0.2; `webref
  body html enqueue-an-element-on-the-appropriate-element-queue`, `invoke-custom-element-reactions`). elidex has one
  flat FIFO (§0.2). The two models agree when no element's reactions are interleaved with another element's, and
  differ otherwise (examples in §9). **Rule**: the Range slices add **no divergence beyond the flat-FIFO grouping**.
  Each slice enqueues at the spec's steps for the methods it owns (the seven are split across RC-1 to RC-4, §5), and
  its tests assert **only orders that are equal under both models**. A difference that the grouping alone explains
  is the registered engine-wide `#11-ce-element-queue-grouping` (§9), not a slice defect. Any other difference is a
  defect fixed in that slice. An upgrade-timing difference for **parsed** elements (enqueued at insertion instead of
  at creation) is neither: it goes to RC-4's question (d) and its conditional slot (§5.4, §8). **Drain point
  (measured, §0.2): not the program's.** HTML §4.13.6 `[CEReactions]` steps 1-4 invoke the reactions before the
  method returns (`webref body html cereactions`). elidex drains at checkpoints instead (eval tail, the engine's
  post-call settle, timers, `define`), for every `[CEReactions]` method in the engine. A per-method drain in four
  Range natives alone would be a second mechanism beside every other method's (one issue, one way). The engine-wide
  drain point is the proposed `#11-ce-reactions-method-scope-drain` (§9). The program's tests assert queue order,
  read at the next checkpoint.
- **E3 (engine→host error contract)**: an engine algorithm that can throw returns, on error, a `DomApiError {
  kind, message }` (`crates/script/elidex-script-session/src/types.rs:149-154`, the type `CloneNode` already
  returns) **plus the records produced before the error**; `Ok` carries the result and its records.
  The host maps the error through the existing `dom_api_error_to_vm_error` (`vm/host/dom_bridge.rs:390-429`) and
  commits the records (M2); it decides nothing and adds no second mapper. `RangePointError` (`range/boundary.rs:12-19`:
  `WrongDocument` / `InvalidNodeType` / `IndexSize`, mapped separately in `vm/host/range_proto.rs:820-840`) is
  **folded into `DomApiError`** in RC-1 (one issue, one way), unless RC-1's per-PR plan names a caller that needs a
  distinct shape, and the plan records the outcome. **The fold covers every Range DOMException the host decides
  today**, not only `RangePointError`'s three. In `vm/host/range_proto.rs` these are `reject_doctype` (`:788-811`,
  InvalidNodeTypeError for a doctype boundary node), `validate_boundary_node_and_offset` (`:488-513`, IndexSizeError at
  `:505`), `require_attached_node` (`:519-539`, InvalidNodeTypeError at `:531` for a parentless node: setStartBefore
  / setStartAfter / setEndBefore / setEndAfter / selectNode) and compareBoundaryPoints (`:676-733`: the `how` check,
  NotSupportedError at `:701`, and the different-roots WrongDocumentError at `:725`). Each is a step of a DOM §5.5
  algorithm, so it belongs to the engine method, which returns `DomApiError` (X1; one mapper). RC-1 edits that file
  anyway (the `RangePointError` mapper), so the fold is one PR. What stays in the host is VM state, not spec steps:
  `detached_range_error` (`:303-308`, a Range used after `Vm::unbind`) and the receiver / argument brand checks. The
  fold needs two kinds the enum lacks (§0.2). So **RC-1** adds
  `WrongDocumentError` and `InvalidNodeTypeError` to `DomApiErrorKind` (`types.rs:195-219`, `#[non_exhaustive]`).
  It adds their `dom_api_error_to_vm_error` arms, mapped to `wk.dom_exc_wrong_document_error` /
  `wk.dom_exc_invalid_node_type_error` (the names `range_proto.rs:820-840` uses), because without an arm a kind maps to
  `VmError::internal` (`dom_bridge.rs:427`). It also widens the mapper's visibility so `range_proto.rs` can call it
  (today a private `fn`). `IndexSize` maps to the existing `IndexSizeError`. RC-3 reuses `InvalidNodeTypeError` for
  surroundContents step 2 and adds no kind.
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
  argument conversion, live-range-reference lookup, error mapping, record commit, wrapper creation, and for RC-4 the
  realm-document read through N4's J12 route (`vm/host/settings_route.rs`, §5.4). No clone pass, loop or walker in
  `vm/host/`: both clone passes run in their own crates' consumers (C2).
- **X2 (ECS-native)**: no new component and no new entity-keyed side-store. `LiveRangeRef` is a handle into the
  existing `LiveRangeRegistry` map (Range objects are not entities). Clones use the existing ECS cloner. Clone
  creation is event data on the existing `MutationEvent` stream (a variant, the idiom `mutation_event.rs:8-15`
  states), consumed at dispatch by two existing consumers, which write existing components and push onto the existing
  reaction queue (the same idiom as `handle_insert` and C0's `Adopt` arm). It adds no subscriber, no registry, no
  reaction variant and no per-entity state. The `ClonedFrom` marker stays as today, the form pass's only.

Pairs (one line each; pairs not listed do not interact):

| Pair | Intersection |
|---|---|
| B × M | the enumeration order is the record order; nodesToRemove omits nested nodes, so one removal record per top-level node |
| B × L | containment is computed once from the originals captured before the first mutation, never re-evaluated against the tree the algorithm is mutating |
| M × L | the target range is set (L1) before the mutations, so each primitive's hook moves it exactly as the spec's live-range rules do |
| B × C | a partially contained child gets a shallow clone plus a recursive subrange; a contained child gets a deep clone (clone) or a move (extract) |
| M × C | clones are appended through M1 into new containers (records with unobserved targets); each clone root's `CloneCreated` event fires between the mutations, in step order, so its upgrades sit among the `Disconnected` / `Connected` reactions those mutations enqueue (C2); the walks at dispatch visit only the created set, because the algorithm's appends into a root come after its event (§0.3 item 26) |
| L × C | cloning touches no pre-existing node, so it fires no live-range update on existing ranges; the bridge ignores `CloneCreated`; the ranges mutex is not held across the cloner entry (L4) |
| E × M | a late error (surround after extract) commits the records produced so far (M2); an early error produces none (E1) |
| E × L | early error: range unchanged; late error: the range keeps the state the spec steps gave it (no rollback) |
| C × E | step 11 runs before the first clone, and a recursive subrange inside an element cannot contain a doctype, so no clone is created before a doctype throw. C2's upgrade reactions are reactions under E2: they are enqueued at their creation step, so a late error (surround) leaves the upgrades of the clones it made in the queue, as the spec's queue would hold them; no native step is needed on the error path |
| B × E | the PI-boundary refusal and step 11's doctype test read the partition B1 computed from the originals, before the first mutation (E1) |
| B × P | none: createContextualFragment reads only the start node (HTML §8.5.7 steps 3-6), never the containment partition, and mutates nothing that exists (P4) |
| X × M | the engine produces the records and the host only commits them through `commit_notify_records` (M2); no record is built in `vm/host/` |
| X × C | the event and both passes are engine-independent (`elidex-ecs` variant, `elidex-dom-api` emitter, `elidex-custom-elements` and `elidex-form` consumers); `vm/host/` has no clone step at all (X1) |
| P × M | parsed nodes go into a new fragment: no record (P4) |
| P × L | createContextualFragment mutates nothing that exists, so no live-range update |
| P × C | parsed elements get their custom-element state from the parser's own derivation (as innerHTML), not from C2 |
| X × E | the engine returns `DomApiError` plus the records; the host only maps (through `dom_api_error_to_vm_error`) and commits (E3), so no error decision sits in `vm/host/` |
| X × P | RC-4's parse goes through `elidex-script-session` (P1); the `Fragment` mode value lives in the parser crates with N9's enum, and the VM native only marshals |
| E × P | createContextualFragment mutates nothing that exists, so any error leaves no records (P4). Its possible throws are a Trusted Types failure (step 2; no Trusted Types yet, §8) and the XML fragment parse of the fragment parsing algorithm steps' XML branch (unreachable while elidex has no XML parsing, §5.4 (b)); the HTML fragment parse does not throw |
| X × L | the VM obtains `LiveRangeRef` from HostData's `live_range_registry` field, not through `split_dom_and_live_ranges` (§4.2), and passes it to the engine with `host.dom()`; the engine reads and writes the registered range, the host never writes a boundary |

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
| WHATWG DOM §5.5 Interface Range | select a node | parent null throws | the throw is decided in the host today (`require_attached_node`, `vm/host/range_proto.rs:519-539`), not in `Range::select_node` (`range/boundary.rs:125`); RC-1 folds it into the engine (E3); RC-3 consumes it | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | ensure pre-insert validity; pre-insert; insert; append; remove; replace all | per-step rejection | `ensure_pre_insertion_validity` (`child_node/mod.rs:251`); session `apply_*` | ✓ | yes |
| WHATWG DOM §4.3.2 Queuing a mutation record | queue a mutation record | childList / characterData | `commit_notify_records` (unchanged) | ✓ | no |
| WHATWG DOM §4.4 Interface Node | clone a node (step 3 "Run any cloning steps"); clone a single node (step 8 node document) | shallow / deep; shadow host; template; custom element | `clone_node_with_shadow_honor` (`node_methods/clone.rs:227`); step 6.7 subtree true (D14, RC-2); step 2.3's registry rewrite for a Document clone (D15, RC-2); one `CloneCreated` event per entry call, scope from the `subtree` flag, handled at dispatch (§4.3, D-g′); node document of clones: Layout (C3) | ✓ | yes |
| WHATWG HTML §4.13.6 Custom element reactions | `[CEReactions]` steps 1-4 (push an element queue; run the steps; pop; invoke); enqueue an element on the appropriate element queue; invoke custom element reactions (per-element reaction queues) | queue order / element-queue grouping / drain point | enqueue points: C2's dispatch-time CE pass + the consumer (RC-2), per-slice rule (E2); grouping: `#11-ce-element-queue-grouping` (registered, §9); drain point: proposed `#11-ce-reactions-method-scope-drain` (§9) | ✗ (grouping and drain point out of scope, E2) | no |
| WHATWG HTML §4.10.5 The input element / §4.10.11 The textarea element | cloning steps (reached via DOM §4.4 clone a node step 3) | input / textarea | `ClonedFrom` + `elidex_form::apply_clone_form_state` (input changes to marked entities, RC-2) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | cloning steps (contents cloned with the copy's template contents' node document) | subtree false / true | `replicate_template_contents` (unchanged); outside the CE pass (§4.3) | ✓ | yes |
| WHATWG HTML §4.13.3 Core concepts | look up a custom element definition (step 1: null registry → null) | — | CE pass scope over template contents (§4.3 condition 6) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | a document's custom element registry (null unless stated otherwise); effective global custom element registry (null for a non-global registry) | inert template document | CE pass scope over template contents (§4.3 condition 6) | ✓ | no |
| WHATWG DOM §4.9 Interface Element | create an element (steps 4.4 and 5.2.2: enqueue the upgrade reaction; step 5.2.1: an async autonomous element is created with a null is value) | `body` context for RC-4; clone copies (reached via "clone a single node" step 2.4) | element creation helper (RC-4); RC-2's dispatch-time CE pass, `apply_clone_creation_ce_semantics` (§4.3 condition 2) | ✓ | no |
| WHATWG DOM §4.10 Interface CharacterData | replace data; substring data | Text / Comment / CDATASection | `apply_replace_data` (unchanged); substring in the contents algorithm | ✓ | yes |
| WHATWG DOM §4.11 Interface Text | split a Text node | insert step 7 | `split_text_at_offset` (unchanged) | ✓ | yes |
| WHATWG HTML §8.5.7 The createContextualFragment() method | steps 1-7 | Element / Text-Comment with parent element / null / `html` in an HTML document | `create_contextual_fragment` (RC-4) | ✓ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | fragment parsing algorithm steps | Legacy mode; runScripts; XML document | session fragment parse (RC-4) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm | context = target; fragment node document; scripting mode | `parse_fragment_progressive` (unchanged) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | HTML fragment parsing algorithm step 8 (Disabled when contextDocument's scripting is disabled) | Fragment / Disabled | N9's fragment entry derives Disabled; RC-4 passes Fragment (§5.4) | ✓ | no |
| WHATWG HTML §13.2.4.5 Other parsing state flags | parser scripting mode | Normal / Disabled / Inert / Fragment; Normal is unreachable on the fragment path (HTML fragment parsing algorithm step 1 asserts Inert or Fragment, and step 8 can only set Disabled) | N9's `ParserScriptingMode` (Normal, Disabled, Inert; F11); RC-4 adds `Fragment` | ✓ | no |
| WHATWG HTML §13.2.6.4.4 The "in head" insertion mode | script start tag steps 2-3 (Fragment: no parser document; Inert: already started = true) | Inert / Fragment | none in elidex: neither backend keeps a parser document or "already started" (§5.4), so RC-4 adds the enum value and its tree-construction mapping only; the script-element half is proposed `#11-script-element-prepare-lifecycle` (§8) | ✗ | no |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | create an element for a token steps 13, 15 (fragment parse: willExecuteScript false → DOM §4.9 create an element enqueues the upgrade) | definition null / non-null | question (d), §5.4 | ✗ | no |
| WHATWG HTML §4.12.1.1 Processing model | already started | Fragment mode leaves it unset | question (a), §5.4 | ✗ | no |
| Selection API §3 Selection interface | deleteFromDocument | rangeCount 0 / 1 | `Selection::delete_from_document` (`selection.rs:513`, RC-1) | ✗ (the step text is unconfirmed: `webref body selection-api dom-selection-deletefromdocument` finds no anchor, so the branch list is from the IDL and the code, not the steps) | yes |

Numbers and titles: `webref heading dom 5.2`, `5.5`, `4.2.3`, `4.3.2` (via `coverage-map`), `4.4`, `4.5`, `4.9`, `4.10`,
`4.11`; `webref heading html 8.5.4`, `8.5.7`, `13.4`, `4.10.5`, `4.10.11`, `4.12.3`, `4.13.3`, `13.2.6.1`,
`13.2.6.4.4`, `4.12.1.1`, `4.13.6`; `webref heading selection-api 3` (§3 Selection interface; `heading selection-api 3.1` /
`3.2` / `3.3` → "no headings under §3.x", so sub-numbers are unconfirmed and not cited; `body selection-api
dom-selection-deletefromdocument` → anchor not in the rendered chapter, so its step text is unconfirmed); `webref coverage-map dom 5.5 dom 5.2 dom 4.2.3 dom 4.3.2 dom 4.4
dom 4.9 dom 4.10 dom 4.11 html 8.5.7 html 8.5.4 html 13.4 html 13.2.4.5 html 4.12.1.1`. Dfns: `webref dfn dom
contained` / `'partially contained'` / `'get the common ancestor'` (§5.5), `'clone a node'` / `'clone a single node'`
(§4.4), `'replace data'` / `'substring data'` (§4.10), `'split a Text node'` (§4.11), `'ensure pre-insert validity'` /
`'replace all'` / `remove` / `append` (§4.2.3), `select` (§5.5), `'create an element'` (§4.9), `'custom element
registry'` (for=Document, §4.5), `'effective global custom element registry'` (§4.5); `webref dfn html
'fragment parsing algorithm steps'` (§8.5.4), `'parser scripting mode'` (§13.2.4.5), `'already started'`
(§4.12.1.1), `'create an element for a token'` (§13.2.6.1), `'look up a custom element definition'` (§4.13.3),
`'appropriate template contents owner document'` (§4.12.3), `CEReactions` (§4.13.6); `webref dfn dom 'cloning steps'` (§4.4), `'create a document'` (§4.5); `webref dfn
selection-api deleteFromDocument` (§3). Prose: `webref body dom concept-range-extract`, `concept-range-clone`, `dom-range-deletecontents`,
`concept-range-insert` (also holds surroundContents), `dom-range-stringifier`, `contained`, `concept-range-select`,
`get-the-common-ancestor`; `webref body html dom-range-createcontextualfragment`, `fragment-parsing-algorithm-steps`,
`html-fragment-parsing-algorithm` (step 8), `parser-scripting-mode`, `parsing-main-inhead` (script start tag
steps 2-3), `create-an-element-for-the-token` (steps 13, 15), `the-template-element` (cloning steps), `look-up-a-custom-element-definition`;
`webref body dom concept-node-clone` (clone a node steps 2, 3, 5, 6.7), `clone-a-single-node` (steps 2.1-2.4, 7, 8),
`concept-create-element` (steps 4.4, 5.2.1, 5.2.2), `create-a-document`,
`document-custom-element-registry`, `effective-global-custom-element-registry`; `webref body html
appropriate-template-contents-owner-document`, `cereactions`, `custom-element-reactions-stack`,
`enqueue-an-element-on-the-appropriate-element-queue`, `enqueue-a-custom-element-upgrade-reaction`,
`invoke-custom-element-reactions` (step 1.3), `concept-upgrade-an-element` (step 4, attributeChangedCallback for
each attribute), `parsing-main-inhead` (the two `noscript` start tag rows); `webref dfn html 'element queue'` (§4.13.6)
(2026-11-01). Breadth: K = 3
(DOM, HTML, Selection API), M = 29. M ≥ 20 is the split signal, and this memo is the split: each per-PR plan carries only its own rows.

Step numbers are cited where `webref body` shows them today; per-PR plans and code comments cite steps **by name**
where the number has moved before (extract's partially-contained-child steps, §0.3 item 1).

## §4. Architecture

### §4.1 Layering map

| Concern | Home | Note |
|---|---|---|
| Containment, toString, delete, contents (clone/extract), insert, surround, context resolution | `elidex-dom-api` `range/` | engine-independent |
| Live-range reference | `elidex-dom-api` `range/live.rs` | existing registry, new handle |
| Record-producing primitives, fragment parse into a new fragment | `elidex-script-session` `mutation/` (`apply_*`); `split_text_at_offset` is in `elidex-dom-api` (`char_data/split_text.rs:105`) | existing primitives; `html_fragment.rs` gains one function in RC-4 |
| `ParserScriptingMode::Fragment` (NEW) (the enum itself is N9's) and its tree-construction mapping (the script start tag steps 2-3 are unobservable in elidex, proposed `#11-script-element-prepare-lifecycle`) | `elidex-html-parser-strict` (N9's enum, F11) and `elidex-html-parser` (html5ever backend) | RC-4 (§5.4) |
| Clone-a-node | `elidex-dom-api` `node_methods/clone.rs` | the canonical cloner; pass 2 keeps today's `ClonedFrom` gate (`<input>` / `<textarea>`); D14 is fixed here (RC-2). The node document of clones is Layout's (C3) |
| Clone-created event | `elidex-ecs` `dom/mutation_event.rs`: `MutationEvent::CloneCreated { root, scope }` and `CloneScope` (NEW); `EcsDom::fire_clone_created` (NEW, beside `fire_split_text`); fired by the canonical cloner's entry in `elidex-dom-api` (§4.3) | engine-independent; no caller can skip it (§4.3 condition 1). R3's Ax1 finding that `cloneNode`'s root is built in `vm/host/` is moot, because the root now comes from the engine's event |
| CE clone-creation pass; form-state pass (both at dispatch) | `elidex-custom-elements`: the `CloneCreated` arm of `CustomElementReactionConsumer` and `apply_clone_creation_ce_semantics` (root + scope); `elidex-form`: the `CloneCreated` arm of `FormControlReconciler` calling today's `apply_clone_form_state` | each walks in its own crate (RC-2, §4.3) |
| Error contract | `elidex-dom-api` engine returns `DomApiError` + records produced before the error (E3) | the host maps through `dom_api_error_to_vm_error` and commits only |
| Brand check, argument conversion, live-range-reference lookup, error mapping, record commit, wrapper; RC-4's realm-document read through N4's J12 route | `vm/host/range_proto_mutation.rs`; `vm/host/settings_route.rs` (NEW in N4) | marshalling only (X1); no `with_range_mut`, no transient `Range::new` carrier (RC-1 DoD); no clone step (C2) |

### §4.2 Live-range reference (RC-1)

`LiveRangeRegistry` already shares its map with the bridge through `Arc<Mutex<HashMap<RangeId, Range>>>`
(`range/live.rs:336-338`). RC-1 adds an owned handle, `LiveRangeRef` (NEW) = (that `Arc`, `RangeId`), obtained from the
registry, with a read (applying the dangling-collapse rule of `finalize_pending` to its own entry) and a boundary write.
The engine methods take `(&LiveRangeRef, &mut EcsDom)`. **RC-1 does not use `split_dom_and_live_ranges`**: that
accessor hands out a shared `&EcsDom` (§0.2), and the engine needs `&mut`. The VM reads the handle from the
`pub(crate)` `live_range_registry` field, the access `range_proto.rs:288` already uses, and then calls the engine with
`host.dom()`. No HostData accessor is added or changed. RC-1's two uses of `split_dom_and_live_ranges` in
`range_proto_mutation.rs` (`:49` in `commit_range_after_mutation`, `:85` in the delete native's snapshot) go with the
snapshot-then-commit path; the readers in `range_proto.rs` keep
it. RC-1 still lands after PR #578 (PM 207). The `split_dom_*` projections move to `host_data/engine_feature/bind.rs`
there, and `host_data/mod.rs` drops to 729 lines (§0.2). RC-1 edits neither file; if its per-PR plan finds it must, it
edits the post-#578 file. `delete_from_document` (`selection.rs:513`) takes the same path. `commit_range_after_mutation`
(`range_proto_mutation.rs:34-66`) and the Selection write-back (`selection.rs:537-542`) are deleted.

Not chosen: keep snapshot-then-overwrite and prove the final write equal to the spec's. The proof holds for
delete and extract (every later mutation is inside a partially contained child or at an index at or after newOffset),
but not for insert (D12) or surround (replace-all in step 4 can move the live range before step 5 reads it), so the
two patterns would remain.

Unit tests in `elidex-dom-api` register their range in a registry wired through the existing
`make_range_only_test_dispatcher` (`range/mod.rs:21`).

### §4.3 Contents algorithm and the clone passes (RC-2)

One recursive function implements `concept-range-extract` and `concept-range-clone` with a mode (Clone | Extract). The
modes differ in exactly three places, all named in the spec: the same-CharacterData case (extract also replaces data),
the partial CharacterData cases (extract also replaces data), and contained children (clone: deep clone; extract:
move). Extract alone sets the range (step 15). The partition (first / last partially contained child, contained
children) comes from B1. Before the first mutation it checks both boundaries (PI refusal, E1) and step 11 (doctype).

**Output (E3).** `Ok`: the fragment and the records. `Err`: a `DomApiError { kind, message }` and the records
produced before the error. The records are empty for every error this algorithm raises, since all of them precede the
first mutation; surround's late errors are RC-3's. The host maps the error through `dom_api_error_to_vm_error` and
commits. The algorithm returns no clone roots: they travel as events (below).

**Created set and reaction order (driver decision D-g′; replaces Rev 4's D-g placeholder, D-a′'s returned list, D-a
and the R2 `ClonedFrom` widening).** Two passes follow a clone, and they act on different sets, so they stay two
passes. The **form pass** implements the HTML cloning steps of `<input>` / `<textarea>` (§3), so it acts only on those
clones, found by today's `ClonedFrom` marker. The **CE pass** implements the custom-element half of "create an
element" for every element created by cloning ("clone a single node" step 2.4, `webref body dom clone-a-single-node`;
"create an element" steps 4.4, 5.2.1-5.2.2, `webref body dom concept-create-element`). It needs every created element,
nothing else, and each element's upgrade must enter the reaction queue at the point of the clone, in spec step order,
among the `Disconnected` / `Connected` reactions of the same call (§0.3 item 18). Both run **at dispatch time** of one
event, in the consumer of their own crate. Conditions, each a requirement on RC-2:

1. **The event and its emitter.** `elidex-ecs` `dom/mutation_event.rs` gains `MutationEvent::CloneCreated { root:
   Entity, scope: CloneScope }` and `pub enum CloneScope { Subtree, SelfOnly }` (NEW, re-exported beside
   `MutationEvent`, `dom/mod.rs:32`, `lib.rs:45`). `EcsDom` gains `pub fn fire_clone_created(root, scope)` (NEW), the
   sibling of `fire_split_text` (`dom/mod.rs:231`). The **canonical cloner's entry** fires it:
   `clone_node_with_shadow_honor` (`node_methods/clone.rs:227-305`) calls it once, after its worklist converges and
   before it returns `Some(cloned)`, with `scope = Subtree` when `deep` and `SelfOnly` otherwise. Its internal helpers
   (`clone_recording`, `replicate_template_contents`, the shadow-replication loop) never fire it. So every call of
   the entry fires exactly once, and the shadow-tree and template-content copies made inside it are covered by their
   root's event, not by events of their own.
   - **Why the entry, not the contents algorithm.** Every production clone goes through this entry (`CloneNode`
     today, the contents algorithm after RC-2: `git grep -n 'clone_node_with_shadow_honor' -- crates ':!*tests*'`).
     A caller cannot forget the event, so the R3 Ax2 risk of a missed root failing open is closed by construction.
     The contents algorithm calls the entry once per clone root, in spec order (§0.3 item 17), so the events fire in
     that order.
   - **Scope from the flag.** A contained child's deep clone (clone step 14.1) is `Subtree`. A partially contained
     child's clone (clone steps 13.1 / 16.1, extract steps 17.1 / 20.1) is shallow, so `SelfOnly`. CharacterData
     clones are shallow too; both passes find nothing in them. `cloneNode(true)` is `Subtree`. `cloneNode(false)` is
     `SelfOnly`, which visits the same set as `Subtree` would on a shallow copy (§0.3 item 25).
   - **`cloneNode` behaviour.** It is today's for both flag values (§0.3 item 29): the same walk, order, cap and
     root, and the same queue position. Only D14 changes the copy for `cloneNode(false)` on a clonable shadow host:
     the shadow children are now deep copies, so their descendants exist and get their upgrades. That is D14's
     intended change, not D-g′'s. D15 changes `document.cloneNode(true)`'s registry association (§0.4).
   - **No crate cycle.** The variant and `CloneScope` live in `elidex-ecs`, the bottom crate. `elidex-dom-api` (the
     emitter), `elidex-custom-elements` and `elidex-form` (the consumers) all already depend on it
     (`elidex-dom-api/Cargo.toml:15`; `elidex-custom-elements/Cargo.toml`, `[dependencies]` = `elidex-ecs`, `hecs`;
     `elidex-form/Cargo.toml:15`).
   - **Other consumers.** The variant is a data addition. Every `match` on `MutationEvent` that has no wildcard
     gains a no-op arm; the per-PR plan lists them (`git grep -l 'MutationEvent::' -- crates ':!*tests*'` = 31 files at
     `ee6c7a1a`). The live-range bridge ignores the event (L × C). C0's `Adopt` touches the same enum, `match` set and
     census; whichever of the two lands second re-runs the census and adds its arms (§9).
2. **The CE arm, at dispatch.** `CustomElementReactionConsumer::handle` (`consumer.rs:75-96`) gains an arm for
   `CloneCreated { root, scope }`. Like `handle_insert` (`:98-123`), it locks the registry, runs
   `apply_clone_creation_ce_semantics(dom, &registry, root, scope)`, drops the guard, then locks the queue and pushes
   one `CustomElementReaction::Upgrade(entity)` per returned candidate, in order. This is the same idiom as today's
   `handle_insert` and C0's `Adopt` arm: a consumer that derives reactions from the event when it fires. So the
   upgrades sit in the flat FIFO at the clone's position: for extract step 17.1 they come before the `Disconnected`
   of the moves at step 17.4 (§0.3 item 18). No reaction variant is added, and `invoke_one` (`flush.rs:100-140`) is
   unchanged. The arm needs `&mut EcsDom` for the `is_value` clear, which `handle` already receives (§0.2). It writes
   only `CustomElementState` and reads the tree, so it is allowed under the re-entry contract
   (`mutation_event.rs:25-31`; §0.3 item 27). It takes the two locks `handle_insert` takes, in the same order, and no
   other (§0.3 item 28).
   - **The registry it reads is the clone-time registry**, by construction: the lookup runs inside the clone.
     Rev 4's argument that `define` cannot run before resolution is no longer needed.
3. **The form arm, at dispatch.** `FormControlReconciler::handle` (`elidex-form/src/reconciler.rs:41-52`) gains an
   arm for `CloneCreated { root, .. }` that calls `apply_clone_form_state(dom, root)` (`elidex-form/src/clone.rs:44-63`,
   unchanged). It writes `FormControlState`, `ElementState` and removes `ClonedFrom`, all components, through
   `create_form_control_state`, the function `handle_insert` already calls at dispatch (`reconciler.rs:84-95`;
   §0.3 item 27). It runs synchronously inside the clone, so a detached clone reads its copied state before the entry
   returns (I3, below). The scope is not needed: at a SelfOnly root's event its walk already sees only the root, its
   shadow tree and its template contents (§0.3 item 26).
   - **Order between the two arms.** The dispatcher calls `form_control` before `custom_elements` (§0.2), where
     today's shim ran CE first. The two touch disjoint components and neither reads the other's, so the swap is
     unobservable (§0.3 item 29).
4. **The walk at dispatch (D-a″).** `apply_clone_creation_ce_semantics` takes `scope: CloneScope` (from
   `elidex-ecs`). `Subtree` runs today's `for_each_shadow_inclusive_descendant` walk from the root, unchanged in order
   and cap. `SelfOnly` visits the root, then today's walk over the root's shadow root if it has one, never its light
   children (§0.3 item 17). A scope argument is chosen over a sibling function so that both visit sets share one walk
   in one function.
   - **The visit set at the event is the created set, by construction.** A `Subtree` root's event fires after the
     entry's worklist, so its whole subtree exists, and nothing the call does later adds a node under it
     (contained children's deep clones are appended to the fragment whole; extract has no `Subtree` roots). A
     `SelfOnly` root has no light children when its event fires: the algorithm appends the subfragment only after the
     entry returns (§0.3 item 26). So the light-children exclusion in D-a″ is also what the tree gives at that moment;
     it stays in the function so that the contract does not depend on the caller's timing.
   - **The contract** "MUST only be called on a freshly cloned subtree" (`upgrade.rs:121-123`) becomes "on the root of
     a `CloneCreated` event, with that event's scope, during its dispatch".
   - **Depth cap.** The cap applies per root, from depth 0, as `cloneNode`'s does today (§0.3 item 17).
5. **Form pass and marker: today's, per root.** Pass 2 (`mark_cloned_form_control`, `node_methods/clone.rs:368-387`)
   attaches `ClonedFrom` only to `<input>` / `<textarea>` clones, "so a large `cloneNode(true)` does not spray a
   marker onto every cloned node" (`:373-374`). `elidex_form::apply_clone_form_state(dom, clone_root)` reads and
   removes every marker over its root's template-inclusive descendants, unconditionally (`elidex-form/src/clone.rs:44-63`;
   one `for_each_shadow_inclusive_descendant` walk per frontier root, `elidex-form/src/init.rs:111-126`).
   - **Today's contract, kept.** One call per clone root, synchronously with the clone, with the cap counted from
     that root. The form arm makes that call once per event, so once per root, in creation order. Rev 3's single call
     on the fragment stays withdrawn: it counted the cap from the fragment, which is not `cloneNode`'s contract.
   - **No re-visit, no moved original.** Rev 4 ran the pass after the algorithm, so a `SelfOnly` root's walk also
     passed its later light children (subrange clones, or moved originals in extract) and relied on "the marker is
     removed on first use". Under D-g′ the walk runs before those children exist (§0.3 item 26), so it never meets a
     subrange clone (each is swept by its own event) or a moved original. That residual is gone. The one residual
     left is today's: a marker deeper than the cap below its root is not swept. That is the same for `cloneNode`,
     and its doc already calls a left-behind marker harmless (`node_methods/clone.rs:60-64`).
   - R2 Ax3-MIN9 (a census ratchet for the widened marker) stays moot: the CE pass reads no marker.
6. **CE-pass scope over template contents.** The CE pass visits each root's shadow-inclusive descendants and **not**
   template-content clones (today's walk, `for_each_shadow_inclusive_descendant`, `upgrade.rs:131`, does not reach
   template contents). Spec basis:
   - Template cloning steps step 2 clone the contents "with document set to copy's template contents's node
     document" (`webref body html the-template-element`). That document is the inert template document,
     created by "create a document" with a null browsing context (appropriate template contents owner document
     step 1.1.1; DOM §4.5 "create a document", `webref body dom create-a-document`).
   - Its custom element registry **is null**: "create a document" sets none, and a document's custom element
     registry is null unless stated otherwise (DOM §4.5, `webref body dom document-custom-element-registry`).
   - The "clone a single node" registry steps 2.1-2.3 (`webref body dom clone-a-single-node`) then give the content
     clone a null registry. A null registry stays null. A global one becomes the inert document's registry's
     effective global custom element registry, which is null because that registry is null and so not global (DOM
     §4.5, `webref body dom effective-global-custom-element-registry`). Look up a custom element definition step 1
     returns null for a null registry (`webref body html look-up-a-custom-element-definition`).
   - **Scoped-registry caveat.** Step 2.3 rewrites only a *global* registry. A scoped (non-global) registry passes
     through unchanged, so a content clone whose source has a scoped registry would keep it. elidex has no scoped
     registries; that is `#11-shadow-scoped-custom-element-registry` (deferred, M4-14). When it lands, this rule is
     re-checked there.
   - The rule keeps `cloneNode` behaviour unchanged, and a test pins it (a cloned `<template>` whose contents hold a
     defined custom element enqueues no upgrade for the content clone). C1 (`#11-template-contents-owner-document`)
     changes those contents' node document; the rule does not depend on it.

One clone path serves `cloneNode` and the Range natives: the entry fires, and the two consumers run their passes at
dispatch. No native has a clone step. `apply_clone_form_state` is unchanged. The VM `cloneNode` post-pass
(`node_methods_extras.rs:214-232`: `enqueue_subtree_upgrade_reactions` and the `apply_clone_form_state` call) is
deleted, and `enqueue_subtree_upgrade_reactions` (`vm/host/custom_elements/upgrade.rs:128`) with it, so the separate
post-clone seam is retired (one issue, one way).

**Removed from Rev 4 (D-g′).** The `CloneCandidates` reaction variant; `resolve_clone_placeholders`; the VM seam and
its four-native caller list; the `invoke_one` fail-safe arm; and the seam census in §5.2's DoD. Moot with them: the
"no clone is connected before resolution" invariant, the registry-at-drain question, and the form pass riding the CE
queue. The `ConsumerDispatcher` field docs (`consumer_dispatcher.rs:52-57`, `:78-87`) are rewritten (§6), because
`custom_elements` now also writes `CustomElementState` and `form_control` now also runs the cloning steps.

**#466 reversal (a design change to a ratified surface; reviewed in `/external-converge` R1, PM 222).** #466's plan
(`docs/plans/2026-07-clone-cloning-steps-fcs.md` §2, the slot `#11-clone-cloning-steps-event`) fixed five invariants
for the form cloning steps. Two of them excluded an event:

- I2 (`:53`): "**Lane boundary** — no B1-core touch (`crates/core/elidex-ecs/src/dom/` off-limits)", forcing "No
  `MutationEvent::Clone` variant (that enum is B1 core) → the seam is NOT a mutation event."
- I3 (`:54`): "`detachedInput.cloneNode().value` must read correctly *before* any insertion", forcing "The FCS copy
  MUST be **synchronous at clone time**, not a deferred event consumed on insertion."
- And the conclusion (`:73-79`): the #331 §6.1 sketch's `MutationEvent::Clone { pairs }` "with a deferred per-layer
  consumer" is rejected because "I2 rules out the event; I3 rules out 'deferred'".

Why neither binds D-g′:

- **I2 was a lane boundary, not an engine invariant.** It recorded which crates L3 could edit when #466 was planned.
  It said nothing against an event on its merits. The enum's own design invites the addition: "new mutation kinds
  are enum variant additions" (`mutation_event.rs:8-12`). The boundary no longer holds for this work. Layout's C0
  adds `Adopt` to the same enum, and RC-2 adds `CloneCreated`; both lanes now edit `EcsDom`, and §9 states how the two
  edits meet. What I2 protected in substance is I1, the crate boundary (`elidex-dom-api` cannot see
  `FormControlState`), and I1 still holds: `elidex-dom-api` fires through `elidex-ecs`, and the copy stays in
  `elidex-form`, reached through the dispatcher. No `elidex-dom-api` → `elidex-form` edge is added.
- **I3 is met, not broken.** I3 excludes a *deferred* consumer, "consumed on insertion". D-g′'s consumers are not
  deferred. Dispatch is synchronous ("Mutation events fired synchronously", `mutation_event.rs:1`; the
  take-and-restore call `EcsDom::dispatch_event`, `dom/mod.rs:199-220`), and the event fires inside `clone_node_with_shadow_honor`,
  before it returns (§0.3 item 26). So the clone's `FormControlState` exists before the cloner entry returns. That is
  earlier than #466's shim, which ran after `invoke_dom_api`. A detached clone reads correctly before any insertion.
- **I4 and I5 hold unchanged**: the same function creates the FCS at clone time, so the insert-time absence guard
  still skips the clone (I4), and the walk covers the whole shadow-inclusive, template-inclusive clone (I5), on the
  same tree (§0.3 item 29).
- **The linkage stays #466's** (its I1 × I2 row): the `ClonedFrom` marker, data owned by `elidex-dom-api`. The event
  carries only the root and the scope, never the pairs.
- **The slot**: `#11-clone-cloning-steps-event` was closed by #466 (`7bafcb9f`) with the event-less seam;
  `tree_clone.rs:28` says "there is no `MutationEvent::Clone`". The defect stays fixed, so the closure stands and the
  slot is not reopened. The doc row is rewritten (§6), and the PM may note on the closed entry that the mechanism is
  now the event (PM list).

deleteContents stays its own algorithm (the spec's is separate and clones nothing); it shares B1 and the newNode /
newOffset helper with extract.

### §4.4 ECS-native check

| Spec / OO notion | elidex form |
|---|---|
| live range object updated by mutation observers inside the engine | existing `LiveRangeRegistry` map + `LiveRangeBridge` driven by `MutationDispatcher` events; `LiveRangeRef` is a handle, not a new registry |
| "clone a node" with per-class cloning steps | ECS cloner + post-pass worklist over (src, dst) pairs (existing); the form cloning steps keep the existing `ClonedFrom` marker on `<input>` / `<textarea>` clones; clone creation is a `MutationEvent` variant (event-as-data on the existing stream), not an observer callback |
| subrange objects | plain values on the stack (L2) |
| `[CEReactions]` element queues and per-element reaction queues | one existing flat reaction queue fed by the mutation-event leg; clone upgrades enter it at dispatch of `CloneCreated` (C2); the per-element grouping is not modelled (`#11-ce-element-queue-grouping`, registered) and neither is the per-method element-queue stack (proposed `#11-ce-reactions-method-scope-drain`) (E2, §9) |
| "create an element" run for each clone copy | an event-driven consumer pass over the clone root (event-as-data, the `handle_insert` / C0 `Adopt` idiom), not a callback the cloner calls |

No component, system or side-store is added. One event variant is added, and no reaction variant. The existing `ClonedFrom` marker is unchanged (§4.3 condition 5).

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
  are deleted. **Extract's interim boundary path**: RC-1 deletes `commit_range_after_mutation`, which extract calls
  today (`range_proto_mutation.rs:125`, §0.3 item 20). So RC-1 moves extract's final boundary write onto
  `LiveRangeRef` as well, keeping today's value `(start_container, start_offset)` (`range/mutation.rs:399-400`).
  The D5 extract fix (`(newNode, newOffset)`) stays in RC-2, which replaces the body. After RC-1, no Range method
  goes through snapshot-then-commit. `delete_from_document` (`selection.rs:513`) on `LiveRangeRef`. Error contract
  E3 for insertNode (HierarchyRequestError exists). The `RangePointError` fold, with the two new
  `DomApiErrorKind` variants `WrongDocumentError` / `InvalidNodeTypeError`, their `dom_api_error_to_vm_error` arms
  and the mapper's visibility (E3); RC-3 reuses `InvalidNodeTypeError`. The host-side Range DOMException decisions
  of `range_proto.rs` fold with it (`reject_doctype`, `validate_boundary_node_and_offset`'s offset check,
  `require_attached_node`, compareBoundaryPoints' `how` and root checks; E3): the engine methods throw them, and the
  natives map.
- **Order**: after RC-0a and PR #578 (§4.2).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`, `crates/dom/elidex-dom-api/src/range/live.rs`, `crates/dom/elidex-dom-api/src/range/containment.rs` (NEW),
  `crates/dom/elidex-dom-api/src/selection.rs`, `crates/dom/elidex-dom-api/src/range/boundary.rs` (`RangePointError`
  fold), `crates/script/elidex-script-session/src/types.rs` (two `DomApiErrorKind` variants),
  `crates/script/elidex-js/src/vm/host/dom_bridge.rs` (two mapper arms, visibility),
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, `crates/script/elidex-js/src/vm/host/range_proto.rs`
  (the `RangePointError` mapper at `:820-840` replaced; the host-side error decisions folded, E3; the toString native
  only if its signature changes), tests.
- **Tests**: unit: toString with Element start (children before `start_offset` excluded) and Element end
  (descendants before `end_offset` included), CDATASection (Rust-constructed); delete with Element end container
  (`<div><p>ab</p><span>X Y</span></div>`, range `(ab, 1)`-`(span, 1)`: X removed); **extract with the same Element
  end container** (X moved into the fragment; D4's extract half, cross-checked against RC-2's rewritten extract,
  which must keep the test green); delete final boundary when start is not an inclusive ancestor of end; another
  live range at the deletion point; D13: `h.attachShadow({mode:'open'}); h.append(a, b)`, range `(h, 1)`-`(h, 2)`:
  `a` not contained, `comparePoint(h, 1)` / `isPointInRange` on the exposed index, deleteContents removes `b` only;
  D13 (b), two negative controls that discriminate (R3 Ax3-IMP1):
  - **toString, different containers**: a `<div>` holding `textBefore`, host `h` (light text plus text in its
    shadow tree) and `textAfter`; range `(textBefore, 1)`-`(textAfter, 1)`. toString is textBefore's tail, `h`'s light
    text and textAfter's head, with no shadow text. The raw walk starts at a Text start container and steps through
    `h`'s raw children, so today it reaches the ShadowRoot.
  - **delete / extract, partially contained host**: `h.append(span)` (`span` holds `ab`), **then**
    `h.attachShadow(...)` with text in the shadow tree, so the ShadowRoot is `h`'s last raw child (`shadow.rs:203`
    appends it); `h` is followed by `tail`; range `(ab, 1)`-`(tail, 1)`, so `h` is partially contained.
    deleteContents / extractContents leave the shadow tree and the ShadowRoot link untouched. Predicted today: the
    raw walk passes from `span` into the ShadowRoot, and the raw-index containment test admits the shadow text.
  - Rev 3's `(parent, 0)`-`(parent, 1)` case is dropped. There `h` is contained, so the whole host is removed or moved
    with its shadow tree, which is correct either way and so does not discriminate.
  VM: the same through JS; `Selection.deleteFromDocument` final boundary; `r.selectNode(x); r.insertNode(x)` end
  boundary; the D10/D11 cases of §0.4. MO: record order unchanged for existing `range_ops.rs` cases; new D4 cases
  emit the removal (delete) and move (extract) records for X.
- **DoD**: negative controls for D1, D2, D3, D4 (delete **and** extract), D5 (delete + Selection), D10, D11, D12,
  D13, each red-first or under its green-branch rule (§0.4), the outcome recorded in the PR body;
  `git grep -n 'contained_top_level_nodes\|next_in_preorder_global\|commit_range_after_mutation' -- crates`
  returns nothing; step 13 is not left in the host: `git grep -n 'with_range_mut\|Range::new' --
  crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` returns nothing (today 4 hits: `:50`, `:75`, `:158`,
  `:193`); `git grep -n 'child_index_in_ancestor\|dom\.get_first_child\|dom\.get_next_sibling' --
  crates/dom/elidex-dom-api/src/range` returns nothing (raw sibling access gone from Range; today **7** hits:
  `range/mod.rs:526`, `:533`, `:549` for `child_index_in_ancestor`, and `:558`, `:564`, `:574`, `:579` for the raw
  steps); **cap policy (B3)**: `git grep -n 'children_iter(' -- crates/dom/elidex-dom-api/src/range ':!*tests*'`
  returns nothing, or the per-PR plan justifies each remaining hit. Today there are 6 capped sites: `range/mod.rs:482`
  (`child_index`), `:503` (`node_length`), `range/mutation.rs:30`, `:179`, `:319`, `:441`. They move to the uncapped
  exposed list. The `(` excludes the two comments at `range/mutation.rs:453` and `:575`, and Selection's own
  `containsNode` walk (`selection.rs:661`, §8) is outside the scope. **Host-side error fold (E3)**: `git grep -n
  'dom_exc_' -- crates/script/elidex-js/src/vm/host/range_proto.rs` shows only `detached_range_error`'s
  `dom_exc_invalid_state_error` (today 9 hits: `:305`, `:505`, `:531`, `:701`, `:725`, `:803`, `:821`, `:828`,
  `:835`), or the per-PR plan justifies each other hit; each folded throw keeps a VM test of its exception name. E2
  for deleteContents, insertNode and `Selection.deleteFromDocument` (enqueue at the spec's steps; tests assert only
  orders equal under both models); RC-1 hygiene rows of §6 done.

### §5.2 RC-2 — one contents algorithm for clone and extract

- **Order**: after RC-1. Not gated on C0: the node document of the nodes it creates is Layout's (C3, §8), so the
  silent-wrong fixes D5 (extract), D6, D7 and D9 do not wait for C0 or the P1 switch-back (PM 201 / 202).
- **Scope**: §4.3. cloneContents implemented; extractContents replaced by the same function. Same-container and
  partial CharacterData for Text, Comment and CDATASection via `get_char_data` and canonical clone + set data (D7);
  partially contained element children cloned shallowly with recursive subranges (D6); doctype throw at step 11
  (D9, `HierarchyRequestError`, before any mutation); PI boundary refused up front (E1); every clone through the
  canonical cloner, and the fragment through today's creation call, so the owner rule is the current canonical one
  and no ownerless creation site is added (C3; the sites are in Appendix A); the clone-created event and its two
  dispatch-time consumer arms, shared with `cloneNode` (D-g′, §4.3 conditions 1-5), which retires the VM `cloneNode`
  post-pass and `enqueue_subtree_upgrade_reactions`; D14 and D15 fixed in the canonical cloner (§0.4), D15 measured
  first under the green-branch rule. **D14 lands before C1** (Layout),
  which edits the same lines (`node_methods/clone.rs:262`, `:290`, `:294-301`; C1 rev 1 `1a2cb286` D-C6); C1 rebases
  onto it (PM 215 Q-Y2, PM 216). ProcessingInstruction has no producer (§0.2,
  unreachable from script); `clone.rs:126` stays an unslotted stub here and gets a proposed slot (§8). The Attribute
  stub (`node_methods/clone.rs:120-125`) is in `CloneNode`'s `invoke` (`node_methods/clone.rs:91`), which RC-2 does not plan to edit (the contents
  algorithm calls `clone_node_with_shadow_honor`, and a range never holds an Attr). The per-PR plan checks this by
  its diff: if RC-2 edits that `match`, the Attribute clone is folded into RC-2; otherwise it is the proposed
  `#11-attr-clone-node` (§8).
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW), `crates/dom/elidex-dom-api/src/range/mutation.rs`, `crates/dom/elidex-dom-api/src/range/mod.rs`,
  `crates/dom/elidex-dom-api/src/node_methods/clone.rs` (D14; the event fire),
  `crates/core/elidex-ecs/src/dom/mutation_event.rs` (variant + `CloneScope`), `crates/core/elidex-ecs/src/dom/mod.rs`
  (`fire_clone_created`, re-export), `crates/core/elidex-ecs/src/lib.rs` (re-export),
  `crates/core/elidex-ecs/src/dom/tree_clone.rs` (doc row only, §6),
  `crates/dom/elidex-custom-elements/src/consumer.rs` (the `CloneCreated` arm and doc),
  `crates/dom/elidex-custom-elements/src/upgrade.rs` (the scope argument), `crates/dom/elidex-form/src/reconciler.rs`
  (the `CloneCreated` arm and module doc), `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` (field docs, §6),
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` (the post-pass deleted), `crates/script/elidex-js/src/vm/host/custom_elements/upgrade.rs` (`enqueue_subtree_upgrade_reactions` deleted),
  every exhaustive `match` on `MutationEvent` (no-op arm; the per-PR plan's census, §4.3 condition 1, re-run if C0's
  `Adopt` lands first, §9), tests
  (`crates/dom/elidex-dom-api/src/node_methods/tests/clone_shadow.rs` (NEW: RC-0a); `crates/dom/elidex-form/src/clone.rs`
  unchanged unless the per-PR plan finds otherwise).
- **Tests**: unit: extract and clone over `<p>a<b>bc<i>d</i>e</b>f</p>` with start inside `bc` and end inside `f`
  (fragment = `c`-text, shallow `<b>` holding `<i>d</i>` and `e`, then `f`-head); same for clone with the tree
  unchanged; Comment same-container and cross-container boundaries; doctype throw with tree unchanged; clone of a
  clonable shadow host and of a `<template>` (structure only: node documents are not asserted, they are Layout's,
  §8); RC-1's D4 extract case still green. VM: a custom element under a
  partially contained `<b>` keeps its identity and is not re-upgraded after extract (C2); a cloned custom element
  gets its upgrade reaction (C1); cloneContents over two contained custom-element subtrees enqueues upgrades in
  the spec's creation order (§0.3 item 17); no upgrade for a template-content clone (§4.3 condition 6).
  **Reaction order (D-g′; a design requirement, with a discriminating run instead of a pre-fix red)**: extract with a
  partially contained `x-clone` (defined, **no observed attributes**, so the order is the same under the element-queue
  model and the flat FIFO, E2) and a moved, connected, upgraded `x-m` inside its subrange; `x-clone`'s constructor
  runs before `x-m`'s `disconnectedCallback`. At the pre-fix head this test is red **because of D6**: today's extract
  makes no clone of a partially contained element (§0.4), so `x-clone`'s constructor never runs. Ordering has no
  pre-existing defect to show, since no clone of that kind exists today. So the ordering is pinned by a
  **discriminating run at RC-2's head**: a mutant that moves the CE pass to after the algorithm (the CE arm collects
  the roots and the native enqueues their upgrades after the engine returns, Rev 4's shape without placeholders) must
  turn the test red, and the PR body records that run. **D-a″ visit sets** (R3 Ax2-MIN2): a partially contained custom element is upgraded
  (`SelfOnly` root); a partially contained clonable host whose shadow tree holds a defined custom element gets the
  shadow-tree clone's upgrade before the upgrades of its subrange's roots; nested order, a `SelfOnly` root whose
  subrange holds a contained `Subtree` root and a further partially contained `SelfOnly` root, gives root order
  then walk order within each `Subtree`. D14: `host.cloneNode(false)` on a host with a clonable shadow root whose
  child has a child: the clone's shadow tree holds a deep copy (negative control; the existing test
  `tests/clone.rs:792-827` is inverted; same for a `<template>` with contents inside that shadow tree), and a defined
  custom element at that grandchild depth gets its upgrade (the CE check on the D14 control). **Shadow-children
  stamp guard**: `clone_node_shallow_shadow_children_carry_owner_document` (`tests/clone.rs:902`) stays green, and
  RC-2 extends it to the shadow child's descendant: once the shadow children are deep clones, `clone_subtree`'s
  `doc_override` stamps them and the hand stamp at `node_methods/clone.rs:294-301` goes. The assertion reads
  `owner_document` and expects the source's document, as the test does today; any further node-document rule is
  Layout's (C3). **Form pass per root**, three topologies: (1) the form control **is** the `SelfOnly` root: a boundary
  point may sit in an element, so `<div><input id=i><span>x</span></div>` with range `(i, 0)`-`(x, 1)` makes `i` the
  first partially contained child (it is an inclusive ancestor of the start node, not of the end node); its shallow
  clone is the root, and its subrange is collapsed. Also `<textarea>` with its Text as the start node, whose subrange
  clones the text tail into the root after the root's event; the copied raw value is the source's, not the tail's.
  (2) An `<input>` inside a contained subtree (a `Subtree` root). (3) An `<input>` inside a partially contained
  element's subrange (its own root, swept by its own event). In each the live value is copied and no `ClonedFrom`
  remains (a world query for `ClonedFrom` is empty after the call). **D15**: `document.cloneNode(true)` of a document
  holding a defined custom element enqueues no upgrade for the clone (measured first; green-branch rule). MO: extract
  records in spec order (start replace data, recursive subrange removals on the partial child, contained moves, end
  replace data); clone produces none.
- **DoD**: negative controls for D6, D7, D9, D5 (extract), D14, D15, each red-first or under the green-branch rule (§0.4); the
  D-g′ ordering test and its mutant run recorded; the
  cloneContents stub test (`tests_range.rs:441-452`) replaced by behaviour tests; no new ownerless creation site:
  `git grep -n -E 'create_(text|document_fragment)[a-z_]*\(' -- crates/dom/elidex-dom-api/src/range ':!*tests*'`
  lists only the fragment creation (today, besides `range/live.rs`'s inline tests that RC-0a moves to a test file,
  4 hits: `range/mutation.rs:266`, `:303`, `:354`, `:392`; the three Text clones
  move to the canonical cloner), and Appendix A is re-run at RC-2's head and sent to Layout. **One clone path (D-g′)**:
  `git grep -n 'enqueue_subtree_upgrade_reactions\|apply_clone_creation_ce_semantics\|apply_clone_form_state' --
  crates/script/elidex-js/src ':!*tests*'` returns nothing (no clone step in the VM; today 5 hits: the calls at
  `node_methods_extras.rs:221`, `:230` and `custom_elements/upgrade.rs:142`, the definition at `upgrade.rs:128`, and
  the doc line `node_methods_extras.rs:190`);
  `git grep -n 'fire_clone_created(' -- crates ':!*tests*'` shows the definition and one call, in
  `clone_node_with_shadow_honor`; `git grep -n 'MutationEvent::CloneCreated' -- crates ':!*tests*'` shows the
  emitter helper, the CE arm, the form arm and the no-op arms of the census, nothing else. The production-cloner
  census (`git grep -n 'clone_node_with_shadow_honor\|"cloneNode"' -- crates`) is re-run, and every caller is the
  entry's own callers listed in §4.3 condition 1. E2 for extractContents and cloneContents (enqueue at the spec's
  steps; tests assert only orders equal under both models); RC-2 hygiene rows of §6 done.

### §5.3 RC-3 — surroundContents

- **Scope**: surroundContents steps 1-7 on `LiveRangeRef`: step 1 (non-Text partially contained → InvalidStateError)
  and step 2 (Document / DocumentType / DocumentFragment → InvalidNodeTypeError, the kind RC-1 adds, E3) before any
  mutation; step 3 = RC-2's extract; step 4 replace all through the session primitive; step 5 =
  RC-1's insert, unchanged; step 6 append; step 7 select (`range/boundary.rs:125`). Late throws (insert step 1 when
  the post-extract start is a Comment; append into a CharacterData `newParent`, the spec's "historical reasons"
  note) leave the mutations done and return their records with the error (E3), which the host commits (M2).
- **Clones**: surround's extract creates CharacterData clones only: step 1 rejects any non-Text partially contained
  node, so a partially contained node is a Text; the same-container case clones the one CharacterData node; contained
  children are moved, not cloned. Those clones go through the canonical entry, so they fire `CloneCreated`, and both
  consumer arms find nothing in them (no element, no marker). The native has no clone step under D-g′ (§4.3), on the
  success path or the late-error path. Rev 4's "RC-3's native calls the clone seam" is withdrawn with the seam.
- **Files**: `crates/dom/elidex-dom-api/src/range/contents.rs` (NEW in RC-2), `crates/dom/elidex-dom-api/src/range/mutation.rs`,
  `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs`, tests (no `DomApiErrorKind` change: RC-1 adds the kind).
- **Tests**: each error branch with tree / range / records checked; surround with `newParent` already having
  children (replace-all records); surround around a Text-only partial selection; range after surround equals
  "select newParent"; late-throw cases with their records delivered.
- **DoD**: the surroundContents stub removed; E2 for surroundContents (enqueue at the spec's steps; tests assert only
  orders equal under both models, so a test with two moved custom elements asserts each one's disconnected before
  its connected, never the interleaving, §9); RC-3 hygiene rows of §6 done.

### §5.4 RC-4 — createContextualFragment

- **Dependencies** (PM 215 Q-Y1, PM 216; no Layout order is claimed). RC-4 **needs N9 landed** (expected: after
  Layout's slice b and N4 / N5, PM 216; N9 is the end of the node-memo chain N1 → N4 → N5 → N9, edges n3, n4 and n7
  in `docs/plans/2026-09-ibp-sandbox.md:585-594`). The "expected" clause is an expectation of timing with its source
  cited, not a claim about Layout's order. N9 lands the `ParserScriptingMode` enum (F11: `Normal | Disabled | Inert`,
  `ibp-sandbox.md:510`), and RC-4 adds the `Fragment` value. The enum's owner stays N9 (F11, a surface of the approved
  plan #532; PM 216). RC-4 also needs **C0**, for two things: `EcsDom::node_document` (the reader) and **C0's
  `EcsDom` entry for new nodes' document** (PM 223). Today that entry is `node_document_for_new_nodes`, a
  `pub(super)` method of the strict parser's `TreeBuilder` (`crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:271`),
  which RC-4's code in `elidex-dom-api` / `elidex-script-session` cannot call. C1 rev 2 (`b518b32f`, O-2) left open
  which PR promotes it; C0 rev 2 promotes it to an `EcsDom` entry (PM 223). **C1** stays in the dependency list
  because PM 216 put it there, but its reason (the entry) has moved to C0, so whether RC-4 still waits for C1 is a PM
  question (PM list). C1 rev 2's edge c-E plans both orders: "If RC-4 lands before C1, its cells for a Range inside
  template contents assert C2's document and C1 flips them; if after, RC-4 asserts I" (C1's own names for the two
  documents; `docs/plans/2026-10-eh-attrs-c1-template-owner-document.md` at `b518b32f`, edge c-E). PM 216 rules out the
  first while C1 is a dependency. **If any of these is late, tell the PM before starting RC-4.** RC-0b still lands
  before N9 (§7, PM 207 Q-X2).
  **Why waiting is acceptable**: createContextualFragment throws `NotSupportedError` today (§0.2), which is visible,
  so it is not a paydown (c) silent-wrong. RC-1..3 carry all the program's silent-wrong fixes, and none of them is
  gated on N9.
- **Scope**: `create_contextual_fragment` (dom-api, `range/mutation.rs:637`) resolves the context per HTML §8.5.7
  steps 3-6 (Element; Text / Comment → parent element; else null; null or HTML-namespace `html` in an HTML document
  → a new `body` created in the start node's node document), then calls the session's fragment entry (N9's one
  fragment entry, which derives Disabled through F10 with `NodeDocumentOf(context)`,
  `docs/plans/2026-09-ibp-sandbox-input.md:66`) with scripting mode `Fragment` and fragment parser mode Legacy (no
  declarative shadow roots), and places the roots into a new DocumentFragment (P4). **Realm document**: F10 takes a
  non-`Option` `realm_document` (`EcsDom::scripting_disabled_for(realm_document, subject)`, N9 F10,
  `docs/plans/2026-09-ibp-sandbox.md:481-484`). The VM native reads it through **J12, the one VM read route**
  (`ibp-sandbox.md:399`, "bound → (the relevant realm's document, `HostData::dom()`); unbound → refuse"), whose host
  file is `crates/script/elidex-js/src/vm/host/settings_route.rs` (NEW) (in **N4**, `docs/plans/2026-09-ibp-sandbox-input.md:63`;
  the route that marshals F10's call, `ibp-sandbox.md:495`). RC-4 adds no second realm-to-document read. It inherits
  **J2** (`ibp-sandbox.md:380`): an unbound VM is refused by the route before F10 is called. The native then passes
  the document down; that is marshalling, and the engine derives nothing from the VM (X1). **Node document**: RC-4 reads it
  only through `EcsDom::node_document` (C0). That is the engine's one reader (PM 157: N4's `effective_node_document`
  is replaced by `node_document` in C0's plan-review), so RC-4 adds none. RC-4's two creation sites (the `body`
  context and the result fragment) take their owner from C0's `EcsDom` entry for new nodes' document (PM 223; one
  creation entry, PM 207). They are listed in Appendix A. The created `body` is destroyed
  before return on every path (P3). Trusted Types: elidex has none (innerHTML passes the string through,
  `dom_inner_html.rs:146`), so step 2 is identity until it exists; the gap is program-wide, not this program's (§8
  proposed slot).
- **The `Fragment` value** (N9's D4 leaves it to this slot, §0.2). RC-4 adds it to `ParserScriptingMode` and maps it
  in **both** parser backends: `elidex-html-parser-strict` (N9's enum) and the html5ever-backed `elidex-html-parser`
  (`crates/dom/elidex-html-parser/Cargo.toml:19`).
  - **What the mapping is.** Tree construction tests the mode only as Disabled or not Disabled: the `noscript` start
    tag rows of the "in head" insertion mode read "if scripting mode is not Disabled" and "if scripting mode is
    Disabled" (`webref body html parsing-main-inhead`; the "in body" row has the same form, `parsing-main-inbody`).
    So html5ever gets scripting enabled for Fragment, the value `src/lib.rs:114-117` passes today for every mode except
    Disabled.
  - **What is not observable.** HTML §13.2.6.4.4 "in head" script start tag step 2 (Fragment: the script gets no
    parser document) and step 3 (only Inert sets "already started") write script-element state (`webref body html
    parsing-main-inhead`). elidex has neither field in either backend: `git grep -n -i -w -E
    'already_started|parser_document' -- crates` = 0, and the strict parser omits that bookkeeping on purpose
    (`elidex-html-parser-strict/src/tree_builder/text_only.rs:50-54`). So Fragment and Inert give the same element,
    and RC-4 cannot implement or test steps 2-3. They are proposed `#11-script-element-prepare-lifecycle` (§8),
    which question (a) informs.
  - "Both backends" therefore means the enum value and its tree-construction mapping, which RC-4 tests (the
    `<noscript>` case below). Question (e) is narrowed to match.
- **Overlap with the fragment plan** (`docs/plans/2026-10-fragment-dsd-not-on-context.md`, #574; PR 1 landed as #577
  `ee6c7a1a`): RC-4 relies on its I1 (the parse never mutates the context) and I2 (a top-level declarative template is
  plain); with Legacy mode the flag is off, so templates are plain at every depth (P2). The context rules differ from
  innerHTML's (target, or the shadow host) and outerHTML's (parent; DocumentFragment parent falls back to "div",
  `html_fragment.rs:138-142`, `:156-162`): RC-4's context is always an Element (the created `body` covers null), so no fallback is
  used. No change to the fragment plan's code.
- **Questions the per-PR plan must answer first (scout)**:
  (a) **Script run**: does elidex run "prepare the script element" (HTML §4.12.1.1) when a `<script>` is
  inserted, and from which state? The answer shapes `#11-script-element-prepare-lifecycle` (proposed, §8). It is
  not an RC-4 test, because the state Fragment and Inert differ in does not exist (above).
  (b) Does elidex have XML documents (fragment parsing algorithm steps step 1)? If not, the branch is unreachable and
  stated so. Its owner is `#11-domparser-xml-real-parsing` (the DOMParser native HTML-parses every XML MIME type
  today, `crates/script/elidex-js/src/vm/host/dom_parser.rs:253-262`).
  (c) **Scripting state**: consume N9's F10 `NodeDocumentOf(context)` derivation (HTML §13.4 HTML fragment parsing
  algorithm step 8: "If contextDocument's scripting is disabled, then set scriptingMode to Disabled"); RC-4 adds no
  `scripting_disabled` argument (today `dom_inner_html.rs:421`, `:476` pass `false`; N9 drops the field).
  (e) **html5ever**: does the html5ever mapping of Fragment give the same tree as the strict backend (in particular
  `<noscript>` with scripting enabled)? If it does not, the per-PR plan names the defect (what a tolerant-path
  fragment parse does differently from the spec), and it becomes the conditional `#11-html5ever-fragment-scripting-mode`
  (proposed, §8). RC-4 does not hide it. The script-element half is not asked here (it is (a)'s slot).
  (d) Are custom-element upgrade reactions for parsed elements enqueued at element creation (HTML §13.2.6.1 "create
  an element for a token" step 13: willExecuteScript is false for a fragment parse; step 15 → DOM §4.9 "create an
  element" enqueues the upgrade reaction) or only at insertion, in innerHTML today? RC-4 mirrors innerHTML; a
  difference is a parser-path defect, conditional slot `#11-fragment-parse-ce-upgrade-timing` (proposed, §8).
- **Files**: `crates/dom/elidex-dom-api/src/range/mutation.rs` (or its successor module), `crates/script/elidex-script-session/src/mutation/html_fragment.rs`
  (+ re-export in `mutation/mod.rs`, 994 lines at `ee6c7a1a`; re-measured at the per-PR plan, §7),
  `crates/dom/elidex-html-parser-strict/` (the enum value and its tree-construction mapping, in the file N9 puts the
  enum in; the script start tag steps 2-3 are not implemented, above), `crates/dom/elidex-html-parser/` (the html5ever
  mapping), `crates/script/elidex-js/src/vm/host/range_proto_mutation.rs` (calls the J12 route for `realm_document`),
  `crates/script/elidex-js/src/vm/host/settings_route.rs` (NEW) (N4's; touched only if J12 needs a new caller-facing
  wrapper, which the per-PR plan decides), tests.
- **Tests**: each context branch (Element; Text and Comment with parent element; Text under a DocumentFragment → body;
  Document start → body; `html` element → body; SVG-namespace `html`-named element is not replaced); `<td>` parsed with
  a `<tr>` context vs body context; nested `<template shadowrootmode>` stays plain; context unchanged and no leaked
  `body` (`world().len()` before and after); no MutationRecord; `<noscript><p>x</p></noscript>` parses as with
  scripting enabled, in each parser backend. That test pins **Disabled vs not Disabled** only: Fragment and Inert give
  the same tree and cannot be told apart until `#11-script-element-prepare-lifecycle` models the script-element
  state (above), so it does not pin Fragment against Inert. A second case, a context whose document has scripting
  disabled (F10 derives Disabled, §5.4 (c)), parses `<noscript>` as Disabled. **P4, measured without the fallback** (R3 Ax3-IMP4): the getter's
  `document_root` fallback (`node_methods/core.rs:279`, §0.2) would make a bound-document case green whatever the
  fragment's node document is. So the start node is in a document that is not the bound one (a DOMParser document,
  or `document.cloneNode(true)`), and `r.createContextualFragment(…).ownerDocument` is that document; or an
  `elidex-dom-api` unit test reads `EcsDom::node_document` of the fragment. The per-PR plan picks one. Rev 3's
  script-state test is removed (it checked state elidex does not have, above).
- **DoD**: the createContextualFragment stub test (`tests_range.rs:454-463`) replaced; questions (a)-(e) answered in the
  per-PR plan; E2 for createContextualFragment (tests assert only orders equal under both models; a parsed-element upgrade-timing difference goes to (d)); RC-4 hygiene rows of §6 done; `#11-range-full-impl`
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
| `node_methods/clone.rs:58-64` "The sole production caller is the VM `cloneNode` shim (which always runs `apply_clone_form_state`)" | rewritten: the consumer is `FormControlReconciler`'s `CloneCreated` arm, run at dispatch; a clone made with no dispatcher installed (this crate's unit tests, an unbound `EcsDom`) leaves the marker, which stays harmless; the marker contract itself is today's (§4.3 condition 5) | RC-2 |
| `tree_clone.rs:28` (the `FormControlState` row) "**Implemented, and event-less** … runs synchronously at clone time from the VM `cloneNode` marshalling shim … (there is no `MutationEvent::Clone`)" | rewritten: the cloning steps run synchronously at clone time from `FormControlReconciler`'s arm for `MutationEvent::CloneCreated` (NEW, D-g′), dispatched before the cloner entry returns; `#11-clone-cloning-steps-event` stays closed (§4.3 "#466 reversal"). The `ClonedFrom` row (`:29`) "within the *same* synchronous clone operation" stays true | RC-2 |
| `mutation_event.rs:1` "Mutation events fired synchronously from `EcsDom` mutation primitives", the variant list (`:45-106`) and the `MutationDispatcher` doc's "Production impl" consumer list (`:319-322`, which also omits the CE, event-handler and canvas consumers today); `consumer.rs:72-74` "Pattern-matches `Insert` / `Remove` / `AttributeChange`"; `reconciler.rs:4-8` "Subscribes to `AttributeChange` … and `Insert`" | extended to name `CloneCreated` (a creation event fired by the canonical cloner entry) and the two consumers that handle it | RC-2 |
| `consumer_dispatcher.rs:52-57` (`form_control` field doc) and `:78-87` (`custom_elements`: "Order-independent — only pushes to the side-band reaction queue, which no other consumer reads") | rewritten: `form_control` also runs the clone cloning steps; `custom_elements` also writes `CustomElementState` (`is_value`) on `CloneCreated`, which no other consumer reads, so both stay order-independent (§0.3 item 29) | RC-2 |
| `custom_elements/upgrade.rs:117-127` (doc of `enqueue_subtree_upgrade_reactions`), `node_methods_extras.rs:214-229` (shim comments), `node_methods_extras.rs:187-190` (the `cloneNode` doc naming `apply_clone_form_state`), `elidex-form/src/clone.rs:16-17` ("invoked synchronously at clone time from the VM `cloneNode` shim") | deleted with the post-pass, or rewritten to name the consumer arms | RC-2 |
| `node_methods/clone.rs:288-289` "shadow children clone with the operation's own `subtree` flag" | rewritten with the D14 fix (step 6.7: subtree true) | RC-2 |
| `selection.rs:690` `#[allow(dead_code)]` on the test helper `elem` ("kept for future cross-doc / no-association tests") | connect it (a no-association test RC-1 needs anyway) or delete it | RC-1 |

**Citation rule (each per-PR plan, on the files it edits).** Every § number, § ↔ title pair and step number in those
files is fixed or verified in that PR. The method is the sweep below (truth first, then the four patterns of Axis 4):

1. Establish the truth first: `webref dfn` / `heading` / `body` for each concept the files cite.
2. **Prefixed**: `git grep -n -P '(DOM|HTML|CSSOM|Selection API) §4\.10(?!\.?\d)' -- <files>`. The lookahead
   `(?!\.?\d)` excludes a following digit or `.digit` only, so a sentence-final `§4.10.` is caught and `§4.10.5` /
   `§4.100` are not (§0.3 item 22).
3. **Unprefixed**: `git grep -n -P '§4\.10(?!\.?\d)' -- <files>`.
4. **Synonym / context**: `git grep -n -i -P '(spec|standard|whatwg)[^§\n]{0,20}§\d' -- <files>` ("per spec §",
   "DOM spec §", "WHATWG §", "HTML Standard §").
5. **Non-existent-section probe**: list every § the files cite **with its spec name**, `git grep -h -o -P
   '(?:\b(?:DOM|HTML|CSSOM|Selection API|ECMA-262|WebIDL)\s+)?§\d+(?:\.\d+)*' -- <files> | sort | uniq -c`, and run
   `webref heading <spec> <n>` on each; a number with no heading is a drift (e.g. `webref heading dom 5.10` → none).
   A bare `§n` in the output has no spec word before it, so its spec comes from the line (`git grep -n -P
   '(?<!DOM |HTML |CSSOM |API |262 |IDL )§<n>\b' -- <files>`), and the plan records which spec it resolved to.
   On `range/mutation.rs` at `ee6c7a1a` this prints 11 distinct forms, among them bare `§5.10` and `§4.8` and the
   prefixed `DOM §4.8`.

Patterns 2-5 are the four of `.claude/skills/elidex-review/axes.md` Axis 4 `[plan]` (2): prefixed, unprefixed,
synonym / context, non-existent-section probe (item 1 is the truth step, not a pattern). Step numbers get their own
grep, case-insensitive, plural-aware and hyphen-aware: `git grep -n -i -P '\bsteps?[\s-]+\d+(\.\d+)*' -- <files>`
(catches "Step 7", "steps 2-3", "step 13.1", and the hyphenated "step-12", "pre-step-12", "step-10/11"), compared
with `webref body`. On `range/mutation.rs` the separator `[\s-]+` finds 41 lines where `\s+` found 33; the 8 extra
lines are the hyphenated forms at `:501`-`:574`. All four patterns and the step grep were checked on a fixture
(2026-11-01; the hyphen forms on a second fixture, Rev 5).

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
| `crates/dom/elidex-dom-api/src/node_methods/tests/clone.rs` | 992 | RC-2 (D14 tests, the D-a″ unit cases); also C1 (Layout) rebases onto D14 in the same area | **RC-0a**: a D14 unit test alone would take it past 1000, so the split comes first (CLAUDE.md touch-time split) rather than putting the test elsewhere. Split at its existing section header `:722` ("Shadow honor", to `:992`) into `node_methods/tests/clone_shadow.rs` (NEW); D14's and RC-2's shadow tests go there. Layout is told, since C1 rebases after RC-2 |
| `crates/script/elidex-script-session/src/mutation/html_fragment.rs` | 858 (tests from `:277`) | RC-4; also N9 (F11's fragment derivation, `ibp-sandbox.md:510-511`) | **RC-0b** (projected): inline tests to `mutation/tests_html_fragment.rs` (NEW), as `mutation/tests_replace_all.rs` already is. Lands **before N9** (PM 207: the first toucher splits; N9 rebases onto it) |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 2024 at `ee6c7a1a`; 729 after PR #578 | not touched (§4.2) | none: PR #578 is that split, and RC-1 lands after it. If a per-PR plan must touch `HostData`, it edits the post-#578 file (`mod.rs` or `engine_feature/bind.rs`) |
| `crates/script/elidex-script-session/src/mutation/mod.rs` | 994 | RC-4 (+1-2 re-export); also edited by C0 (D-5's fragment arm, C0 memo O-4 (b)) | **re-measured at each per-PR plan** that touches it: if C0 (or anything else) has taken it to ≥ 1000, a standalone pure-move split comes first (CLAUDE.md touch-time split); at `ee6c7a1a` none |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | RC-2 (`fire_clone_created`, about 15 lines, and a re-export); also edited by C0 (Layout, for `Adopt`; C0 rev 2's `EcsDom` entry for new nodes' document, PM 223, may land here too) | none as prereq at ~940; re-measured at RC-2's per-PR plan, after whichever of C0 / RC-2 lands first, and a split comes first if the two together take it to ≥ 1000 |
| `crates/dom/elidex-dom-api/src/range/mutation.rs` | 640 | RC-1..RC-4 | none as prereq; new code goes into `containment.rs` / `contents.rs` (NEW) so it does not grow past ~800 |
| `range/mod.rs` 587, `range/boundary.rs` 299, `range_proto_mutation.rs` 262, `range_proto.rs` 906, `tests_mutation_observer/range_ops.rs` 586, `node_methods/clone.rs` 458, `elidex-custom-elements/src/upgrade.rs` 526, `consumer.rs` 329, `node_methods_extras.rs` 293, `custom_elements/upgrade.rs` 487, `elidex-form/src/clone.rs` 304, `elidex-form/src/reconciler.rs` 333, `vm/consumer_dispatcher.rs` 160, `child_node/mod.rs` 459, `dom_bridge.rs` 696, `elidex-ecs` `dom/mutation_event.rs` 352 (also C0's `Adopt`), `elidex-script-session/src/types.rs` 327 | under 1000 | various | each per-PR plan re-measures. `range_proto.rs` (906) is touched by RC-1 (the `RangePointError` mapper and the host-side error fold, E3), which moves code out of it; RC-1's plan re-measures it. `reaction.rs` and `custom_elements/flush.rs` are no longer touched (D-g′ adds no reaction variant) |

RC-0a and RC-0b are pure moves (no text change beyond module declarations), reviewed with `git diff --color-moved`.

## §8. Out of scope, and slots

- **`#11-shadow-tree-live-range-tracking`** (open, unchanged): shadow-tree live-range boundary updates. The program
  keeps the shallow light-tree contract of `range/live.rs:38-55`, which follows from the engine firing no event for
  ShadowRoot-parented mutations (`tree/mutation.rs:289-291`; L1). (D13 is a different defect: Range's own light-tree
  indices and walks on a host, not boundaries inside the shadow tree.)
- **`#11-range-client-rects`** (open, owner Layout): `getBoundingClientRect` / `getClientRects`, a consumer of the
  forced style/layout flush umbrella. Not touched.
- **`#11-insertadjacenthtml-context-validity`** (open; re-eval 2026-08-26, **overdue**; ledger
  `project_open-defer-slots.md`, the slot's own entry, cited by name because its line moves): its trigger lists "Range work touching `apply_insert_adjacent_html`"; RC-4
  does not touch it, so the trigger does not fire. Its defect text also names `Range.createContextualFragment` in the
  parser-path family. That entry is **moot**: createContextualFragment parses into a new fragment and inserts
  nothing, so there is no pre-insert validity step to skip (HTML §8.5.7 steps 1-7). Proposed disposition (PM):
  narrow the family to innerHTML / outerHTML / setHTMLUnsafe / insertAdjacentHTML, extend-with-cause to 2026-12-01
  (cause: no parser-path insertion-validity pass is planned).
- **Selection.containsNode**: uses the Selection spec's own containment (`node_fully_contained`,
  `selection.rs:652-670`), not DOM "contained"; B1 does not replace it.
- **`#11-selection-document-membership`** (C0's R-6 (c) **proposal**, not a registered slot; informational): its
  site `selection.rs:614-619` is in RC-1's file. RC-1 does not change it. If this memo lands before C0, the ID is
  named on main with no ledger row, so it needs a homing row (PM list, H5).
- **IBP-sandbox N9** (Layout): introduces `ParserScriptingMode` (Normal, Disabled, Inert), the scripting-disabled
  derivation and F11 at every fragment-parse caller (§0.2). The enum's owner stays N9 (F11; PM 216). RC-4 adds
  `Fragment` after N9 lands (§5.4).
- **Node document of Range-created nodes** (C3's write rule; R2's D8; shallow-clone stamping): a facet of
  `#11-ownerless-node-creation` (registered, PM 207), owned by **Layout** (C0 / C1). One mechanism: every creation
  site takes its owner from C0's `EcsDom` entry for new nodes' document (PM 223), and reads use only `node_document`
  (C0).
  Appendix A is this program's input to C0 rev 2. Template contents' owner document is C1's
  (`#11-template-contents-owner-document`).
- **XML fragment parsing** (§5.4 (b)): `#11-domparser-xml-real-parsing` (`vm/host/dom_parser.rs:253-262`).

**Proposed slots outside the program's own fixes** (for the PM to register, fold or decline; owner left to the PM):

| Proposed ID | Defect | Why deferred | Trigger | Re-eval |
|---|---|---|---|---|
| `#11-trusted-types` | elidex has no Trusted Types; every sink that takes `(TrustedHTML or DOMString)` (createContextualFragment step 2, innerHTML, …) treats the string as is | program-wide gap (one mechanism for every sink), not a Range defect; until it exists, createContextualFragment step 2 is identity, like innerHTML | a Trusted Types / CSP `require-trusted-types-for` program | 2026-12-01 |
| `#11-processing-instruction-clone` | `CloneNode` refuses ProcessingInstruction with NotSupportedError (`node_methods/clone.rs:126-131`), an unslotted stub | unreachable from script (no PI producer, §0.2); the Range contents algorithm refuses a PI boundary before any mutation (E1) | a PI producer (XML parsing, `createProcessingInstruction`) | 2026-12-01 |
| `#11-attr-clone-node` | `attr.cloneNode()` throws NotSupportedError (`node_methods/clone.rs:120-125`); reachable from script through `document.createAttribute` (`char_data/attr.rs:22`) | not a Range defect: a range never holds an Attr, and RC-2 does not plan to edit `CloneNode`'s `invoke` (`node_methods/clone.rs:91`) (§5.2; if it does, the fix folds into RC-2). **Fold candidate** (PM decides): the open `#11-dom-api-attr-entity-model-unification` (the `elidex-dom-api` Attr handlers keep a second Attr-identity mode; ledger, that slot's entry), since an Attr clone needs the Attr entity model that slot settles | an Attr-node program, the fold, or RC-2's diff touching that `match` | 2026-12-01 |
| `#11-script-element-prepare-lifecycle` (replaces Rev 3's conditional script-run slot) | elidex keeps no script-element "already started" or parser-document state (`git grep -n -i -w -E 'already_started\|parser_document' -- crates` = 0; the strict parser omits it on purpose, `text_only.rs:50-54`). So HTML §13.2.6.4.4 script start tag steps 2-3 cannot be represented: parser scripting modes Inert and Fragment give the same element, and the spec's difference (Inert's scripts never run; Fragment's run when inserted, HTML §4.12.1.1 prepare the script element) is not modelled. **Neighbour**: the closed `#11-tags-T2a-script-load-lifecycle` (roadmap `m4-12-platform-gap-roadmap.md`, that slot's row: "CLOSED 2026-05-16 (sweep A10 close-as-won't-fix)", about the `async` / `defer` load lifecycle of a dynamic `<script>.src`). This slot differs: it is the "already started" / parser-document state that decides *whether* an inserted parsed script runs at all, which createContextualFragment (Fragment) and innerHTML (Inert) must tell apart; T2a was about *when* a fetched script runs. The won't-fix reason (framework demand for dynamic `.src`) does not cover it | the defect is in script-element processing (dynamic script execution), not in Range; RC-4 adds the `Fragment` value and its tree mapping, which is all that is observable today (§5.4) | RC-4's per-PR plan answering (a), or a dynamic script-execution program | 2026-12-01 |
| `#11-html5ever-fragment-scripting-mode` (conditional, §5.4 (e)) | the html5ever backend's tree for a Fragment-mode parse differs from the strict backend's (e.g. `<noscript>` with scripting enabled) | only if RC-4's question (e) finds a difference; a tolerant-path parser defect, not a Range one | RC-4's per-PR plan answering (e) | 2026-12-01 |
| `#11-live-range-bridge-wide-sibling-index` (X5) | past `MAX_ANCESTOR_DEPTH` (10,000) previous siblings, `EcsDom::index_in_parent` returns `None` and the tree primitives fire no `MutationEvent` (`navigation.rs:55-75`; `tree/mutation.rs:46`, `:75`, `:191`), so live ranges (and every other consumer) miss those updates | pre-existing and engine-wide (the bridge and every consumer share the fire site), not a Range-algorithm defect; Range's own derivations are uncapped (B3) | a wide-DOM program, or a report of a missed live-range update | 2026-12-01 |
| `#11-ce-reactions-method-scope-drain` (**proposed**, E2; the full registration proposal is in §9) | `[CEReactions]` methods do not invoke their element queue before returning (HTML §4.13.6 `[CEReactions]` steps 1-4); elidex drains only at checkpoints (§0.2, §9) | engine-wide (§9) | §9 | 2026-12-01 |
| `#11-fragment-parse-ce-upgrade-timing` (conditional, §5.4 (d)) | parsed custom elements get their upgrade reaction at insertion instead of at creation (HTML §13.2.6.1 steps 13, 15) | only if RC-4's question (d) finds a difference; a parser-path defect shared with innerHTML. Its fix should reuse or generalise RC-2's `MutationEvent::CloneCreated` (a creation event the CE consumer handles at dispatch, §4.3) instead of adding a second creation mechanism such as an `ElementsCreated` event (R4 Ax2; one issue, one way) | RC-4's per-PR plan answering (d) | 2026-12-01 |

The two conditional slots (`#11-fragment-parse-ce-upgrade-timing`, `#11-html5ever-fragment-scripting-mode`) are
registered only if their trigger finds the defect. Until then each is named in this memo with no ledger row, so each
needs an interim homing row "Not registered by design (conditional)" (PM list). The script-element slot's owner
(dynamic script execution) is for the PM to assign. A `[CEReactions]` **queue-order** difference has one of three
homes (E2): the registered `#11-ce-element-queue-grouping` (§9) when the flat-FIFO grouping alone explains it, RC-4's
question (d) for parsed-element upgrade timing, and otherwise the slice that owns the method, which fixes it. The
**drain point** is the proposed `#11-ce-reactions-method-scope-drain` (§9).

## §9. Order, suspension points, cross-lane dependencies, definition of done

**Order.** RC-0a → RC-1 → RC-2 → RC-3; RC-0b before N9; RC-4 after its three landings (RC-4 is independent of
RC-1..3). No Layout order is stated (PM 215 Q-Y1). Cross-lane:

- **Suspension points (PM decision 201).** Each RC is a suspension point: after any merged slice, L3 may switch to P1
  when C0 lands ("L3 switches to P1 at a slice boundary once C0 lands", PM 201). No slice is left half-landed; the
  program resumes at the next RC.
- **RC-1 after PR #578.** `LiveRangeRef` is written against the post-#578 `engine_feature/` layout (§4.2).
- **RC-2 is not gated on C0.** The node document of the nodes it creates is Layout's (§8); RC-2 adds no ownerless
  creation site (C3), and Appendix A, re-run at RC-2's head, goes to Layout.
- **RC-2 (D14) before C1 on `node_methods/clone.rs`.** D14 and C1 rev 1 (`1a2cb286`, D-C6) edit the same lines
  (`clone.rs:262` pass 3's subtree flag, `:290` the step 6.7 child clone, `:294-301` the shallow-branch stamp). D14
  lands first and C1 rebases onto it (PM 215 Q-Y2; agreed by Layout, PM 216; C1 rev 2 `b518b32f` records it as
  edge c-D14). RC-0a's split of
  `node_methods/tests/clone.rs` (§7) lands before both.
- **`<template>` clone tests and C1.** RC-2's template tests assert structure, not node documents. The CE-pass rule
  (§4.3 condition 6) does not depend on C1.
- **RC-0b before N9** (PM 207 Q-X2). RC-0b splits `html_fragment.rs`'s tests first (touch-time split: the first
  toucher splits); N9 rebases onto it.
- **RC-2 and C0 on `MutationEvent` (H6).** C0's `Adopt` and RC-2's `CloneCreated` add variants to the same enum
  (`crates/core/elidex-ecs/src/dom/mutation_event.rs`), arms to the same consumer `match`es
  (`CustomElementReactionConsumer::handle`, and C0's `Adopt` arm sits beside RC-2's arm there), and rows to the same
  census of exhaustive `MutationEvent` matches (§4.3 condition 1). Both also edit `crates/core/elidex-ecs/src/dom/mod.rs`
  (§7). Neither waits for the other. **Whichever of the two lands second re-runs the census at its own head and adds
  its no-op arms to every exhaustive match the first one added**, and re-measures `dom/mod.rs` (§7). The PM tells
  Layout (PM 222); H6 below is the text.
- **RC-4's dependencies** (§5.4; PM 215 Q-Y1, PM 216, PM 223): N9 landed (the enum; expected after Layout's slice b
  and N4 / N5, the end of the node-memo chain, `docs/plans/2026-09-ibp-sandbox.md:585-594`; N4 lands the J12 route),
  C0 (`node_document`, and C0's `EcsDom` entry for new nodes' document, PM 223), and C1 (kept per PM 216; PM
  question, §5.4). This lists dependencies and makes no ordering claim. If any of them is late, the
  driver tells the PM before starting RC-4. Waiting is acceptable because createContextualFragment throws today
  (visible, not paydown (c)), and RC-1..3, which carry every silent-wrong fix, are not gated on N9. The
  `range/mutation.rs:632` line: N9 re-points it, RC-4 deletes it (§6).
- **`mutation/mod.rs`** (994 lines, C0 edits it): re-measured at each per-PR plan (§7).

**Engine-wide `[CEReactions]` slots (E2).** The Range slices promise only that they add no divergence beyond these
two. Both are about the reaction machinery every `[CEReactions]` method shares, not about Range.

- **`#11-ce-element-queue-grouping`** (**registered**, 2026-11-01, PM 223; owner L3, ledger entry). Grounds for the
  registration, as found by R4 (Ax2, Ax4):
  - **Spec model.** Each element has its own custom element reaction queue, and an element queue holds elements
    (`webref body html custom-element-reactions-stack`; `webref dfn html 'element queue'` → §4.13.6). Enqueueing a
    reaction appends it to the element's own reaction queue and then adds the element to the current element queue
    ("enqueue a custom element upgrade reaction" steps 1-2, `enqueue-a-custom-element-upgrade-reaction`; "enqueue an
    element on the appropriate element queue" step 3, `enqueue-an-element-on-the-appropriate-element-queue`).
    Invoking an element queue dequeues an element and runs **all** of its reactions before the next element
    ("invoke custom element reactions" steps 1.1-1.3, `invoke-custom-element-reactions`). So an element's reactions
    run together, at the position where it first entered the queue, and a reaction enqueued while the element's
    reactions run (for example by its upgrade) runs right after them.
  - **elidex.** One flat FIFO of reactions (`consumer.rs:120-121` `queue.extend`), drained in waves of the whole
    queue, with reactions enqueued during a wave going to the next wave (`vm/host/custom_elements/flush.rs:47-71`).
  - **Divergence examples** (read from the code and the spec, not run). (1) `insertNode(x)` where `x` is connected and has a custom-element child `y`: the
    remove of step 9 enqueues `x` disconnected and `y` disconnected, then the insert enqueues `x` connected and `y`
    connected. The spec runs x.disc, x.conn, y.disc, y.conn; elidex runs x.disc, y.disc, x.conn, y.conn.
    (2) `surroundContents(p)` over two connected custom elements `m1`, `m2`: step 3's extract disconnects both, and
    step 5's insert of `p` (holding them) reconnects them. Spec: m1.disc, m1.conn, m2.disc, m2.conn; elidex:
    m1.disc, m2.disc, m1.conn, m2.conn. (3) R4 Ax2's case: extract with a partially contained `x-clone` that has an
    observed attribute, and a moved connected `x-m`. The upgrade of `x-clone` enqueues its attributeChangedCallback
    ("upgrade an element" step 4, `webref body html concept-upgrade-an-element`). Spec: x-clone constructor, x-clone
    attributeChanged, x-m disconnected. elidex: x-clone constructor, x-m disconnected, then x-clone attributeChanged
    in the next wave (`upgrade.rs` `finalize_success` pushes to the back of the queue).
  - **Why it is not Range's.** Every `[CEReactions]` method that enqueues reactions for two elements interleaves them
    the same way (`appendChild` of a subtree, `replaceChildren`, innerHTML); a Range-only grouping would be a second
    queue model (one issue, one way). The Range tests assert only orders that are equal under both models (E2).
- **`#11-ce-reactions-method-scope-drain`** (**proposed**, not registered; this memo's proposal from Rev 4, §0.3 item
  19; full text for the PM, PM 224):
  - **Defect.** No `[CEReactions]` native drains its reaction queue when the method returns. HTML §4.13.6
    `[CEReactions]` steps 1-6 push an element queue, run the method's steps, pop the queue, **invoke its reactions**
    (step 4), and only then rethrow or return (steps 5-6) (`webref body html cereactions`; `webref heading html
    4.13.6` = Custom element reactions). elidex drains only at the callers of `flush_ce_reactions` (`git grep -n
    'flush_ce_reactions(' -- crates/script/elidex-js/src`, outside tests): the `eval` tail
    (`vm/interpreter.rs:54`), the engine's post-call settle (`engine.rs:165`), timers (`vm/natives_timer.rs:281`)
    and `customElements.define` (`vm/host/custom_elements/define.rs:204`). So a callback the spec runs inside the
    method (before the next statement of the calling script) runs after the script, or after the task. Example:
    `document.body.append(el); log(el.connectedRan)` logs `true` in the spec and, predicted from the drain sites above, `false` in elidex (not run).
  - **Why deferred.** Engine-wide, not Range-specific: it is one mechanism for every `[CEReactions]` member
    (`appendChild`, `setAttribute`, innerHTML, the seven Range / Selection methods, …). A per-method drain in the four
    Range natives would be a second mechanism beside every other method's (one issue, one way). The fix is the
    element-queue stack itself (push at native entry, pop and invoke at exit), which touches every `[CEReactions]`
    native's bracket and the flush's re-entry, so it is edge-dense and gets its own plan-review.
  - **Owner (proposed; the PM decides).** L3, which owns the CE reactions and already owns
    `#11-ce-element-queue-grouping`; the VM lane is the alternative, since the bracket is in the VM natives.
  - **Trigger.** A `[CEReactions]` timing WPT or test, the S5 VM-drives-shell event-loop work, or the merge below.
  - **Re-eval.** 2026-12-01, the same as the other proposed slots.
  - **Relations.** (a) `#11-ce-reaction-mutation-observer-ordering` (open; the VM drains MutationObserver
    microtasks before CE reactions, ledger entry): both are about *when* the reaction queue drains relative to other
    work, so it is a **merge candidate** (the PM decides). (b) `#11-ce-element-queue-grouping`: the same family,
    the other half of the element-queue model (that slot is the *order inside* a drain, this one is *where* the
    drain happens). A fix of either should build the element-queue stack once for both.

**Definition of done.**

- RC-0a, RC-1 (after #578), RC-2, RC-3 merged in that order; RC-2 before C1's clone edit; RC-0b before N9; RC-4
  after N9, C0 and C1.
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
  `#11-range-tostring-boundary-text` → RC-1 (was "slice 3"); that slot's entry cites `range/mutation.rs:16-86`,
  which becomes `:16-68`. Ledger rows are cited by slot name, not line.
- **Pointer rows** (`project_open-defer-slots.md`, "Registered in a plan memo" and "Registered in a memory file"):
  - `#11-range-comment-extract-clone` points to the B1.3-ii plan's §8 (`docs/plans/2026-06-b1.3-ii-splittext-range-characterdata-records-plan.md:205`).
    The PM re-points it to this umbrella (`docs/plans/2026-11-range-contents-program.md`, RC-2, §0.4 D7) and keeps the
    B1.3-ii cite as its origin.
  - `#11-range-full-impl` points to `m4-12-platform-gap-roadmap.md:1251`, but the row is at `:1252`
    (`grep -n 'range-full-impl' memory/m4-12-platform-gap-roadmap.md`). The PM corrects it, or re-points it to this
    umbrella once the roadmap row below is replaced.
- **IBP-sandbox `:763`** (a PM / Layout hand-off; the memo does not edit Layout's file). Replacement for its
  `#11-range-full-impl` row's last cell, "the Fragment parser mode is its only new consumer (D4); N9's plan-memo
  re-evaluates; still a PR7 prerequisite": "the Fragment parser mode is its only new consumer (D4); re-evaluated by
  the Range contents umbrella (`docs/plans/2026-11-range-contents-program.md`): promote-to-PR, RC-4 adds Fragment to
  N9's enum after N9 lands; re-eval 2026-12-01". The "PR7 prerequisite" clause goes (D-26 PR7 landed in #458).
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

Settled and removed from this list: R1-R4 (ratified by PM decision 215); Q-Y1 (no Layout order) and Q-Y2 (D14 first,
C1 rebases) (PM 215); RC-4's dependency text and the clone.rs order agreed by Layout (PM 216); Appendix A sent to
Layout and D14's slot registered (PM 212). Q-X1 to Q-X4 of Rev 2 were settled by PM 207. R5 is withdrawn (§0.1).
Rev 5: the #466 reversal goes to `/external-converge` R1 (PM 222); `#11-ce-element-queue-grouping` is registered
(PM 223); the C0 overlap is told to Layout by the PM (PM 222, H6); the new-node document entry is C0 rev 2's (PM 223),
so Rev 4's H1 question is answered and dropped.

**Questions** (the PM decides):

- **Q-Z1 (C1 as an RC-4 dependency).** PM 216 listed C1 because RC-4 needed `node_document_for_new_nodes`. PM 223
  moves that entry to C0. Does C1 stay a dependency of RC-4? If not, C1 rev 2's edge c-E "RC-4 before C1" branch
  becomes possible (§5.4). Rev 5 keeps C1 listed until the PM rules.
- **Q-Z2 (owner of the proposed `#11-ce-reactions-method-scope-drain`).** Proposed L3; the VM lane is the alternative (§9). Merge
  with `#11-ce-reaction-mutation-observer-ordering` or not.

**Hand-offs** (the memo edits neither Layout's files nor the ledger):

- **H1 (Layout, statement)** RC-4's two creation sites (Appendix A11 / A12) take their owner from C0's `EcsDom` entry
  for new nodes' document, which C0 rev 2 adds by promoting the strict `TreeBuilder`'s `pub(super)`
  `node_document_for_new_nodes` (`tree_builder/mod.rs:271`) (PM 223). C1 rev 2 O-2 is thereby answered.
- **H2 (Layout)** RC-0a splits `node_methods/tests/clone.rs` (§7), then RC-2 lands D14 on `node_methods/clone.rs`,
  and C1 rebases onto both (§9).
- **H3 (Layout / PM)** Replacement text for `docs/plans/2026-09-ibp-sandbox.md:763` (§9).
- **H4 (Layout, observation)** `docs/plans/2026-09-ibp-sandbox-input.md:66` cites HTML §13.4 "steps 5, 10" for the
  mode derivation. Step 5 (contextDocument) is right. Step 10 is misattributed: it sets the tokenizer state. The
  scripting-disabled step, step 8, is missing (`webref body html html-fragment-parsing-algorithm`).
- **H5 (PM, ledger and roadmap)** All in §9: the home texts and the toString slot's line range; the two pointer-row
  edits (`#11-range-comment-extract-clone` re-pointed, `#11-range-full-impl` `:1251` → `:1252`); the lifecycle of the
  four pre-existing slots (promote-to-PR, 2026-12-01); the roadmap row text. Also narrow and re-date the overdue
  `#11-insertadjacenthtml-context-validity` (§8). New in Rev 5: interim "Not registered by design (conditional)"
  homing rows for `#11-fragment-parse-ce-upgrade-timing` and `#11-html5ever-fragment-scripting-mode` (§8); a homing
  row for C0's proposed `#11-selection-document-membership` if this memo lands before C0 (§8); optionally a note on
  the closed `#11-clone-cloning-steps-event` that its mechanism becomes `CloneCreated` in RC-2 (§4.3 "#466
  reversal").
- **H6 (Layout, via the PM, PM 222)** C0's `Adopt` and RC-2's `CloneCreated` share the `MutationEvent` enum, the
  consumer `match`es and the census of exhaustive matches, and both edit `elidex-ecs` `dom/mod.rs`. Whichever lands
  second re-runs the census at its head, adds its arms to every exhaustive match the first one added, and re-measures
  `dom/mod.rs` (§9, §7).
- **H7 (Layout, observation)** C1 rev 2's edge c-E plans "RC-4 before C1" as one branch; PM 216 rules it out while C1
  is an RC-4 dependency (Q-Z1).

**Proposed slots** (register, fold or decline; Why / trigger / re-eval in §0.4, §8 and §9):

- From §0.4, closed by the program's own slices: `#11-range-contained-enumeration-end-element` (D4),
  `#11-range-delete-extract-new-boundary` (D5), `#11-range-insert-node-step13-timing` (D12),
  `#11-range-compare-points-shadow-index` (D13, raw-sibling tree access), `#11-range-insert-node-validity` (D10,
  D11), `#11-range-contents-doctype-check` (D9), `#11-clone-document-registry-rewrite` (D15, new in Rev 5). Folded IDs
  need "Not registered by design" homing rows.
- From §8 / §9, outside the program: `#11-trusted-types`, `#11-processing-instruction-clone`, `#11-attr-clone-node`
  (fold candidate `#11-dom-api-attr-entity-model-unification`), `#11-script-element-prepare-lifecycle` (owner: dynamic
  script execution, to assign), `#11-live-range-bridge-wide-sibling-index` (was Rev 3's X5 question),
  `#11-ce-reactions-method-scope-drain` (**proposed**; full proposal in §9; Q-Z2).
- Conditional (registered only if the per-PR plan finds the defect; interim homing rows in H5):
  `#11-fragment-parse-ce-upgrade-timing` (RC-4 (d)), `#11-html5ever-fragment-scripting-mode` (RC-4 (e)).
- Withdrawn, never registered; each needs a "Not registered by design" homing row while the memo names it:
  `#11-range-contents-node-document` (R2's D8, now a facet of `#11-ownerless-node-creation`);
  `#11-fragment-script-run-on-insert` (Rev 3's conditional, replaced by `#11-script-element-prepare-lifecycle`).

**For the per-PR plans** (not PM questions): RC-1: the `RangePointError` and host-side error fold (E3); D10 / D11 /
D12 / D13 measurements; the cap-policy justifications (§5.1 DoD). RC-2: the Attribute-stub check (§5.2); the census
of exhaustive `MutationEvent` matches, re-run if C0 lands first (§4.3 condition 1, §9); D15's measurement; the
D-g′ ordering mutant run. RC-4: §5.4 (a)-(e); the P4 producer; whether J12 needs a new wrapper in
`settings_route.rs`.

## Appendix A. Range node-creation sites for C0 rev 2

Every site in the Range code, and in the planned RC-2 / RC-4 algorithms, that creates a node, at `ee6c7a1a`.
"Ownerless" means the node gets no `AssociatedDocument` at creation. Re-run (prints exactly A1-A5 at `ee6c7a1a`):
`git grep -n -P '\bdom\.(create_[a-z_]+|clone_(node|subtree)[a-z_]*)\(|split_text_at_offset\(' -- crates/dom/elidex-dom-api/src/range crates/dom/elidex-dom-api/src/selection.rs crates/script/elidex-js/src/vm/host/range_proto_mutation.rs crates/script/elidex-js/src/vm/host/range_proto.rs ':!crates/dom/elidex-dom-api/src/range/tests.rs' | awk -F: '!($1 ~ /range\/live\.rs$/ && $2 >= 498) && !($1 ~ /selection\.rs$/ && $2 >= 676)'`.
The `dom.` receiver drops the VM's `create_element_wrapper` / `create_constructor_only_function` calls
(`range_proto.rs:176`, `:379`, …) and the `create_contextual_fragment` definitions (`range/mutation.rs:637`,
`range_proto_mutation.rs:250`). The pathspec drops `range/tests.rs`, and the `awk` filter drops the inline test
modules of `range/live.rs` (from `:498`) and `selection.rs` (from `:676`). After RC-0a moves those modules out, the
`awk` filter is no longer needed.

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
| A11 | RC-4, HTML §8.5.7 step 6 | the `body` context element | (planned) created in the start node's node document | C0's `EcsDom` entry for new nodes' document (PM 223) |
| A12 | RC-4 result | the DocumentFragment holding the parsed roots | (planned) node document = the start node's (P4) | C0's `EcsDom` entry for new nodes' document (PM 223) |

Not Range sites: the nodes the fragment parse creates (parser path, `html_fragment.rs` / the parser crates); RC-3
creates no node (`newParent` is the caller's); `selection.rs:687`, `:692` are test helpers.
