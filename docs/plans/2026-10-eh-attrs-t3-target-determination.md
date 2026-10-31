# T3 `EH-attrs-at-creation-T3` — *determine the target of an event handler* follows the spec (per-PR plan-memo)

Nested PR **T3** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in place by
#570 and to rev 8.4 in place by this PR, PM decisions 129 and 131; "the umbrella"). It closes slot
`#11-event-handler-target-determination` (all four facets), and, by PM decision 129, it also takes from slice b's S-5
the part that ends X-W for the pairs HEAD targeted at the element. It is the prerequisite x3 of the umbrella's slice b.
Siblings cited by ID: slice a (`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a
plan"), IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570,
implementation in flight; "the N1 memo"), and the IBP-sandbox node memo (`docs/plans/2026-09-ibp-sandbox.md`; "the
node memo"). C1 (`#11-template-contents-owner-document`, branch A, PM decision 123) has its plan in drafting; this memo
notes interactions with it only (§8.2).

**Placement (evidence).** T3 sits under the **EH-attrs umbrella**, not under IBP-sandbox:
- umbrella §1.2 nested-PR table, row **T3** `EH-attrs-at-creation-T3` ("owner: Layout lane, this program; PM decision
  25"); edges x3 (T3 → slice b, semantic precondition) and x6 (slice a → T3, scheduling) in §1.3; §8.1's first bullet;
- the ledger row (`project_open-defer-slots.md`, section "Event handlers — pre-existing slot", the sub-bullet **"Owner
  re-routed 2026-10-25 (PM decision 25)"**): "the owner is now the EH-attrs-at-creation program (Layout lane). T3 is a
  standalone prerequisite PR of slice b, outside the IBP-sandbox DAG". The row's first paragraph still reads "Owner:
  the IBP-sandbox program"; the sub-bullet supersedes it (§8.3 item 1 folds it at T3's landing);
- the N1 memo §7 row "*determine the target of an event handler*": "reassigned by PM decision 25 to the EH-attrs
  program, whose T3 PR carries facets 1, 3 and 4".

**Scheduling.** PM decisions 122/123, order unchanged: N1's implementation → T3 → C1 → slice b. T3's implementation
branch is cut from main **after N1's implementation lands**; this memo's code coordinates are `74e15df1` (pre-N1) and
are re-measured at that base (§9 item 1).

**Scope.**
- One function realises *determine the target of an event handler* (WHATWG HTML §8.1.8.1 Event handlers), steps 1–4,
  and every caller the spec gives it calls that function: the attribute change steps (content attributes, at every
  processing timing of slice a), the event handler IDL attribute getter and setter (both VM accessor pairs). Facets:
  (1) step 3, a node document that is not an active document → null; (2) step 4, the node document's relevant global
  object; (3) step 1's `frameset`; (4) step 2's Window-reflecting body element event handler set. Step 1's class test
  is HTML-namespaced (D-T5).
- **PM decision 129 (user, 2026-10-31; O-1 of rev 1):** X-W, slice a's rule that a Window-delegating pair is re-raw'd
  at every insertion, ends in T3 for the pairs HEAD targeted at the element: `body` × the six Window-reflecting names,
  and `frameset` × all 24 Window-delegating names. Those pairs are handed to the spec target at the spec's processing
  points (D-T7). The option text the user saw: 「HEAD より悪い状態が main に載らない。T3 は大きくなり、slice b の設計を少し先取りする。plan-review で確かめる」.
  X-W keeps only the pairs HEAD already targeted at the Window (`body` × the 18 WindowEventHandlers names) until slice
  b's S-5. The umbrella's slice-b scope is reduced to match, in place (§8.4).

**Not in scope.** X-W for `body` × WindowEventHandlers names, and the D-8 restriction (both slice b's S-5);
creation-time processing (slice b's S-1, which also closes residual R-C, §1.2); C1 (`#11-template-contents-owner-document`;
residual R-A, §1.2); insertion adoption (`#11-cross-document-adopt-on-insert`; residual R-B); `document.body`'s class
test (PM decision 133, D-T5); the attribute change steps' step 1 (`#11-event-handler-content-attribute-applicability`,
closed by slice b, D-T8); the `HTMLFrameSetElement` interface (`#11-htmlframesetelement-interface`, compat, PM
decision 130); multi-realm (B1; §8.2 hand-off).

**Changelog.** *Rev 2* (2026-10-31): PM decisions 129–133 and the scout's answers to rev 1's Q-1 to Q-6 folded in.
PM 129: D-T7 rewritten (X-W ends for the element-targeted pairs; class EW, §0.4), W1 / W2 and their window pins T-14 /
T-15 become spec-outcome cells, §2.1 enumerates every pair class × timing × document kind against HEAD and the spec,
and the umbrella is edited in place (§8.4). The enumeration finds four residual classes in which T3 is worse than HEAD
(R-A to R-D, §1.2), contrary to the premise of PM 129's option text; they go to the PM as O-6. PM 130:
`#11-htmlframesetelement-interface` text (§8.1). PM 131: the umbrella edits carry history. PM 132: D-T3 confirmed,
with the `#11-browsing-context-state-ecs-components` pointer. PM 133: D-T5 re-scoped; the `document.body` sibling and
cell T-13 are withdrawn. New slot `#11-event-handler-content-attribute-applicability`: closed by slice b (D-T8), text
in §8.1. The scout's answers became premises H-T15 to H-T23; premise errors P-T4 to P-T7 record what they overturned
in rev 1. *Rev 1* (2026-10-31, `e288fe0b`): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`74e15df1`** coordinate (origin/main, #570), measured on the worktree with
`git grep -n -E '<pattern>' 74e15df1 -- <paths>`, `git show 74e15df1:<path> | sed -n <range>p`, or `/usr/bin/grep -n`
(never the default ugrep). Spec text is `.claude/tools/webref body` output of 2026-10-31; every `§X.Y = title` pair is
`webref heading` / `dfn` output (§3's footnote). Paths are abbreviated after first use: `session/…` =
`crates/script/elidex-script-session/src/…`, `elidex-js/…` = `crates/script/elidex-js/src/…`, `ecs/…` =
`crates/core/elidex-ecs/src/…`. N1's implementation will move lines in `elidex-js/vm/host/event_handler_attrs.rs` and
`session/scripting.rs` (N1 memo §5.3); every coordinate in those two files is re-measured at T3's base (§9 item 1).

Symbols written as `Type` + method (`EcsDom::window_entity`, `HostData::window_entity`, `test_support::install`)
name existing methods or functions inside `impl` / `mod` blocks, which preflight's literal symbol grep does not find
(its soft warnings). `EcsDom::effective_node_document` (planned, N4) is the node memo's name for N4's future home of
D-T4's function.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `74e15df1` |
|---|---|---|---|
| H-T1 | One target function, two content-attribute callers | `git grep -n 'resolve_handler_target(' 74e15df1 -- crates` | defined `session/event_handler_processing/mod.rs:249`; called by `process_handler_content_attributes` `mod.rs:130` (all three timings: Bind, Insertion, HandlerWrite) and by the consumer's AttributeChange arm `session/event_handler_consumer.rs:420`. Nothing else |
| H-T2 | HEAD's target rule | `sed -n 244,262p session/event_handler_processing/mod.rs` | `scope == HandlerScope::Window && is_body_element(origin)` → `dom.window_entity().unwrap_or(origin)`, else `origin` (`:249-255`); `is_body_element` is a tag-only, ASCII-case-insensitive `body` test (`:258-262`). So: no step 3; step 4 = the World's one `NodeKind::Window` entity (`ecs/dom/mod.rs:450`, a linear scan); no `frameset`; the reflecting six (`HandlerScope::Global` rows) stay on the element |
| H-T3 | The IDL path determines its target separately | `sed -n 323,358p` and `sed -n 418,483p elidex-js/vm/host/event_handler_attrs.rs` | normal pair (`native_event_handler_get` `:323`, `native_event_handler_set` `:344`): target = the receiver, always. Body pair (`native_body_weh_get` `:439`, `native_body_weh_set` `:461`), installed on `HTMLBodyElement.prototype` for `HandlerScope::Window` rows only (`install_body_weh_delegation` `:207-214`, called once, `elidex-js/vm/host/html_passive_protos.rs:66`): target = `ctx.host().window_entity()` (`:448`, `:471`), no step 3. The reflecting six on a `<body>` use `HTMLElement.prototype`'s normal pair (`elidex-js/vm/host/html_element_proto.rs:134`) |
| H-T4 | Steps 1–2 already have one predicate, used by timings only | `sed -n 213,232p session/event_handler_processing/mod.rs` | `is_window_delegating_pair` (`:216`): `is_body_or_frameset` (`:228`, tag-only) ∧ (`HandlerScope::Window` row ∨ `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `:42`). Read by `process_handler_content_attributes` (`:111`), `process_before_handler_write` (`:156`) and `is_only_deliverable_ordinary_name` (`:208`), **not** by `resolve_handler_target`. At HEAD the X-W classifier and the target rule disagree on the reflecting six and on `frameset` |
| H-T5 | The spec's three callers | `.claude/tools/webref body html event-handler-attributes` | the getter steps 1–2 (determine; null → return null), the setter steps 1–2 (determine; null → return), the attribute change steps steps 2–3 (determine; null → return). Step 3's note: a `body` created in an active document but not connected still targets the Window |
| H-T6 | "Active document" in HEAD's model | `sed -n 12,82p session/scripting.rs`; `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 4; H-T15 | `scripting_disabled_for_platform_object`'s docstring (`:19-30`): "`bound_document` is the active document of the engine's (single) browsing context". With H-T15: in one `EcsDom` the bound document is the only document with a browsing context, and the one Window is its relevant global object (`webref body html navigables`: a navigable's active window "will always equal the navigable's active document's relevant global object") |
| H-T7 | The node-document derivation already has one rule | `sed -n 95,114p session/scripting.rs` | clause (b) derives the **effective node document** inline (`:104-110`): composed tree root if it is a Document, else `owner_document`; `None` → fail open. The node memo plans its promotion to `EcsDom::effective_node_document` by N4 (node memo §0.4 D8, §4 "Node document" row). `git grep -n 'find_tree_root_composed' 74e15df1 -- crates` → `scripting.rs:104` and `elidex-js/vm/host/node_proto.rs:455` (the `isConnected` getter) only |
| H-T8 | Insertion does not adopt | `sed -n 35,54p session/scripting.rs`; `webref body dom mutation-algorithms` (insert step "Adopt node into parent's node document") | `EcsDom::append_child` relinks without re-homing `AssociatedDocument`; slot `#11-cross-document-adopt-on-insert` |
| H-T9 | DOMParser documents are in the page `EcsDom`, built with the dispatcher detached, and reach the consumer after the build | `git show 74e15df1:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,200p` | `parse_into_inert_document` takes the dispatcher (`:112`), builds on `create_document_node()` (`:119`), restores it (`:198-199`); later `setAttribute` and inserts on its nodes fire the consumer (N1 memo H-P13) |
| H-T10 | The consumer has no bound document today | `sed -n 96,145p elidex-js/vm/consumer_dispatcher.rs`; `sed -n 215,316p elidex-js/vm/vm_api.rs` | `ConsumerDispatcher::new` builds `EventHandlerAttributeConsumer::with_policy(policy)` (`:110`); the bound `document` reaches the consumer only as `initialize_consumers(dom, document)` (`:141`, called `vm_api.rs:315`). The Window entity is resolved before the dispatcher is built (`vm_api.rs:224-230`) |
| H-T11 | `EcsDom::window_entity`'s production readers | `git grep -n -E '\.window_entity\(\)' 74e15df1 -- crates` minus `HostData` / `runtime` receivers | one: `resolve_handler_target` (`mod.rs:251`). Tests: `session/event_handler_processing/tests_insertion_and_write.rs:60`, `:101`, `:235` |
| H-T12 | The body-or-frameset class test is written three times | `git grep -n 'eq_ignore_ascii_case("body")' 74e15df1 -- crates` | `mod.rs:231` (`is_body_or_frameset`), `mod.rs:261` (`is_body_element`), `crates/dom/elidex-dom-api/src/char_data/document_props.rs:173` (`first_body_or_frameset_child`, the `document.body` getter). None checks the HTML namespace; the precedent `EcsDom::is_base_element` (`ecs/dom/classify.rs:39-46`) does, through `EcsDom::is_html_namespace` |
| H-T13 | Slice a's window pins that name T3 | `git grep -n 'T3' 74e15df1 -- crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs` | C17 (`:39`), C18 (`:64`), C19b (`:122`), C19c (`:132`), P3 (`:172`) say "reached after T3 and slice b" or "T3 facet 3" |
| H-T14 | `<body onload>` never runs at HEAD | `git show 74e15df1:crates/shell/elidex-shell/src/pipeline.rs \| sed -n 462,474p` | the document `load` event is dispatched at the Window entity (`:470`), non-bubbling; `<body onload>` lands on the body element (H-T2), so it never runs. Same for an initial `<body onerror\|onfocus\|onblur\|onresize\|onscroll>` against Window-targeted events |
| H-T15 | (rev 1 Q-1) One bound document and one Window per `EcsDom`; other Documents have no Window | `git grep -n 'create_window_root()' 74e15df1 -- crates`; `git grep -n 'create_document_node()' 74e15df1 -- crates`; `git show 74e15df1:crates/script/elidex-js/src/vm/host_data/mod.rs \| sed -n 1690,1716p` | the Window root is created at one site, `vm_api.rs:227`, only when `HostData::window_entity` is `None`; `HostData::set_window_entity` asserts on a second set (`host_data/mod.rs:1710-1715`). Every iframe, navigation and worker gets a new `EcsDom` and `Vm` (scout, 2026-10-31: `elidex-shell/src/pipeline.rs:226`, `:735`; `content/iframe/load.rs:465-484`; `content/navigation.rs:276-290`; `app/navigation.rs:323`; workers use `create_worker_global_scope_root`). Production `create_document_node()` callers: `inert_document.rs:119` (DOMParser: a **second, unbound, windowless Document in the page `EcsDom`**, persistent) and `ecs/dom/tree/teardown.rs:294` (the fragment parser's throwaway document, despawned at `finish_detached_fragment`, `:321-333`). **Caveat (a stated defect, not T3's):** `HostData::window_entity` is initialised `None` once (`host_data/mod.rs:856`) and is **not cleared on unbind** (`git grep -n -E 'window_entity\s*=\s*None' 74e15df1 -- crates/script/elidex-js/src` → none), so a `Vm` re-bound to a different `EcsDom` would keep a Window entity of the old World. "One World per `Vm`" is a caller contract (doc comments), not enforced. No production path rebinds across `EcsDom`s (above). Unverified: whether `elidex-wasm-runtime`'s bind (`host/state.rs:50`, `:125`) shares the JS `EcsDom` |
| H-T16 | (rev 1 Q-2) Which nodes have no resolvable node document | `git show 74e15df1:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 328,362p` and `sed -n 806,850p`; `git grep -n 'create_element_with_owner' 74e15df1 -- crates/dom/elidex-dom-api/src/document.rs`; `git show 74e15df1:crates/core/elidex-ecs/src/dom/tree/teardown.rs \| sed -n 303,334p` | `EcsDom::create_element` passes no owner (`:335-337`); `owner_document` falls back to the tree root (`:839-`), so a node with no `AssociatedDocument` is unresolvable only while it is detached. The VM's `createElement` passes the receiver document (`elidex-dom-api/src/document.rs:235`), so `dp.createElement('body')` is owned by `dp`. Fragment-parse and DOMParser nodes are re-homed into the context's owner document by `finish_detached_fragment` (`teardown.rs:327-329`, `EcsDom::adopt_subtree`); the adopt is skipped only for a documentless context, which then yields orphans. So an unresolvable `body` / `frameset` comes only from an owner-less `create_element` caller or a documentless fragment context, both in the bound realm |
| H-T17 | (rev 1 Q-3) A `frameset` wrapper's prototype | `git show 74e15df1:crates/script/elidex-js/src/vm/host/elements.rs \| sed -n 108,125p` and `sed -n 186,275p` | no `frameset` arm in `tag_specific_html_prototype` / `tag_specific_t2b_prototype`, so `HTMLElement.prototype` (`:121-123`). T-7's `f.onload` reaches `HTMLElement.prototype`'s normal pair |
| H-T18 | (rev 1 Q-4) Foreign elements take HTML prototypes | `git show 74e15df1:crates/script/elidex-js/src/vm/host_data/mod.rs \| sed -n 1580,1628p` | `prototype_kind_for` returns `PrototypeKind::Element` for every `TagType` entity (`:1626-1628`); `tag_matches_ascii_case` (`:1587-1596`) reads no namespace. Foreign elements carry `TagType` (`create_element_ns` → `create_element_with_owner`, `ecs/dom/mod.rs:374-381`, `:352-357`). So an SVG `body` gets `HTMLBodyElement.prototype` with the body pair (`elements.rs:264-266`), engine-wide for every tag-specific prototype |
| H-T19 | (rev 1 Q-5) The AttributeChange event carries no namespace | `git show 74e15df1:crates/core/elidex-ecs/src/dom/mutation_event.rs \| sed -n 306,311p` | `AttributeChange { node, name, old_value, new_value }`; emitters `ecs/dom/attribute.rs:135`, `:321` (scout). The attribute change steps' step 1 "namespace is not null" cannot be tested at the consumer (D-T8) |
| H-T20 | (rev 1 Q-6) The shell harness runs the `load` dispatch | `git show 74e15df1:crates/shell/elidex-shell/src/content_test_support.rs \| sed -n 377,399p`; `pipeline.rs:374`, `:432-478` | `build_main_document` → `build_pipeline_from_loaded` → `run_scripts_and_finalize` (`pipeline.rs:374`) → `dispatch_lifecycle_events` (`:432-478`), which dispatches `load` at the Window (H-T14). T-E1 observes it (scout, confidence high-medium; T-E1's red run is the check) |
| H-T21 | Template contents report the bound document | `git show 74e15df1:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 540,591p`; `git show 74e15df1:crates/core/elidex-ecs/src/components.rs \| sed -n 360,369p` | `attach_template_contents` gives the contents fragment the template's own node document (the C2 approximation, `:552-557`); `TemplateContents { fragment }` links template → fragment only, no reverse fact. A script-created `body` appended into `tpl.content` keeps its `AssociatedDocument` = the bound document (H-T8); its composed root is the fragment, not a Document, so the effective node document (H-T7) is the bound document. Spec: the contents' node document is the *appropriate template contents owner document*, whose "browsing context is null" (`webref body html the-template-element`), so never active |
| H-T22 | SVG elements accept every event attribute | `.claude/tools/webref body SVG2 EventAttributes` | SVG2 §15.8 Event attributes: "For every event type that the user agent supports, SVG supports that as an event attribute … The event attributes are available on all SVG elements". So `onhashchange` on an SVG `body` is an event attribute of that element (target: the element, step 1) |
| H-T23 | MathML elements' event attributes | `.claude/tools/webref body mathml-core global-attributes`; `.claude/tools/webref idl mathml-core MathMLElement` | MathML Core §2.1.3 Global Attributes lists "on* event handler attributes"; `MathMLElement includes GlobalEventHandlers`. Whether WindowEventHandlers names are among them is **not determined** from webref (no dfn) |

### §0.3 Premise errors (reported)

- **P-T1.** The ledger row's coordinate "`resolve_handler_target` (`…/event_handler_processing/mod.rs:249`) … code as of
  `fd6955e8`" mixes frames: at `fd6955e8` the function is `session/event_handler_consumer.rs:328`
  (`git show fd6955e8:crates/script/elidex-script-session/src/event_handler_processing/mod.rs` fails: the file was added by
  #563); `mod.rs:249` is the `3cd84a04` / `74e15df1` position. §8.3 item 1 carries the correction (a landing-time record).
- **P-T2.** The slot records **four** facets, not three (facet (4), the reflecting set, was added 2026-10-25), and its
  owner is the EH-attrs program (re-routed sub-bullet), although its first paragraph still says IBP-sandbox.
- **P-T3.** The Reachability example (`onload` → `onhashchange`) is already corrected in the ledger as read on
  2026-10-31 (line 807 reads `setAttribute('onhashchange', …)`).
- **P-T4.** The scout's row "DOMParser / createHTMLDocument: no second document in production" is **wrong**: DOMParser
  (`elidex-js/vm/host/dom_parser.rs:292-297` → `parse_into_inert_document`) creates a second, unbound, windowless
  Document in the page `EcsDom` (`inert_document.rs:119`). It does not change Q-1's answer about bound documents and
  Windows (H-T15). `createHTMLDocument` does not exist (`git grep -n -i 'createHTMLDocument' 74e15df1 -- crates` → none).
- **P-T5.** Rev 1 §1 said every outcome change at T3 is toward the spec except the O-1 window, and §2 A3×A8 said a
  template-contents `body` keeps HEAD's outcome. Both are wrong for the pairs HEAD targeted at the element: in template
  contents T3 writes them to the Window, where HEAD wrote the element and the spec writes nothing (H-T21; residual R-A,
  cells EW-13, EW-14). C18's pair (a WindowEventHandlers name on `body`) does keep HEAD's outcome.
- **P-T6.** Rev 1 R2 said each proxy facet of step 3 gives HEAD's target or the spec's. Wrong: a DOMParser `body`
  appended into a detached element of the bound document is adopted into the bound document in the spec, but its
  effective node document stays the DOMParser document (H-T7, H-T16), so T3 returns null where HEAD and the spec give the
  Window (residual R-B, cell XW-9). The reverse facet writes the Window for an element-targeted pair where neither does
  (EW-16).
- **P-T7.** Rev 1 §3.1 said no case turns a null into a delivery except W2. Residuals R-A (EW-13) and R-B (EW-16) do.

### §0.4 Decisions (this PR)

Pair classes used below (all on HTML-namespace elements unless stated): **XW** = `body` × the 18 WindowEventHandlers
names, the pairs HEAD already targets at the Window; **EW** = `body` × the 6 Window-reflecting names and `frameset` × all
24 Window-delegating names, the pairs HEAD targets at the element (PM decision 129's population); **FN** = a
foreign-namespace `body` / `frameset` with any of those 24 names; **OR** = every other pair.

- **D-T1 — one function, every spec caller.** `determine_event_handler_target(dom, target, name, active) ->
  Option<Entity>` (NEW), in `session/event_handler_processing/mod.rs`, `pub`, is the only realisation of the algorithm.
  `None` is the spec's null. Callers: the attribute change steps at `process_handler_content_attributes` (every timing)
  and the AttributeChange arm; the VM getter and setter, both pairs. `resolve_handler_target` and `is_body_element` are
  deleted. The body pair keeps its own receiver brand check (`require_body_receiver`; WebIDL `HTMLBodyElement`) but no
  longer chooses a target. The VmObject handler backend (`native_vm_event_handler_get` / `_set`) does not call it: its
  receivers are never `body` / `frameset` elements, so step 1 returns the receiver by type (stated in its docstring).
- **D-T2 — steps 1–2 are `is_window_delegating_pair`.** The function calls slice a's classifier for steps 1–2 and returns
  `Some(target)` when it is false. The X-W classifier and the target rule then agree by construction (H-T4's
  disagreement is the defect of facets (3) and (4)). Step 2's name test is by name: the `scope` parameter leaves target
  determination (`HandlerScope` stays the IDL-install key).
- **D-T3 — steps 3–4 over a marshalled `ActiveDocument` (NEW); PM decision 132.** `pub struct ActiveDocument { document:
  Entity, window: Entity }`: the active document of the engine's single browsing context and its relevant global object
  (H-T6, H-T15). Step 3: the target's effective node document (D-T4) equals `active.document`, else `None`. Step 4:
  `active.window`. The consumer receives it at construction from `Vm::bind` (`document` and the resolved Window, H-T10);
  the VM accessors build it from `HostData` (`document_entity_opt()`, `window_entity()`). `EcsDom::window_entity` loses
  its last production reader and is deleted (H-T11; dead-code rule). **PM decision 132:** marshalling is accepted; the
  active document and its Window are browsing-context state, not a per-element fact, so the side-store→component rule
  does not apply, and their move into ECS components is `#11-browsing-context-state-ecs-components`, decided with B1.
  *Rejected:* (A) a component relating the Document and its Window, written at bind (it duplicates `HostData`'s
  `document_entity` / `window_entity` pair, which B1 §5 req 5 migrates as one cluster); (D) `document_root()` keying
  (slice a's D-5 / cell U12; the node memo's "No reader keys on `document_root()`"). *Relied-on caveat* (H-T15):
  `ActiveDocument.window` comes from `HostData::window_entity`, which unbind does not clear; T3 is correct only under
  the caller contract "one `EcsDom` per `Vm`", which every production bind path keeps. B1 makes it structural (§8.2).
- **D-T4 — one node-document derivation.** The effective-node-document rule (H-T7) is extracted from clause (b) into
  `effective_node_document(dom, node) -> Option<Entity>` (NEW), a `pub` free function in `session/scripting.rs`, called by
  clause (b) (code-identical) and by D-T1's step 3. Unresolvable (`None`) → treated as `active.document`, the direction of
  clause (b)'s fail-open on the same ground ("such nodes belong to the bound document's realm by construction",
  `scripting.rs:55-58`). H-T16 measures that ground for `body` / `frameset`: only bound-realm nodes are unresolvable, so the
  direction gives the spec's Window (cells XW-13, EW-18). N4 keeps its row: it moves this one function into
  `EcsDom::effective_node_document` in `ecs/dom/document_policy.rs` as the node memo plans; no ratified text is edited.
  The derivation is not refined for template contents or for cross-document moves (R-A, R-B): each refinement either
  changes N1's clause (b) or duplicates C1 / insertion adoption (§1.2, O-6). *Rejected:* landing
  `EcsDom::effective_node_document` here (it edits N4's approved row), and `owner_document` alone (wrong in both
  directions for a connected node, H-T8; cell T-3 kills it).
- **D-T5 — step 1's class test is HTML-namespaced; `document.body` is not touched (PM decision 133).** The session's
  `is_body_or_frameset` (`mod.rs:228`) gains `EcsDom::is_html_namespace` (the `is_base_element` precedent) and stays
  session-private; `is_body_element` is deleted (D-T1); no `EcsDom` symbol is added. PM decision 133 unifies with
  `document.body`'s predicate only if T3's step 3 or 4 uses it; neither does (step 3 reads the node document, step 4 the
  global object; "the body element" of HTML §3.1.7 DOM tree accessors is not read). So
  `first_body_or_frameset_child` (`document_props.rs:169-173`) keeps its tag-only test, rev 1's cell T-13 is withdrawn,
  and the `document.body` defect is recorded for the PM (§8.1 item 3). The namespace stays in step 1 because T3 itself
  needs it: under a tag-only test an SVG `body`'s `onfocus` would move from the element (HEAD's and the spec's target)
  to the Window (cell T-9b). The VM's prototype selection and `require_body_receiver` (`tag_matches_ascii_case`) are
  interface brand checks, not this predicate, and are not touched (H-T18; residual R-D). The reading of PM 133 that
  keeps the namespace check is asked back as O-5.
- **D-T6 — order inside the IDL setter.** receiver brand check → determine (D-T1) → `None`: return → slice a's
  `marshal_before_handler_write` (D-6) → activate. Determination precedes D-6 per the setter's steps 1–2; the reorder
  changes nothing observable, because `None` occurs only for Window-delegating pairs, for which
  `process_before_handler_write` returns at once (`mod.rs:156`). The getter: brand check → determine → `None`: null →
  read (N1's reconcile, unchanged).
- **D-T7 — X-W ends for EW; XW keeps it until S-5 (PM decision 129).** `process_handler_content_attributes` delivers a
  Window-delegating pair by class and timing:

  | class | Bind (T-a1) | Insertion (T-a2) | HandlerWrite (T-a3) | attribute change steps (AttributeChange arm), IDL setter |
  |---|---|---|---|---|
  | XW | delivered (X-W) | delivered at every insertion (X-W, until S-5) | not processed | T3's target |
  | EW | delivered, once per document (the bind pass's own once rule, D-5) | **not delivered** | not processed | T3's target |
  | OR | once (K-1) | once, if unmarked (K-1) | D-8 | element |

  The X-W population is named by `is_x_w_pair(dom, element, name)` (NEW, session-private): an HTML `body` and a
  `HandlerScope::Window` name. Its docstring names S-5 as its remover. EW's creation-time attributes on an element created
  after bind (by DOMParser, by a fragment parse, by a clone) are therefore **not processed** until slice b's S-1. That is
  the spec's outcome wherever the element's node document at creation is not active (the DOMParser build, the fragment
  parser's throwaway document, a clone owned by a DOMParser document: cells EW-10, EW-11, T-15b), and residual R-C where
  it is active (a clone or import into the bound document: EW-12). EW thus satisfies K-1 to K-3 unconditionally from T3:
  nothing derived from an EW attribute is written after the bind pass except by the attribute change steps and the IDL
  setter, which are the spec's writers.
  *Rejected:* (α) EW joining the ordinary pairs' once rule (processed at the first insertion if unmarked). The first
  insertion reads the target at the insertion, not at the element's creation, so a DOMParser or fragment-parsed `body`'s
  attribute reaches the page Window at its first insertion into the active document (W2, cell T-15b's mutant), and an
  unmarked element's attribute overwrites a `window.on<T>` written before that insertion (W1 on the D-8-failing and
  clone paths, U-T6). (β) A timing-dependent target (two target rules). Rev 1's form (X-W kept for EW, onto T3's
  target) is withdrawn by PM decision 129.
- **D-T8 — `#11-event-handler-content-attribute-applicability` is closed by slice b, not T3.** (i) T3 does not need it to
  be no worse than HEAD: on every HTML element other than `body` / `frameset` T3's target is HEAD's (class OR, cell OR-1),
  and on SVG elements every event type is an event attribute (H-T22), so T3's namespaced step 1 sends an SVG `body`'s
  WindowEventHandlers attribute to the element, which is the spec's target (FN-2). (ii) Step 1 of the attribute change
  steps is the guard of D-2's processing function, which slice b extends to every creation path (S-1); closing it there
  covers every caller at once, where closing it in T3 would cover today's three and leave slice b to re-touch its new
  ones. (iii) Its attribute-namespace facet needs `MutationEvent::AttributeChange` to carry a namespace (H-T19) and the
  silent writers' namespaces, which slice b's S-7 census re-runs. The MathML facet (H-T23) is part of the slot (FN-4).
- **D-T9 — `frameset`'s interface is compat (PM decision 130).** `#11-htmlframesetelement-interface` is registered in
  the compat layer: `frameset` is an obsolete feature (HTML §16.3.2 Frames, under §16.3 Requirements for
  implementations), so its interface belongs to a compat plugin, not core (CLAUDE.md core/compat/deprecated). Owner: none
  until the compat plugin work starts. T3 changes only target determination for a `frameset`; its interface is not
  added.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| A1 class (step 1) | `body` / `frameset`, HTML namespace (D-T5) |
| A2 names (step 2) | WindowEventHandlers members ∪ the reflecting six = `is_window_delegating_pair` (D-T2) |
| A3 activeness (step 3) | effective node document (D-T4) == `active.document`, else null |
| A4 global (step 4) | `active.window` (D-T3) |
| A5 callers | attribute change steps at every timing + AttributeChange arm; getter; setter, both pairs (D-T1, D-T6) |
| A6 processing timing of Window-delegating pairs | XW: X-W until S-5; EW: bind pass and the spec's writers only (D-T7) |

Coupled from outside: A7 N1's compile-gate node clause (same derivation, D-T4), A8 C1 (template contents report the
bound document, H-T21), A9 insertion adoption (`#11-cross-document-adopt-on-insert`, H-T8).

### §1.1 Outcome changes at T3 (toward the spec)

- WindowEventHandlers name on a `body` whose effective node document is not active (a DOMParser document, a node
  inserted into one): HEAD → page Window; T3 → null at every caller (facet 1; XW-3 to XW-5, XW-7).
- EW in the active document: HEAD → the element; T3 → the Window (facets 3, 4; EW-1 to EW-4). In a non-active document:
  HEAD → the element; T3 → null (EW-5, EW-6).
- EW is never re-raw'd at insertion (PM decision 129): a `window.on<T>` write survives a later insertion of the element
  (rev 1's W1, now cell T-14), and an attribute set while the element was outside an active document is not delivered
  at its insertion (rev 1's W2, now T-15; T-15b for a DOMParser-built attribute).
- An initial `<body onload>` (and the other five, and an initial `<frameset>`'s 24) runs on the Window (H-T14; closes
  umbrella K-0's E-G1 (i)).
- A foreign-namespace `body` is not step 1's class (FN-1, FN-2).

### §1.2 Is T3 no worse than HEAD? (PM decision 129's premise)

Criterion: the umbrella's K-0 — at every observable step (an IDL getter read, a dispatch, a read of `window.on<T>`), T3's
outcome equals HEAD's, or equals the spec's where HEAD's does not. §2.1 enumerates every pair class × timing × document
kind. Every XW, EW, FN and OR row meets the criterion **except four residual classes**, each read through a
pre-existing gap that T3 does not own:

- **R-A — template contents (C1 gap; EW-13, EW-14; pin T-17).** An EW pair written on a `body` / `frameset` inside
  template contents (or inside a clone of them) goes to the **Window**: elidex reports the bound document for template
  contents (H-T21), so step 3 passes. HEAD wrote the element; the spec writes nothing (inert document). A `window.on<T>`
  read and a Window dispatch differ from both. XW pairs keep HEAD's outcome there (C18).
- **R-B — a cross-document move followed by a detached state (insertion-adoption gap; XW-9, EW-15, EW-16; pin T-18).**
  The effective node document of a detached node is its stale owner (H-T8). A DOMParser `body` appended into a detached
  element of the bound document: T3 → null; HEAD and the spec → the Window for XW (XW-9); for EW the Window is HEAD's
  (untouched) but the getter reads null where HEAD and the spec read the attribute (EW-15). A bound `body` appended into
  a DOMParser document and removed again: T3 writes an EW attribute to the Window, where HEAD wrote the element and the
  spec writes nothing (EW-16).
- **R-C — a clone created in the active document (creation-time gap; EW-12; pin T-16).** `cloneNode` of a bound
  `body` / `frameset`, or `importNode` into the bound document, carrying an EW attribute: the spec writes the Window at
  the clone's creation; T3 writes nothing (D-T7), so `window.on<T>` keeps HEAD's value, but the clone's getter reads the
  Window's value, where HEAD reads the attribute's function after the clone's first insertion (and null before it). The
  getter differs from both whenever the Window's value is not the attribute's function. Rejected alternative (α, D-T7)
  turns this into a Window overwrite (W1 on the clone path), which PM decision 129 rules out.
- **R-D — foreign-namespace interfaces and MathML applicability (FN-3, FN-4; pin T-19).** An SVG `body` has
  `HTMLBodyElement.prototype`'s WindowEventHandlers accessors (H-T18). `s.onhashchange = fn` writes a handler on `s` at
  T3 (HEAD: on the Window; spec: an ordinary property, since `SVGElement` has no such attribute), so a `hashchange`
  dispatched at `s` runs `fn`. A MathML `body` with a WindowEventHandlers attribute goes to the element at T3 (HEAD: the
  Window); the spec's outcome depends on MathML Core's event-attribute set (H-T23).

So **PM decision 129's premise "HEAD より悪い状態が main に載らない" does not hold for T3 as scoped**: R-A to R-D remain.
Each is pinned (§6.1 T-16 to T-19), recorded (§8.1, §8.3), and sent to the PM as **O-6** with its options. W1 and W2,
the windows PM decision 129 targeted, are closed.

### §1.3 Edge density

**Edge-dense** (*verdict*). Six own axes meet in one algorithm with three spec callers, and three external couplings
(A7–A9) change outcomes at the intersections (§2: R-A at A3×A8, R-B at A3×A9, C8h's mechanism at A3×A7). So CLAUDE.md
"Edge-dense work" applies: plan-review is mandatory (rule (b)). **Split: none** — T3 is a narrowly-scoped per-PR slice
under the approved EH-attrs umbrella (§1.2 row T3, as edited by PM decision 129), the rule's base case (c). Splitting by
facet or by caller would leave two target rules live (content attribute vs IDL, or body vs frameset), and splitting
D-T7 off would land the target change without the timing change, which is rev 1's W1 / W2 state that PM decision 129
rejects. Breadth: K = 2 specs, M = 9 rows (§3); `webref coverage-map html 8.1.8.1 html 8.1.8.2 html 7.3.2 html
8.1.3.3.4 dom 4.4 html 16.3.2` → "ok → single PR scope".

## §2. Coupled invariants

- **A1×A2** — steps 1–2 are one conjunction (`is_window_delegating_pair`); a frameset passes A1 for every A2 name.
- **A1×A5** — every caller reaches A1 through D-T1 only; the VM brand checks are separate (interfaces, D-T5, R-D).
- **A1×A6** — `is_x_w_pair` reads the same namespaced class (`body` half), so X-W's population is a subset of A1×A2.
- **A2×A5** — the IDL normal pair now asks A2 too: `body.onfocus` (HTMLElement accessor) → Window (cell T-5).
- **A2×A6** — every Window-delegating pair targets the Window or null; XW is re-raw'd at insertion, EW is not.
- **A3×A5** — null at each caller: content attribute → no write and no clear; getter → null; setter → return.
- **A3×A6** — an XW re-raw at an insertion into a non-active document delivers nothing (T-3); into the active one, the
  Window (C8). EW has no insertion delivery.
- **A3×A7** — one derivation (D-T4): N1's compile gate and step 3 cannot disagree on a node's document.
- **A3×A8** — template contents report the bound document: XW keeps HEAD's outcome (C18); EW is R-A.
- **A3×A9** — no insertion adoption: R-B.
- **A4×A5** — `ActiveDocument` has two marshalling points (consumer construction, VM `HostData`), one source each.
- **A2×A7** — N1's C8h (`dd.body.setAttribute('onfocus', 'A')`) keeps its outcome (getter null, nothing runs) but by
  step 3 now: no entry is written (cell U-T3).
- **A5×A7** — the getter determines before N1's reconcile (getter step 1 before step 3).

### §2.1 Enumeration: pair class × timing × document kind (HEAD / T3 / spec)

Document kinds: **A** active (connected to the bound document, or detached with it as owner, never moved across
documents); **P** a DOMParser document (connected to it, or detached with it as owner); **T** template contents; **TC**
a clone of template contents; **X1** a DOMParser-owned node in a detached subtree of the bound document; **X2** a bound
node moved into a DOMParser document and removed again; **U** no resolvable node document (H-T16). Timings: **bind**
(T-a1); **AC** (`setAttribute` / `removeAttribute`, the attribute change steps); **set** / **get** (IDL); **ins→A** /
**ins→P** (an insertion into that kind, first or later); **parse** (creation by the DOMParser build or a fragment
parse); **clone** (creation by `cloneNode` / `importNode`); **T-a3** (slice a's first-write processing). Verdict:
**both** (T3 = HEAD = spec), **spec** (T3 = spec ≠ HEAD), **HEAD** (T3 = HEAD ≠ spec), **✗** (neither).

| Row | Class | Timing | Doc | HEAD | T3 | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| XW-1 | XW | bind | A | Window | Window | Window | both | E4 (slice a) |
| XW-2 | XW | AC, set, get | A | Window | Window | Window | both | C20 (i) |
| XW-3 | XW | AC | P | page Window | nothing | nothing | spec | T-2 |
| XW-4 | XW | set | P | page Window | return | return | spec | T-1, C17 |
| XW-5 | XW | get | P | the Window's value | null | null | spec | T-1 |
| XW-6 | XW | ins→A | any | re-raw onto the Window over any write | same (X-W) | nothing | HEAD | C8, C17, U2 |
| XW-7 | XW | ins→P | any | re-raw onto the page Window | nothing | nothing | spec | T-3, C19 |
| XW-8 | XW | AC, set, get | T, TC | Window | Window | null | HEAD | C18 |
| XW-9 | XW | AC, set | X1 | Window | nothing | Window | **✗ R-B** | T-18 |
| XW-10 | XW | AC, set | X2 | Window | Window | nothing | HEAD | — |
| XW-11 | XW | clone | A | nothing at creation; re-raw at insertion | same | Window at creation | HEAD (umbrella defect (3)) | — |
| XW-12 | XW | parse | P | nothing at creation; re-raw at ins→A | nothing at creation; re-raw at ins→A | nothing | HEAD (XW-6) | C17 |
| XW-13 | XW | AC, set | U | Window | Window | Window | both | U-T1 |
| EW-1 | EW | bind | A | element | Window | Window | spec | T-E1, U-T2 |
| EW-2 | EW | AC | A | element | Window | Window | spec | T-4, T-6, T-8b, C19b, C19c, P3 |
| EW-3 | EW | set | A | element | Window | Window | spec | T-5, T-7 |
| EW-4 | EW | get | A | the element's value | the Window's value | the Window's value | spec | T-4, T-5 |
| EW-5 | EW | AC, set | P | element | nothing / return | nothing / return | spec | T-6b, U-T3, C19b |
| EW-6 | EW | get | P | the element's value (from N1: null for a raw value) | null | null | spec | N1's C8h |
| EW-7 | EW | ins→A after an AC / set | any | re-raw onto the element over any write | nothing | nothing | spec | T-14 (rev 1's W1) |
| EW-8 | EW | ins→A after an AC while in P | P→A | re-raw onto the element | nothing | nothing | spec | T-15 (rev 1's W2) |
| EW-9 | EW | ins→P | any | re-raw onto the element | nothing | nothing | spec | C19b |
| EW-10 | EW | parse, then ins→A | P | element at the first insertion | nothing | nothing (null at creation) | spec | T-15b |
| EW-11 | EW | clone, then ins→A | P (clone owned by P) | element at the first insertion | nothing | nothing | spec | U-T6 (first-insertion row) |
| EW-12 | EW | clone | A | Window: untouched. Clone's getter: null, then the attribute's function after its first insertion | Window: untouched. Clone's getter: the Window's value | Window := the attribute at creation | Window: HEAD. Getter: **✗ R-C** unless the Window's value is the attribute's function | T-16 |
| EW-13 | EW | AC, set | T, TC | element | **Window** | nothing | **✗ R-A** | T-17 |
| EW-14 | EW | get | T, TC | the element's value | the Window's value | null | **✗ R-A** where the Window's value is not null | T-17 |
| EW-15 | EW | AC, set | X1 | element | nothing | Window | Window: HEAD. Getter: **✗ R-B** (null where both read the attribute) | — |
| EW-16 | EW | AC, set | X2 | element | **Window** | nothing | **✗ R-B** | — |
| EW-17 | EW | T-a3 | any | not processed | not processed | — | both (D-8) | C20 (i), U-T6 |
| EW-18 | EW | AC, set | U | element | Window | Window | spec | U-T1 |
| FN-1 | FN (SVG; reflecting six on `body`, any name on `frameset`) | AC, set | any | element | element | element (H-T22) | both | T-9b |
| FN-2 | FN (SVG `body`, WindowEventHandlers name) | AC | any | Window (tag-only) | element | element (H-T22) | spec | T-9 |
| FN-3 | FN (SVG `body`, WindowEventHandlers name) | set (body pair, H-T18) | any | Window | a handler on the element | an ordinary property, no handler | `window.on<T>`: spec. Dispatch at the element: **✗ R-D** | T-19 |
| FN-4 | FN (MathML `body`, WindowEventHandlers name) | AC | any | Window | element | not determined (H-T23) | **✗ R-D** if MathML excludes the name | — |
| OR-1 | OR | every timing | any | element | element (D-T1 steps 1–2) | element, or nothing where step 1 of the attribute change steps returns | HEAD or both (D-T8) | U-T1 |

Rows not listed are covered by a listed one: `removeAttribute` follows its row's AC column (clear instead of set, on
the same target); a later bind processes nothing for every class (D-5); XW and EW in a non-active document at the bind
pass do not occur (the bind pass walks only the bound document, umbrella K-5).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 1–4 | 1 not an HTML body/frameset → self; 2 not WEH and not reflecting → self; 3 effective node document ≠ active document → null; 4 → active Window | `determine_event_handler_target` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, replacing `resolve_handler_target` (`:249`) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute getter steps 1–3 | null target → null; else get the current value (N1's reconcile) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `native_event_handler_get` `:323`, `native_body_weh_get` `:439` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter steps 1–4 | null target → return; null value → deactivate; else activate | `native_event_handler_set` `:344`, `native_body_weh_set` `:461` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps steps 2–5 | null target → return (no set, no clear); step 1 and step 5.1 not touched (§7) | `crates/script/elidex-script-session/src/event_handler_consumer.rs:420`; `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:120-132` (D-T7's delivery by class and timing; `is_x_w_pair` (NEW)) | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the WEH table and the Window-reflecting body element event handler set | content and IDL attributes on body and frameset act on the Window | `is_window_delegating_pair` `mod.rs:216`; `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `mod.rs:42` (unchanged) | ✓ | yes |
| WHATWG HTML §7.3.2 Browsing contexts | *active document* | in HEAD's model: the bound document (H-T6, H-T15) | `ActiveDocument` (NEW), marshalled at `crates/script/elidex-js/src/vm/vm_api.rs:292` and in `event_handler_attrs.rs` | ✗ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | *relevant global object* of the node document | active document → its Window (navigable's active window, §7.3.1) | `ActiveDocument::window` (NEW) | ✗ | no |
| WHATWG DOM §4.4 Interface Node | *node document* | effective node document (composed root Document, else `owner_document`; `None` → active) | `effective_node_document` (NEW) extracted from `crates/script/elidex-script-session/src/scripting.rs:104-110` | ✗ | yes |
| WHATWG HTML §16.3.2 Frames | `frameset` element; `HTMLFrameSetElement includes WindowEventHandlers` | content attributes and inherited GlobalEventHandlers accessors → step 1; the interface itself absent (compat, D-T9) | `is_body_or_frameset` `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:228` (namespaced, D-T5) | ✗ | yes |

Numbers and titles (2026-10-31): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `8.1.8.2.1` (IDL definitions), `7.3.2`
(Browsing contexts, `#windows`), `7.3.1` (Navigables), `8.1.3.3.4` (Relevant), `16.3` (Requirements for
implementations), `16.3.2` (Frames), `3.1.7` (DOM tree accessors), `4.12.3` (The template element), `7.2.2` (The Window
object), `8.1.3.4` (Enabling and disabling scripting); `webref heading dom 4.4` (Interface Node), `4.2.3` (Mutation
algorithms), `4.5` (Interface Document), `4.9` (Interface Element); `webref heading SVG2 15.8` (Event attributes,
`#EventAttributes`); `webref heading mathml-core 2.1.3` (Global Attributes). `webref dfn html`: 'determine the target of
an event handler' → §8.1.8.1 `#determining-the-target-of-an-event-handler`; 'Window-reflecting body element event
handler set' → §8.1.8.2; 'active document' → §7.3.2 `#active-document` (and for=navigable §7.3.1 `#nav-document`);
'relevant global object' → §8.1.3.3.4 `#concept-relevant-global`; 'associated Document' → §7.2.2
`#concept-document-window`; 'appropriate template contents owner document' → §4.12.3
`#appropriate-template-contents-owner-document`; 'WindowEventHandlers' → §8.1.8.2.1; 'frameset' → §16.3.2 `#frameset`;
`webref dfn dom`: 'node document' → §4.4 `#concept-node-document`, 'clone a node' → §4.4 `#concept-node-clone`, 'adopt' →
§4.5 `#concept-node-adopt`. Step texts: `webref body html event-handler-attributes`,
`event-handlers-on-elements,-document-objects,-and-window-objects`, `frames`, `navigables`, `relevant`,
`the-template-element`; `webref body dom mutation-algorithms`; `webref body SVG2 EventAttributes`; `webref body
mathml-core global-attributes`; `webref idl html HTMLFrameSetElement`, `webref idl mathml-core MathMLElement`, `webref
idl SVG2 SVGElement`.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler content attributes on `body` / `frameset` (parser-baked
and `setAttribute`; in a DOMParser document they reach the consumer only after the build, H-T9); IDL handler writes on
`body` / `frameset` / any element (`el.on<T> = …`); `DOMParser` documents; tree moves between documents (the
effective node document is re-read at each determination); template contents; `createElementNS` with a foreign
namespace and local name `body` / `frameset`; `cloneNode` / `importNode`; the order of `window.on<T>` writes and
insertions. Every change either narrows where a handler lands (null instead of the page Window for a non-active
document) or moves it to the spec's target in the same realm (the active document's own Window), except the residuals
of §1.2: R-A and R-B (EW-16) deliver an attribute to the page Window that the spec delivers nowhere (same realm, the
bound document's own Window); R-B (XW-9) and R-C withhold a delivery the spec makes; R-D moves a handler onto a foreign
element. No case delivers a handler to a Window other than the bound document's (H-T15; the unbind caveat is a caller
contract that every production bind keeps). Security boundaries (CSP, sandbox) are not changed; CSP stays
`#11-csp-inline-handler-enforcement`.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| target determination | `determine_event_handler_target` (NEW), free fn, `session/event_handler_processing/mod.rs` | D-T1 |
| active-document context | `ActiveDocument` (NEW), `Copy` struct of two entities, same module; held by `EventHandlerAttributeConsumer` (constructor `EventHandlerAttributeConsumer::new(policy, active)`, replacing `with_policy`; `Default` derive removed); built per call by the VM accessors | D-T3 |
| node document | `effective_node_document` (NEW), `session/scripting.rs`; N4 later moves it to `EcsDom` | D-T4 |
| step-1 class | `is_body_or_frameset` (existing, session-private, gains the HTML namespace) | D-T5 |
| steps 1–2 | `is_window_delegating_pair` (kept) | D-T2 |
| X-W's remaining population | `is_x_w_pair` (NEW), session-private, removed by S-5 | D-T7 |
| deleted | `resolve_handler_target`, `is_body_element`, `EcsDom::window_entity` | D-T1, D-T3, D-T5 |

No component, store, trait or crate edge is added (`elidex-js` already depends on `elidex-script-session`).

### §4.2 Shape

```rust
// session/event_handler_processing/mod.rs
pub fn determine_event_handler_target(          // (NEW)
    dom: &EcsDom, target: Entity, name: &str, active: ActiveDocument,
) -> Option<Entity> {
    if !is_window_delegating_pair(dom, target, name) {   // steps 1–2
        return Some(target);
    }
    let node_document = effective_node_document(dom, target).unwrap_or(active.document);
    (node_document == active.document).then_some(active.window)   // steps 3–4
}

// in process_handler_content_attributes (D-T7), per handler content attribute:
let deliver = match (is_window_delegating_pair(dom, element, &name), timing) {
    (false, _) => !marked,                                            // OR: once
    (true, ProcessingTiming::Bind) => true,                           // XW, EW
    (true, ProcessingTiming::Insertion) => is_x_w_pair(dom, element, &name), // XW only (NEW)
    (true, ProcessingTiming::HandlerWrite) => false,
};
// then: target = determine_event_handler_target(...); None → skip
```

Call sites of `determine_event_handler_target` after T3 (re-runnable: `/usr/bin/grep -rn
'determine_event_handler_target(' crates` → exactly the five below plus the definition):
1. `process_handler_content_attributes` — `None` → `continue` (no write); signature gains `active` (so do
   `process_bound_document` and `process_before_handler_write`; the latter's VM caller marshals it).
2. `handle_attribute_change` — `None` → return before set / clear.
3. `native_event_handler_get` / 4. `native_event_handler_set` — the normal pair, with `ActiveDocument` from `HostData`.
5. the body pair's shared core (`native_body_weh_get` / `_set` call one helper after `require_body_receiver`).

The VM builds `ActiveDocument` through one host helper (`ActiveDocument` from `host.document_entity_opt()` and
`host.window_entity()`; `None` = unbound → the accessor's existing silent no-op). It reads `HostData` only (marshalling,
Layering mandate); no `EcsDom::*` call is added in `vm/host/`. The marked shortcut (`mod.rs:85`) is unchanged: a marked
`frameset` now has no insertion delivery, so the shortcut's "only `body` / `frameset` have Window-delegating pairs" stays a
sound over-approximation.

### §4.3 ECS-native and Layering checks

The algorithm is a free function over the ECS DOM (a system-like read: tag/namespace components, tree relation,
`AssociatedDocument`), returning an entity the caller writes `EventListeners` on, the existing component. OO → ECS
idiom: the "override the accessor on the subclass prototype" pattern (`HTMLBodyElement.prototype`'s delegation pair)
is replaced, for the target choice, by one query on the receiver entity; the prototype install remains only as the
WebIDL surface. `ActiveDocument` is a per-call value, not a store (D-T3 records why it is not a component yet, PM
decision 132). Layering: steps 1–4 and D-T7's timing live in `elidex-script-session`; `vm/host/event_handler_attrs.rs`
brand-checks, marshals `HostData` into `ActiveDocument`, and executes the result on the listener store.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `determine_event_handler_target` (NEW) | HTML §8.1.8.1 *determine the target of an event handler* steps 1–4; step 3 = "not an active document" in the single-browsing-context model (H-T6); B1 hand-off; the residual proxy facets (`#11-cross-document-adopt-on-insert`, `#11-template-contents-owner-document`) |
| `ActiveDocument` (NEW) | HTML §7.3.2 *active document*; §8.1.3.3.4 *relevant global object*; §7.3.1 (active window = active document's relevant global); `#11-browsing-context-state-ecs-components` (PM decision 132); the unbind caveat (H-T15) |
| `effective_node_document` (NEW) | DOM §4.4 *node document*; the adopt-equivalent proxy (`#11-cross-document-adopt-on-insert`); N4's planned home |
| `is_body_or_frameset` | HTML §8.1.8.1 step 1 ("a body element or a frameset element", HTML namespace); §16.3.2 `frameset` |
| `is_x_w_pair` (NEW) | umbrella §0.5 X-W as narrowed by PM decision 129; removed by slice b's S-5; slot `#11-eh-attrs-creation-time-call-sites` defect (7) |
| `native_event_handler_get` / `_set`, body pair | §8.1.8.1 getter steps 1–3 / setter steps 1–4 |
| `handle_attribute_change`, `process_handler_content_attributes` | §8.1.8.1 attribute change steps 2–3 (the docstring's "Not realised: step 3" at `mod.rs:64-68` is removed; the X-W paragraph `:70-75` is rewritten for D-T7) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The pairs whose target changes are exactly those `is_window_delegating_pair` accepts and HEAD's rule did not, plus the
non-active ones: `EVENT_HANDLER_ATTRS` rows with `HandlerScope::Window` (18; the slice-a ratchet test
`tests_insertion_and_write.rs:318-328` equates them with `webref idl html WindowEventHandlers`) and the six of
`WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (ratchet `:331-342`), on `body` and `frameset`. EW = 6 + 24 = 30
pairs; XW = 18. Unit cells U-T1 and U-T6 derive their tables from those two lists (not a sample).

### §5.2 Existing tests whose expectation changes (red at T3 by design, rewritten in the same commit)

Computed from `tests_event_handler_attrs_window_delegating.rs` at `74e15df1` (re-runnable: `git show
74e15df1:crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs`).

| Test | HEAD expectation | T3 expectation | Why |
|---|---|---|---|
| C17 `c17_domparser_body_weh_idl_write_then_append` (`:39`) | `true,true,true,true,true,true` | `true,false,true,true,true,true` | XW: the setter returns (null target, XW-4); the append still re-raws (XW-6, X-W until S-5). Stays a window pin for its last four values |
| C19b `c19b_body_reflecting_pair_reraw_at_insertion` (`:122`) | `true,true,true,true` | `true,true,false,true` | EW: `setAttribute('onfocus')` on a detached bound `b` writes the Window (EW-2); no re-raw at either append (EW-9, EW-7); the setter returns while `b` is in `dp` (EW-5); `b.onfocus` reads the Window's function; a non-bubbling `focus` at `b` runs nothing. Equals the spec → becomes a guard (renamed: no re-raw) |
| C19c `c19c_frameset_reflecting_pair_reraw_at_insertion` (`:132`) | same as C19b | same as C19b | facets 3 + 4, same steps; spec-equal → guard |
| P3 `p3_frameset_clone_classified_by_tag_and_name` (`:172`) | `true,true` | `true,false` | `onhashchange` on a `frameset` targets the Window (EW-2); the clone's copy is not processed (D-T7); `hashchange` at `f` runs nothing. Spec-equal |
| C8, C18, C19, C20 (i), C20 (ii); session U2, U3 (ii), P1 (i), P1 (ii) | — | unchanged | XW pairs (C8, C18, C19, U2, U3 (ii), P1), the T-a3 rule (C20) |

Kept green and checked: `tests_event_handler_attrs.rs` F-12 (`:235`), F-12b (`:446`), F-14 (`:263`); N1's C8h (outcome
unchanged, mechanism now step 3); `tests_call_listener.rs:120`; the shell E1–E8 (`content_eh_attrs_bind_tests.rs`). The
session tests at `tests_insertion_and_write.rs:60`, `:101`, `:235` read `dom.window_entity()` and switch to the harness's
Window (`test_support::install` / `init_fresh` take `ActiveDocument`).

### §5.3 Touched files

Sizes and paths verified 2026-10-31 against `74e15df1` by the §0.2 commands and `wc -l` on each file.

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` — D-T1/D-T2/D-T3/D-T5/D-T7; deletions;
  harness (`test_support::install` / `init_fresh` take `ActiveDocument`).
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` — constructor, both arms.
- `crates/script/elidex-script-session/src/scripting.rs` — D-T4 extraction.
- `crates/script/elidex-script-session/src/lib.rs` — exports.
- `crates/core/elidex-ecs/src/dom/mod.rs` — delete `EcsDom::window_entity` (930 lines; a deletion, no growth).
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — accessors (745 lines + N1's changes; < 1000).
- `crates/script/elidex-js/src/vm/consumer_dispatcher.rs`, `crates/script/elidex-js/src/vm/vm_api.rs` — marshal
  `ActiveDocument` at bind.
- tests: `session/event_handler_processing/tests_target.rs` (NEW), `tests_bind.rs`, `tests_insertion_and_write.rs`
  (harness); `elidex-js/vm/tests/tests_event_handler_target.rs` (NEW), `tests_event_handler_attrs_window_delegating.rs`
  (§5.2); `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` (cell T-E1; 346 lines).
- docs: the umbrella (§8.4, in this PR).

Not touched (rev 1 touched them): `crates/core/elidex-ecs/src/dom/classify.rs` and
`crates/dom/elidex-dom-api/src/char_data/document_props.rs` (D-T5, PM decision 133).

### §5.4 Named risks

- **R1 — the residuals R-A to R-D** (§1.2): pinned (T-16 to T-19), recorded (§8.1, §8.3), O-6.
- **R2 — merge contact with N1** in `event_handler_attrs.rs` and `scripting.rs` (N1 narrows clause (b)'s arguments to
  `Entity`); T3 is written on N1's landed base (§9 item 1).
- **R3 — `ActiveDocument` staleness.** The consumer's copy is fixed at construction; it is rebuilt at every bind (slice a
  H13), and each production `Vm` binds one `EcsDom`'s one bound document (H-T15). The unbind caveat (H-T15) is the
  relied-on contract; a cross-`EcsDom` rebind would make step 4 return the old World's Window (B1 removes the
  possibility).
- **R4 — `onerror` on `<body>`** now lands on the Window and compiles with the one-argument form, where the spec gives a
  Window `onerror` five parameters: `#11-event-handler-attribute-edge-cases` facet 2 becomes reachable through
  `<body onerror>` (§8.3 item 4).
- **R5 — the merge contact with C1.** C1 changes the node document of template contents; T3's step 3 reads it through
  D-T4. §8.2 states what C1 must do for R-A to close.

## §6. Test plan

Red-first: each **red** cell is committed failing on T3's base before the fix commit. Every VM cell uses
`with_page` / `run` (`elidex-js/vm/test_helpers.rs:166`, `:191`; `dp` is a DOMParser document with an empty body, `el` a
connected element, `fn` counts its calls in `fnRan`).

### §6.1 VM integration (`tests_event_handler_target.rs`, NEW)

| Cell | Kind | Script | HEAD | T3 (= spec unless marked) |
|---|---|---|---|---|
| T-1 | red (facet 1, IDL; XW-4, XW-5) | `dd = new DOMParser().parseFromString('<body></body>','text/html'); dd.body.onhashchange = fn; r.push(window.onhashchange === null, dd.body.onhashchange === null)` | `false,false` (`fn` on the Window) | `true,true` |
| T-2 | red (facet 1, attribute change steps; XW-3) | `dd.body.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.onhashchange === null, window.hit === undefined)` | `false,false` | `true,true` |
| T-3 | red (facet 1, XW re-raw into a non-active document; kills `owner_document`-only; XW-7) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; dp.body.append(b); r.push(window.onhashchange === fn)` | `false` (re-raw) | `true` |
| T-4 | red (facet 4, content attribute; EW-2, EW-4) | `document.body.setAttribute('onload','window.hit = 1'); r.push(typeof window.onload === 'function', document.body.onload === window.onload); window.dispatchEvent(new Event('load')); r.push(window.hit === 1)` | `false,false,false` | `true,true,true` |
| T-5 | red (facet 4, IDL through `HTMLElement.prototype`; EW-3) | `document.body.onfocus = fn; r.push(window.onfocus === fn, document.body.onfocus === fn); window.dispatchEvent(new Event('focus')); r.push(fnRan === 1)` | `false,true,false` | `true,true,true` |
| T-6 | red (facet 3, content attribute; EW-2) | `f = document.createElement('frameset'); f.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.hit === 1)` | `false` | `true` |
| T-6b | guard (facet 3, non-active; EW-5) | `dd = parseFromString('<frameset></frameset>','text/html'); dd.body.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null, dd.body.tagName === 'FRAMESET')` | `true,true` | `true,true` |
| T-7 | red (facets 3 + 4, IDL; EW-3, H-T17) | `f = document.createElement('frameset'); f.onload = fn; r.push(window.onload === fn, f.onload === fn)` | `false,true` | `true,true` |
| T-8a | guard (step 3 note; XW-2) | `b = document.createElement('body'); b.onhashchange = fn; r.push(window.onhashchange === fn)` | `true` | `true` |
| T-8b | red (step 3 note, reflecting; EW-2) | `b = document.createElement('body'); b.setAttribute('onresize','window.hit = 1'); r.push(typeof window.onresize === 'function')` | `false` | `true` |
| T-9 | red (step 1 namespace, WEH; FN-2) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.setAttribute('onhashchange','window.hit = 1'); r.push(window.onhashchange === null); s.dispatchEvent(new Event('hashchange')); r.push(window.hit === 1)` | `false,false` (tag-only: on the Window) | `true,true` |
| T-9b | guard (step 1 namespace, reflecting; FN-1; kills a tag-only class test) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.setAttribute('onfocus','window.hit = 1'); r.push(window.onfocus === null)` | `true` | `true` (tag-only mutant: `false`) |
| T-14 | red + guard (EW-2, EW-7; rev 1's W1 pin, now a spec cell) | `document.body.setAttribute('onresize','window.hit = 1'); r.push(typeof window.onresize === 'function'); window.onresize = fn; document.documentElement.append(document.body); r.push(window.onresize === fn)` | `false,true` | `true,true` (rev 1's re-raw: `true,false`) |
| T-15 | red + guard (EW-5, EW-6, EW-8; rev 1's W2 pin, now a spec cell) | `dd = parseFromString('<body></body>','text/html'); b = dd.body; b.setAttribute('onscroll','window.hit = 1'); r.push(window.onscroll === null); document.documentElement.append(b); r.push(window.onscroll === null, b.onscroll === null)` | `true,true,false` (the element's re-raw compiles) | `true,true,true` (rev 1: `true,false,false`) |
| T-15b | red + guard (EW-10; kills α) | `dd = parseFromString('<body onscroll="window.hit = 1"></body>','text/html'); b = dd.body; document.documentElement.append(b); r.push(window.onscroll === null, b.onscroll === null)` | `true,false` | `true,true` (α: `false,false`) |
| T-16 | **residual pin R-C** (EW-12; O-6) | `b = document.createElement('body'); b.setAttribute('onfocus','window.hit = 1'); window.onfocus = fn; c = b.cloneNode(); el.append(c); r.push(c.onfocus === fn, window.onfocus === fn)` | `false,true` | T3: `true,true`. Spec: `false,false`. Closed by slice b's S-1 (clone path) |
| T-17 | **residual pin R-A** (EW-13; O-6) | `b = document.createElement('body'); tpl = document.createElement('template'); tpl.content.append(b); b.setAttribute('onfocus','window.hit = 1'); r.push(window.onfocus === null)` | `true` | T3: `false`. Spec: `true`. Closed by C1 with the condition of §8.2 |
| T-18 | **residual pin R-B** (XW-9; O-6) | `p = document.createElement('div'); dd = parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); b.setAttribute('onhashchange','window.hit = 1'); r.push(typeof window.onhashchange === 'function')` | `true` | T3: `false`. Spec: `true`. Closed by `#11-cross-document-adopt-on-insert` |
| T-19 | **residual pin R-D** (FN-3; O-6) | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.onhashchange = fn; s.dispatchEvent(new Event('hashchange')); r.push(fnRan === 0, window.onhashchange === null)` | `true,false` | T3: `false,true`. Spec: `true,true`. Closed by the foreign-interface defect's owner (§8.1 item 4) |

The four residual pins assert T3's outcome with a comment naming the closer; O-6 may turn any of them into a red cell
of a widened T3.

### §6.2 Unit (session, `tests_target.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T1 | red (full enum) | for every `EVENT_HANDLER_ATTRS` name and every reflecting name × tag {HTML `body`, HTML `frameset`, `div`, SVG `body`} × node-document state {bound connected, bound detached, foreign connected (a `create_document_node()` document), foreign-owned detached, bound-owned in a foreign tree, unresolvable}: `determine_event_handler_target` | the spec table: not Window-delegating → `Some(target)`; Window-delegating and active (or unresolvable) → `Some(window)`; else `None`. HEAD's rule fails every `frameset`, reflecting, foreign-namespace and non-active WEH row |
| U-T2 | red (bind pass, facet 4; EW-1) | pre-bind page with `<body onload=A>`; `init_fresh` | the Window's `load` entry is raw `A`; the body has no `load` entry. HEAD: the reverse |
| U-T3 | red (C8h's store; EW-5) | foreign document's `body`, `setAttribute('onfocus', 'A')` through the consumer | no `focus` entry on the body and none on the Window. HEAD: raw `A` on the body |
| U-T4 | guard (D-5 keying) | U12's shape: bound document `d2 = create_document_node()` ≠ `document_root()`; a `body` in `d2` with `onhashchange=A`; `active = {d2, w}` | delivered to `w`; a `document_root()`-keyed step 3 returns `None` |
| U-T5 | guard (D-T4 refactor) | `scripting.rs` tests (`:117-`) | unchanged and green |
| U-T6 | red (D-T7 delivery matrix) | for class ∈ {XW, EW, OR} (derived from the two lists and a `div` name) × timing ∈ {Bind, first Insertion of an unmarked element, later Insertion of a marked element, HandlerWrite}: an element carrying one such attribute plus one ordinary attribute, its Window entry taken (`take_uncompiled`) after the first delivery | XW: delivered at Bind and at every Insertion, never at HandlerWrite. EW: delivered at Bind only. OR: once. HEAD: EW delivered at every Insertion onto the element |

### §6.3 End to end (shell)

| Cell | Kind | Markup | HEAD | T3 |
|---|---|---|---|---|
| T-E1 | red (H-T14, EW-1) | `build_main_document("<body onload=\"document.body.dataset.l=1\">…")`, then `probe_body_attr(&pipeline, "data-l")` (the E4 pattern, `content_eh_attrs_bind_tests.rs:165-181`) | `None` | `Some("1")` (H-T20) |

### §6.4 Mutation guards

| Mutant | Killed by |
|---|---|
| step 3 dropped | T-1, T-2, T-3, T-15, U-T1, U-T3 |
| step 3 by `owner_document` alone | T-3, U-T1 (bound-owned in a foreign tree) |
| step 3 keyed on `document_root()` | U-T4 |
| reflecting set dropped from step 2 | T-4, T-5, T-8b, T-14, U-T1, U-T2 |
| `frameset` dropped from step 1 | T-6, T-7, P3, U-T1 |
| class test without namespace | T-9, T-9b, U-T1 |
| an IDL accessor bypassing D-T1 | T-1 (body pair), T-5 / T-7 (normal pair) |
| unresolvable → `None` | U-T1 (unresolvable row) |
| X-W not narrowed (EW re-raw'd at insertion, rev 1's form) | T-14, T-15, U-T6 |
| EW under the ordinary once rule (α) | T-15b, U-T6 (first-insertion row) |
| X-W narrowed too far (XW not re-raw'd) | C8, U2, U-T6 |

## §7. Out of scope and slots

| Item | Why not T3 | Home |
|---|---|---|
| X-W for XW (`body` × WEH re-raw at insertion); the D-8 restriction | slice b's S-5 (PM decision 129 moved only EW's part) | slice b; `#11-eh-attrs-creation-time-call-sites` defect (7) (§8.3 item 2) |
| EW's creation-time attributes on post-bind elements; residual R-C | creation-time processing | slice b S-1 |
| template contents report the bound document; residual R-A | C1 (and insertion adoption, §8.2) | `#11-template-contents-owner-document` |
| insertion adoption; residual R-B; the other proxy facets | DOM §4.2.3 | `#11-cross-document-adopt-on-insert` (§8.3 item 3) |
| foreign elements take HTML interfaces; residual R-D (FN-3) | interface selection, engine-wide (H-T18) | proposed defect text (§8.1 item 4; PM to route) |
| **the `HTMLFrameSetElement` interface**: no `HTMLFrameSetElement.prototype`, so its 18 WindowEventHandlers IDL attributes and `cols` / `rows` read `undefined` | an interface surface; compat (PM decision 130, D-T9) | `#11-htmlframesetelement-interface` (§8.1 item 1) |
| **attribute change steps step 1** (applicability, attribute namespace, MathML; FN-4) | D-T8 | `#11-event-handler-content-attribute-applicability`, closed by slice b (§8.1 item 2) |
| `document.body`'s tag-only class test (H-T12) | PM decision 133: T3's steps 3–4 do not use "the body element" | proposed defect text (§8.1 item 3; PM to route) |
| attribute change steps step 5.1 (CSP) | not modelled | `#11-csp-inline-handler-enforcement` |
| multi-realm step 4; the unbind caveat | B1 | hand-off (§8.2) |
| the ASCII-case-insensitive tag compare (an XML-document `createElementNS(html, 'BODY')` counts as `body`) | engine-wide tag-predicate convention (`is_base_element` precedent) | not changed; noted |

Own deferrals: **0**. The residuals are read through gaps with existing or proposed homes (O-6).

## §8. Edges, hand-offs, records

### §8.1 Slot texts (for the PM; defects only)

1. **`#11-htmlframesetelement-interface`** — registered by PM decision 130 (compat layer). Text: "pre-existing. elidex
   has no `HTMLFrameSetElement` interface (HTML §16.3.2 Frames: `interface HTMLFrameSetElement : HTMLElement` with
   reflected `cols` / `rows`, and `HTMLFrameSetElement includes WindowEventHandlers`). A `frameset` element's wrapper
   takes `HTMLElement.prototype`: the per-tag prototype selection has no `frameset` arm
   (`crates/script/elidex-js/src/vm/host/elements.rs:186-275` at `74e15df1`). So `frameset.cols` / `.rows` and its 18
   WindowEventHandlers IDL attributes read `undefined`. Since T3 (#<PR>, `<squash>`) its handler content attributes and
   the six Window-reflecting IDL attributes it inherits from `HTMLElement.prototype` act on the Window (HTML §8.1.8.1
   *determine the target of an event handler* steps 1–2). **Layer:** compat — `frameset` is an obsolete feature (HTML
   §16.3 Requirements for implementations), so the interface belongs to a compat plugin, not core (CLAUDE.md
   core/compat/deprecated). **Owner:** none; decided when the compat plugin work starts (PM decision 130).
   **Trigger:** the compat plugin's first plan, or a compatibility-survey hit on a frameset page. ⚠ No remedy reviewed."
2. **`#11-event-handler-content-attribute-applicability`** — registered; owner Layout lane (EH-attrs program); closed
   by slice b (D-T8). Text: "pre-existing. The attribute change steps (HTML §8.1.8.1 Event handlers) step 1 returns
   when the attribute's namespace is not null, or its local name is not the name of an event handler content attribute
   on the element. elidex realises neither test: (1) the consumer looks the name up in `EVENT_HANDLER_ATTRS` regardless
   of the element (`event_handler_attr_lookup`,
   `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:237` at `74e15df1`), so a
   WindowEventHandlers name on an HTML element other than `body` / `frameset` (HTML §8.1.8.2: they are content attributes
   of body and frameset elements only; e.g. `div.setAttribute('onhashchange', X)`, then a `hashchange` dispatched at
   the `div` runs `X`), or a Document-only name (`HandlerScope::DocumentOnly`: `onreadystatechange`,
   `onvisibilitychange`) on any HTML element, installs a handler on that element; (2) `MutationEvent::AttributeChange`
   carries no namespace (`crates/core/elidex-ecs/src/dom/mutation_event.rs:306-311`), so
   `setAttributeNS(ns, 'onclick', X)` with a non-null `ns` installs one; (3) which names MathML Core §2.1.3 Global
   Attributes' 'on* event handler attributes' admits on MathML elements is not modelled (every name installs; since T3 a
   MathML `body`'s WindowEventHandlers attribute lands on that element, where HEAD sent it to the Window). SVG elements
   are not a defect: SVG2 §15.8 Event attributes makes every supported event type an event attribute of every SVG
   element. **Owner:** Layout lane (program `EH-attrs-at-creation`); **closes at slice b**, which extends the processing
   function (whose guard step 1 is) to every creation path (S-1) and re-runs the writer census (S-7). **Trigger:** slice
   b's plan. ⚠ No remedy reviewed."
3. **`document.body`'s class test** (pre-existing; PM to route; PM decision 133 keeps it out of T3). Text: "HTML §3.1.7
   DOM tree accessors: *the body element* is the first child of the `html` element that is either a body element or a
   frameset element, both HTML-namespace elements. `first_body_or_frameset_child`
   (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:169-173` at `74e15df1`; the `document.body` getter and the
   VM focus fallback) compares the tag only, so an SVG `body` child of `<html>` placed before the HTML body is returned
   as `document.body`. Since T3 the event-handler step-1 test is namespaced and this one is not: two predicates for one
   class test. **Trigger:** the next PR touching `document.body` or `elidex-dom-api` tree accessors. ⚠ No remedy
   reviewed."
4. **Foreign elements take HTML interfaces** (pre-existing, engine-wide; PM to route; check the ledger for an existing
   row first). Text: "The VM chooses an element wrapper's prototype by tag name alone: `prototype_kind_for` returns
   `PrototypeKind::Element` for every `TagType` entity (`crates/script/elidex-js/src/vm/host_data/mod.rs:1626-1628` at
   `74e15df1`), and the per-tag selection (`crates/script/elidex-js/src/vm/host/elements.rs:186-`) uses
   `tag_matches_ascii_case` (`host_data/mod.rs:1587-1596`), which reads no namespace. So an SVG or MathML element whose
   local name is an HTML tag (`a`, `script`, `style`, `title`, `body`, …) gets that HTML interface's prototype, and every
   other foreign element gets `HTMLElement.prototype`; no `SVGElement` / `MathMLElement` interface is exposed. Reachable
   through T3: an SVG `body` has `HTMLBodyElement.prototype`'s WindowEventHandlers accessors, so `s.onhashchange = fn`
   writes a handler on `s` (since T3; at HEAD it wrote the Window), where the spec makes it an ordinary property, and a
   `hashchange` dispatched at `s` runs `fn` (T3 memo cell T-19). **Trigger:** an SVG / MathML DOM interface PR, or a WPT
   `svg/` subset. ⚠ No remedy reviewed."

### §8.2 Hand-offs

- **B1** (`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 5 / req 7): `ActiveDocument` is the single-browsing-
  context form of steps 3–4. Under a multi-realm World, "active" and "relevant global" become per document; B1's plan
  replaces `ActiveDocument`'s two marshalling points (the consumer's construction in `Vm::bind`, the VM accessor
  helper) with a per-document resolution, and `determine_event_handler_target`'s body; no caller changes. BFCache (a
  document kept non-active with its Window) is the case that makes this a real distinction. The unbind caveat (H-T15)
  becomes structurally impossible under "1 agent = 1 World = 1 Vm". The ECS home of the pair is
  `#11-browsing-context-state-ecs-components` (PM decision 132).
- **N4** (node memo §0.4 D8, §4 "Node document"): `effective_node_document` exists from T3 as a session function with
  both readers; N4's move into `EcsDom` relocates one function and its two calls.
- **Slice b**: S-5 removes X-W for XW and the D-8 restriction; it inverts C8, U2 and C17's last four values. S-1's
  creation-time processing reads `ActiveDocument` at each creation path and closes R-C (EW-12: a clone created in the
  active document delivers its EW attribute to the Window at creation; cell T-16 inverts). Slice b also closes
  `#11-event-handler-content-attribute-applicability` (D-T8).
- **C1** (interactions only; its plan is in drafting): (i) C1 gives template contents an inert owner document. That
  alone does **not** close R-A for a script-created `body` appended into `tpl.content`: insertion does not adopt (H-T8),
  so the node keeps the bound document as its owner and its effective node document (D-T4, detached branch) stays the
  bound document. R-A closes when the derivation reaches the contents' owner for such a node: insertion adoption, or a
  fragment-root rule in `effective_node_document` (the owner of a `DocumentFragment` root). Either is C1's plan's
  decision, and either changes N1's clause (b) too, since the derivation is shared (D-T4). (ii) A clone of template
  contents (TC) inherits its source's owner (umbrella §9 N-2); C1's owner propagation decides TC. (iii) T3's cells T-17
  and C18 are the ones C1 inverts.

### §8.3 Records for existing slots (for the PM, exact texts; all at T3's landing)

1. `#11-event-handler-target-determination` — **close**, with this record: "Closed by T3 (#<PR>, `<squash>`): facets
   (1)–(4) realised by `determine_event_handler_target`, the only caller of steps 1–4 (attribute change steps, IDL getter
   and setter); facet (2) in HEAD's single-browsing-context model (B1 replaces the source of `ActiveDocument`). Residuals
   homed elsewhere: template contents → `#11-template-contents-owner-document`; proxy facets of step 3 →
   `#11-cross-document-adopt-on-insert`; the `frameset` interface → `#11-htmlframesetelement-interface`; a clone created
   in the active document → `#11-eh-attrs-creation-time-call-sites`; foreign-namespace interfaces → <the PM's home for §8.1
   item 4>." In the same edit, the row's first paragraph: "(`…/event_handler_processing/mod.rs:249`) … code as of
   `fd6955e8`" → "(`crates/script/elidex-script-session/src/event_handler_consumer.rs:328` at `fd6955e8`;
   `event_handler_processing/mod.rs:249` since #563)", "in three ways" → "in four ways (facet (4) below)", and
   "**Owner**: the IBP-sandbox program (Layout lane), since its N-series rewrites the handler path." → "**Owner**: the
   EH-attrs-at-creation program (Layout lane), T3 (PM decision 25; sub-bullet below)." (P-T1, P-T2; a landing-time
   record, not applied before T3 lands.)
2. `#11-eh-attrs-creation-time-call-sites` defect (7) — replace "(7) a `<body>` / `<frameset>` handler that delegates to
   the Window (X-W) is re-raw'd at every insertion over any write, and a `<body>` outside an active document delivers it
   to the page Window." by "(7) a `<body>` handler for a WindowEventHandlers name (X-W, narrowed by T3) is re-raw'd onto
   the Window at every insertion of the element into the active document, over any write (since T3 the Window-reflecting
   names on `<body>` and every Window-delegating name on `<frameset>` are not re-raw'd, PM decision 129, and an insertion
   into a non-active document delivers nothing); and a `<body>` / `<frameset>` created in the active document by a clone
   does not deliver its Window-reflecting (for `<body>`) or Window-delegating (for `<frameset>`) attribute to the Window
   at creation, so the clone's getter reads the Window's current value (T3 memo R-C, cell T-16)." In the classification
   sentence, drop "and (7)'s pairs that HEAD targets at the element (both to the spec's `null`)" and the "three" becomes
   "two"; in **Closes**, "which removes the restriction, X-W and processing at insertion" stays (X-W's remaining pairs).
   This is the text the umbrella §8.2 carries from rev 8.4 (§8.4 item 13).
3. `#11-cross-document-adopt-on-insert` — add to its consumer list: "the event-handler target determination's step 3
   (T3, `determine_event_handler_target`, through `effective_node_document`). Its proxy facets after a cross-document
   move: a DOMParser `body` in a detached subtree of the bound document targets null where HEAD and the spec give the
   Window (T3 memo XW-9, cell T-18); a bound `body` moved into a DOMParser document and removed again delivers a
   Window-reflecting or `frameset` attribute to the Window where the spec delivers nothing (EW-16)."
4. `#11-event-handler-attribute-edge-cases` — append to facet (2): "Reachable through `<body onerror>` since T3 (the
   reflecting set now targets the Window)."
5. `#11-template-contents-owner-document` — add: "Consumer since T3: event-handler target determination step 3. A
   Window-reflecting `<body>` / `<frameset>` handler written inside template contents reaches the Window (T3 memo R-A,
   cell T-17); closing it also needs the contents' owner to reach a script-appended node (T3 memo §8.2 C1 (i))."
6. Umbrella K-0's E-G1 (i) and the ledger row's facet (4) bullet: closed by T3 (the record of item 1 covers both).

### §8.4 Umbrella in-place edits carried by this PR (PM decisions 129, 131)

Each edit keeps history in the same style as PM decisions 93 / 105 (umbrella revs 8.1 to 8.3): a changelog
entry *Rev 8.4* names PM decisions 129–131 and this memo, and every edited clause says what it said before T3
("until T3: …") where the old statement was decided text.

1. Changelog: *Rev 8.4* entry.
2. §0.5 X-W, *Status*.
3. §0.5 K-0, the "`<body>` / `<frameset>` Window-delegating handlers (X-W)" paragraph.
4. §0.5 "K-0's wording and X-W", its last sentence.
5. §0.5 K-1 and K-2 italic qualifiers.
6. §0.6 S-5.
7. §1.2 rows T3 and slice b.
8. §1.3 x3 and x6.
9. §1.4 "Removed by slice b" item (1).
10. §1.5 heading and its X-W bullet.
11. §6.1 cells C19b and C19c, their *Spec* notes.
12. §7 G3 and G4.
13. §8.1 first bullet, and two new bullets (`#11-htmlframesetelement-interface`,
    `#11-event-handler-content-attribute-applicability`); §8.2 defect (7), its classification sentence.
14. §10.2 (slice b closes the applicability slot) and §10.3 (T3's scope).

## §9. Definition of done

1. T3's branch is cut from main after N1's implementation lands; every `74e15df1` coordinate in §0.2 is re-measured
   there by its command and the memo's coordinates updated before implementation starts.
2. `/elidex-plan-review` passes on this memo (mandatory, §1.3), and O-6 is decided by the PM / user before it.
3. Red-first: T-1–T-9, T-14, T-15, T-15b, U-T1–U-T3, U-T6, T-E1 committed failing; the fix makes them and §5.2's
   rewritten cells green; the guards (T-6b, T-8a, T-9b, U-T4, U-T5) and the residual pins (T-16 to T-19, as O-6 decides)
   are green at the head.
4. `/usr/bin/grep -rn 'resolve_handler_target\|is_body_element' crates` and `/usr/bin/grep -rn 'fn window_entity'
   crates/core/elidex-ecs` → 0 hits (`HostData::window_entity` and the `ScriptEngine` trait method stay);
   `determine_event_handler_target(` → the definition and the five call sites of §4.2; `is_x_w_pair(` → the definition
   and one call.
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-ecs -p elidex-shell --all-features` green; `mise run ci`
   before push.
6. §8.1 / §8.3 texts handed to the PM in the PR description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-10-eh-attrs-t3-target-determination.md` and the same on the umbrella exit 0.

## §10. Open questions

**Decided since rev 1:**
- **O-1** → PM decision 129 (user, 2026-10-31): T3 also closes W1 and W2 (D-T7; §8.4).
- **O-2** → PM decision 130: `#11-htmlframesetelement-interface` registered, compat, no owner yet (D-T9; §8.1 item 1).
- **O-3** → PM decision 131: the umbrella's edits ride this PR, with history (§8.4).
- **O-4** → PM decision 132: D-T3 confirmed; pointer to `#11-browsing-context-state-ecs-components`.
- **O-5** → PM decision 133: no `document.body` unification. **Confirm the reading** (PM): D-T5 keeps the HTML
  namespace in step 1's own predicate, because T3 uses it (T-9b shows a tag-only test is worse than HEAD) and it is not
  the `document.body` predicate; the other reading (no namespace at all in T3) would give T-9b `false`.

**For the PM / user:**
- **O-6 — the residuals R-A to R-D (§1.2) against PM decision 129's premise.** T3 closes W1 and W2 but is worse than
  HEAD in four residual classes, each read through a gap T3 does not own. Options per class:
  - R-A (template contents): (a) accept until C1, with §8.2's condition on C1's plan; (b) T3 adds a template-contents
    test to step 3 (a node whose tree root is some template's contents fragment, found by a query over
    `TemplateContents`). (b) covers direct contents but not TC, and either changes N1's clause (b) (if put in the shared
    derivation, spec-ward; slice a's window pins C14 / C15 flip) or splits the node-document answer between two readers
    (rejected by D-T4). (c) Reorder C1 before T3: not sufficient alone (§8.2 C1 (i)).
  - R-B (cross-document move, then detached): accept, bounded by `#11-cross-document-adopt-on-insert`, as N1 accepted the
    same proxy for clause (b); the alternative is insertion adoption before T3.
  - R-C (clone in the active document): accept until slice b's S-1; the alternative is T3 taking S-1's clone path for
    Window-delegating pairs, which pre-empts slice b's call-site decision (umbrella O9).
  - R-D (foreign interfaces, MathML): accept; register §8.1 item 4's defect.
  Recommended: accept all four as pinned residuals, with R-A's closure condition written into C1's plan. Every
  alternative either pre-empts a sibling's decision (C1, insertion adoption, slice b's O9) or changes N1's gate.

**Facts needed before plan-review (scout):**
- **Q-7** — at T3's base: does a non-bubbling `Event('focus')` / `Event('hashchange')` dispatched at an element run a
  non-capture handler on the Window (it must not; C19b's and P3's T3 expectations assume it does not)?
- **Q-8** — does `parseFromString('<frameset></frameset>', 'text/html')` produce a `frameset` as `dd.body` (T-6b)?
- **Q-9** — whether the ledger already has a row for foreign elements taking HTML interfaces (§8.1 item 4).

Rev 1's Q-1 to Q-6 are answered (H-T15 to H-T20); Q-1's "unverified: `elidex-wasm-runtime` bind" stays open in H-T15 and
does not affect T3 (no Wasm path reaches the handler consumer at `74e15df1`: `git grep -n
'EventHandlerAttributeConsumer' 74e15df1 -- crates/script/elidex-wasm-runtime` → none).

No browser measurement is needed: every outcome in §6 is spec-defined, except FN-4 (MathML, H-T23), which T3 does not
pin.
