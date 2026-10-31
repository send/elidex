# T3 `EH-attrs-at-creation-T3` — *determine the target of an event handler* follows the spec (per-PR plan-memo)

Nested PR **T3** of the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157` at rev 8, carried to rev 8.3 in place by
#570 and to rev 8.5 in place by this PR, PM decisions 129, 131 and 153; "the umbrella"). It closes slot
`#11-event-handler-target-determination` (all four facets). By PM decision 129 it also takes from slice b's S-5 the part
that ends X-W for the pairs HEAD targeted at the element, and by PM decision 153 it takes from slice b's S-1 the clone
path (D-T10). It is the prerequisite x3 of the umbrella's slice b. Siblings cited by ID: slice a
(`docs/plans/2026-10-eh-attrs-slice-a.md`, landed by #563 `3cd84a04`; "the slice-a plan"), IBP-sandbox N1
(`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 4.5, merged as a plan by #570, implementation in flight; "the N1
memo"), the IBP-sandbox node memo (`docs/plans/2026-09-ibp-sandbox.md`; "the node memo"), **C0** (slot
`#11-cross-document-adopt-on-insert`, insertion adoption; owner Layout lane as C0 of this program, PM decision 143; plan
not yet drafted, branch `layout-dom-c0-insert-adopt-plan` holds no memo on 2026-10-31; its requirements are the C1
memo's §4.6 C0-1 to C0-4) and **C1** (slot `#11-template-contents-owner-document`, branch A, PM decision 123; plan
`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md` rev 1, branch `layout-eh-c1-template-owner-plan` @
`bc52d1b9`, pre-plan-review; "the C1 memo").

**Placement (evidence).** T3 sits under the **EH-attrs umbrella**, not under IBP-sandbox:
- umbrella §1.2 nested-PR table, row **T3** `EH-attrs-at-creation-T3` ("owner: Layout lane, this program; PM decision
  25"); edges x3 (T3 → slice b, semantic precondition) and x6 (slice a → T3, scheduling) in §1.3; §8.1's first bullet;
- the ledger row (`project_open-defer-slots.md`, section "Event handlers — pre-existing slot", the sub-bullet **"Owner
  re-routed 2026-10-25 (PM decision 25)"**): "the owner is now the EH-attrs-at-creation program (Layout lane). T3 is a
  standalone prerequisite PR of slice b, outside the IBP-sandbox DAG". The row's first paragraph still reads "Owner:
  the IBP-sandbox program"; the sub-bullet supersedes it (§8.3 item 1 folds it at T3's landing);
- the N1 memo §7 row "*determine the target of an event handler*": "reassigned by PM decision 25 to the EH-attrs
  program, whose T3 PR carries facets 1, 3 and 4".

**Scheduling (PM decision 153, user, 2026-10-31).** Order: **N1's implementation → C0 → C1 → T3 → slice b**. The option
text the user saw: 「順番を N1 → C0 → C1 → T3 → slice b にして、R-A と R-B は T3 の前に閉じます。R-C（clone）は slice b の S-1 の該当部分を T3 に移して閉じ、R-D（SVG の body）も T3 の中で閉じるかを designer に検討させます。閉じられないものが残ったら、改めて伺います。T3 は大きくなり、入るのも遅くなります。」
Until rev 3 the order was PM decisions 122/123's "N1's implementation → T3 → C1 → slice b" (and PM decision 143's
"N1 → T3 → C0 → C1 → slice b"). T3's implementation branch is cut from main **after C1 lands** (N1 and C0 before it);
this memo's code coordinates are `1ed126b4` (pre-N1, pre-C0, pre-C1) and are re-measured at that base (§9 item 1).
**T3's base** ("the base") is main at that cut: `1ed126b4` plus N1, C0 and C1. K-0's "HEAD" for T3 is the base (§1.2).

**Scope.**
- One function realises *determine the target of an event handler* (WHATWG HTML §8.1.8.1 Event handlers), steps 1–4,
  and every caller the spec gives it calls that function: the attribute change steps (content attributes, at every
  processing timing), the event handler IDL attribute getter and setter (both VM accessor pairs). Facets: (1) step 3,
  a node document that is not an active document → null; (2) step 4, the node document's relevant global object; (3)
  step 1's `frameset`; (4) step 2's Window-reflecting body element event handler set. Step 1's class test is
  HTML-namespaced (D-T5).
- **PM decision 129 (user, 2026-10-31; O-1 of rev 1):** X-W, slice a's rule that a Window-delegating pair is re-raw'd
  at every insertion, ends in T3 for the pairs HEAD targeted at the element: `body` × the six Window-reflecting names,
  and `frameset` × all 24 Window-delegating names. Those pairs are handed to the spec target at the spec's processing
  points (D-T7). The option text the user saw: 「HEAD より悪い状態が main に載らない。T3 は大きくなり、slice b の設計を少し先取りする。plan-review で確かめる」.
  X-W keeps only the pairs HEAD already targeted at the Window (`body` × the 18 WindowEventHandlers names) until slice
  b's S-5. The umbrella's slice-b scope is reduced to match, in place (§8.4).
- **PM decision 153 (R-C):** the **clone path of S-1** — an element created by *clone a node* has its handler content
  attributes processed at its creation, every pair class, shadow-including and template contents included (D-T10).
  The umbrella's S-1 loses the clone path, in place (§8.4), as PM decision 129 did for S-5.
- **PM decision 153 (R-D), this memo's closure:** an HTML-interface choice for `body` that reads the namespace (D-T11),
  so a foreign-namespace `body` no longer carries `HTMLBodyElement.prototype`'s WindowEventHandlers accessors.

**Not in scope.** X-W for `body` × WindowEventHandlers names, and the D-8 restriction (both slice b's S-5); the other
creation paths of S-1 (fragment build, DOMParser build, declarative shadow content, pre-bind template contents; slice
b); insertion adoption (C0) and template contents' owner document (C1), both landed before T3; `document.body`'s class
test (PM decision 133, D-T5); the attribute change steps' step 1 (`#11-event-handler-content-attribute-applicability`,
closed by slice b, D-T8; but see residual R-D2, §2.2); the `HTMLFrameSetElement` interface
(`#11-htmlframesetelement-interface`, compat, PM decision 130); the foreign-element interfaces other than `body`'s
(§8.1 item 4); multi-realm (B1; §8.2 hand-off).

**Changelog.** *Rev 3* (2026-10-31): PM decision 153 (the user's choice for rev 2's O-6) and the PM's confirmation of
PM decision 133's reading (rev 2 O-5). The order becomes N1 → C0 → C1 → T3 → slice b; the premises, D-T4, §1.2 and §2.1
are re-derived on a base where insertion adopts (C0) and template contents are owned by the inert document (C1):
residual **R-A is closed by C1 (on C0)** and **R-B by C0** (§1.2). **R-C is closed in T3**: the clone path of S-1 moves
into T3 (D-T10, new cells T-16 as red, T-20, T-20b, U-T7; the umbrella's S-1 loses it, §8.4). **R-D** splits: R-D1
(a foreign `body`'s HTML interface) is closed in T3 (D-T11, cell T-19 as red); **R-D2** (a non-HTML, non-SVG `body`'s
WindowEventHandlers content attribute, now determined by MathML Core §2.1.4) cannot be closed in T3 without moving a
ratified item and goes to the user (§2.2, O-7). §2.1 has a *Base* column (rev 2: `74e15df1`) and ends with the count
of rows worse than the base: **1** (FN-4), **0** under O-7 (a). Premise errors P-T8 to P-T13. The umbrella goes to
rev 8.5 (§8.4): §1.2 gains rows C0 and C1 and the new order, §1.3 gains edges x7 to x10. Code coordinates move to
`1ed126b4` (#568 moved `ecs/dom/tree/teardown.rs` only). *Rev 2* (2026-10-31, `44927b5f`, rebased as `94ebfce5`): PM
decisions 129–133 and the scout's answers to rev 1's Q-1 to Q-6 folded in. PM 129: D-T7 rewritten (X-W ends for the
element-targeted pairs; class EW, §0.4), W1 / W2 and their window pins T-14 / T-15 become spec-outcome cells, §2.1
enumerates every pair class × timing × document kind against HEAD and the spec, and the umbrella is edited in place
(§8.4). The enumeration found four residual classes in which T3 was worse than HEAD (R-A to R-D), sent to the PM as
O-6. PM 130: `#11-htmlframesetelement-interface` text (§8.1). PM 131: the umbrella edits carry history. PM 132: D-T3
confirmed, with the `#11-browsing-context-state-ecs-components` pointer. PM 133: D-T5 re-scoped; the `document.body`
sibling and cell T-13 are withdrawn. New slot `#11-event-handler-content-attribute-applicability`: closed by slice b
(D-T8), text in §8.1. The scout's answers became premises H-T15 to H-T23; premise errors P-T4 to P-T7. *Rev 1*
(2026-10-31, `e288fe0b`): first draft, pre-plan-review.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every code `file:line` is a **`1ed126b4`** coordinate (origin/main, #568), measured on the worktree with
`git grep -n -E '<pattern>' 1ed126b4 -- <paths>`, `git show 1ed126b4:<path> | sed -n <range>p`, or `/usr/bin/grep -n`
(never the default ugrep). `git diff --stat 74e15df1 1ed126b4 -- crates` touches `ecs/dom/tree/teardown.rs`,
`ecs/dom/tests/destroy.rs` and the strict `tree_builder/mod.rs` only; of those, this memo cites `teardown.rs` (H-T15,
H-T16, re-measured). Spec text is `.claude/tools/webref body` output of 2026-10-31; every `§X.Y = title` pair is
`webref heading` / `dfn` output (§3's footnote). Paths are abbreviated after first use: `session/…` =
`crates/script/elidex-script-session/src/…`, `elidex-js/…` = `crates/script/elidex-js/src/…`, `ecs/…` =
`crates/core/elidex-ecs/src/…`. N1, C0 and C1 will move lines (N1 memo §5.3; C1 memo §5.3; C0 has no memo yet); every
coordinate is re-measured at T3's base (§9 item 1).

**Base outcomes.** Where a *Base* cell of §2.1 or §6 differs from `1ed126b4`'s outcome, the difference is N1's, C0's or
C1's own change, owned by that PR's plan (its K-0 lens), and is marked † with its source. T3's verdict compares T3 with
the base and with the spec only.

Symbols written as `Type` + method (`EcsDom::window_entity`, `HostData::window_entity`, `test_support::install`)
name existing methods or functions inside `impl` / `mod` blocks, which preflight's literal symbol grep does not find
(its soft warnings). `EcsDom::effective_node_document` (planned, N4) is the node memo's name for N4's future home of
D-T4's function. `EcsDom::appropriate_template_contents_owner_document` (NEW: C1) and `EcsDom::is_inert_template_document`
(NEW: C1) are the C1 memo's, present on T3's base.

### §0.2 Premises re-measured

| ID | Premise | Command | Result at `1ed126b4` (and on the base, where stated) |
|---|---|---|---|
| H-T1 | One target function, two content-attribute callers | `git grep -n 'resolve_handler_target(' 1ed126b4 -- crates` | defined `session/event_handler_processing/mod.rs:249`; called by `process_handler_content_attributes` `mod.rs:130` (all three timings: Bind, Insertion, HandlerWrite) and by the consumer's AttributeChange arm `session/event_handler_consumer.rs:420`. Nothing else |
| H-T2 | HEAD's target rule | `sed -n 244,262p session/event_handler_processing/mod.rs` | `scope == HandlerScope::Window && is_body_element(origin)` → `dom.window_entity().unwrap_or(origin)`, else `origin` (`:249-255`); `is_body_element` is a tag-only, ASCII-case-insensitive `body` test (`:258-262`). So: no step 3; step 4 = the World's one `NodeKind::Window` entity (`ecs/dom/mod.rs:450`, a linear scan); no `frameset`; the reflecting six (`HandlerScope::Global` rows) stay on the element. N1, C0 and C1 do not touch it (N1 memo §5.3; C1 memo Scope) |
| H-T3 | The IDL path determines its target separately | `sed -n 323,358p` and `sed -n 418,483p elidex-js/vm/host/event_handler_attrs.rs` | normal pair (`native_event_handler_get` `:323`, `native_event_handler_set` `:344`): target = the receiver, always. Body pair (`native_body_weh_get` `:439`, `native_body_weh_set` `:461`), installed on `HTMLBodyElement.prototype` for `HandlerScope::Window` rows only (`install_body_weh_delegation` `:207-214`, called once, `elidex-js/vm/host/html_passive_protos.rs:66`): target = `ctx.host().window_entity()` (`:448`, `:471`), no step 3; receiver check `require_body_receiver` (`:418-432`) is `tag_matches_ascii_case(entity, "body")`, tag-only. The reflecting six on a `<body>` use `HTMLElement.prototype`'s normal pair (`elidex-js/vm/host/html_element_proto.rs:134`) |
| H-T4 | Steps 1–2 already have one predicate, used by timings only | `sed -n 213,232p session/event_handler_processing/mod.rs` | `is_window_delegating_pair` (`:216`): `is_body_or_frameset` (`:228`, tag-only) ∧ (`HandlerScope::Window` row ∨ `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `:42`). Read by `process_handler_content_attributes` (`:111`), `process_before_handler_write` (`:156`) and `is_only_deliverable_ordinary_name` (`:208`), **not** by `resolve_handler_target`. At HEAD the X-W classifier and the target rule disagree on the reflecting six and on `frameset` |
| H-T5 | The spec's three callers | `.claude/tools/webref body html event-handler-attributes` | the getter steps 1–2 (determine; null → return null), the setter steps 1–2 (determine; null → return), the attribute change steps steps 2–3 (determine; null → return). Step 1 of the attribute change steps: "If namespace is not null, or localName is not the name of an event handler content attribute on element, then return." Step 3's note: a `body` created in an active document but not connected still targets the Window |
| H-T6 | "Active document" in HEAD's model | `sed -n 12,82p session/scripting.rs`; `docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 4; H-T15 | `scripting_disabled_for_platform_object`'s docstring (`:19-30`): "`bound_document` is the active document of the engine's (single) browsing context". With H-T15: in one `EcsDom` the bound document is the only document with a browsing context, and the one Window is its relevant global object (`webref body html navigables`: a navigable's active window "will always equal the navigable's active document's relevant global object") |
| H-T7 | The node-document derivation has one rule at `1ed126b4` | `sed -n 95,114p session/scripting.rs` | clause (b) derives the **effective node document** inline (`:104-110`): composed tree root if it is a Document, else `owner_document`; `None` → fail open. Its docstring: "when it lands `AssociatedDocument` becomes reliable for connected nodes and this rule collapses" (`:51-53`, the C1 memo's C0-4). The node memo plans the rule's promotion to `EcsDom::effective_node_document` by N4 (node memo §0.4 D8). `git grep -n 'find_tree_root_composed' 1ed126b4 -- crates` → `scripting.rs:104` and `elidex-js/vm/host/node_proto.rs:455` (the `isConnected` getter) only. **On the base**, which derivation clause (b) uses is C0's decision (C0-4; Q-10) |
| H-T8 | Insertion adopts on the base | `sed -n 35,54p session/scripting.rs`; `webref body dom mutation-algorithms` (insert step "Adopt node into parent's node document"); the C1 memo §4.6 | at `1ed126b4`, `EcsDom::append_child` relinks without re-homing `AssociatedDocument`. **On the base**, C0 adopts every inserted node into the parent's node document before its `MutationEvent::Insert` (C0-1, C0-3), and the parent's node document of a template contents fragment is the fragment's (C0-2). So a node's node document follows its last insertion; removal keeps it (as in the spec) |
| H-T9 | DOMParser documents are in the page `EcsDom`, built with the dispatcher detached, and reach the consumer after the build | `git show 1ed126b4:crates/dom/elidex-form/src/inert_document.rs \| sed -n 108,200p` | `parse_into_inert_document` takes the dispatcher (`:112`), builds on `create_document_node()` (`:119`), restores it (`:198-199`); later `setAttribute` and inserts on its nodes fire the consumer (N1 memo H-P13) |
| H-T10 | The consumer has no bound document today | `sed -n 96,145p elidex-js/vm/consumer_dispatcher.rs`; `sed -n 215,316p elidex-js/vm/vm_api.rs` | `ConsumerDispatcher::new` builds `EventHandlerAttributeConsumer::with_policy(policy)` (`:110`); the bound `document` reaches the consumer only as `initialize_consumers(dom, document)` (`:141`, called `vm_api.rs:315`). The Window entity is resolved before the dispatcher is built (`vm_api.rs:224-230`) |
| H-T11 | `EcsDom::window_entity`'s production readers | `git grep -n -E '\.window_entity\(\)' 1ed126b4 -- crates` minus `HostData` / `runtime` receivers | one: `resolve_handler_target` (`mod.rs:251`). Tests: `session/event_handler_processing/tests_insertion_and_write.rs:60`, `:101`, `:235` |
| H-T12 | The body-or-frameset class test is written three times | `git grep -n 'eq_ignore_ascii_case("body")' 1ed126b4 -- crates` | `mod.rs:231` (`is_body_or_frameset`), `mod.rs:261` (`is_body_element`), `crates/dom/elidex-dom-api/src/char_data/document_props.rs:173` (`first_body_or_frameset_child`, the `document.body` getter). None checks the HTML namespace; the precedent `EcsDom::is_base_element` (`ecs/dom/classify.rs:39-46`) does, through `EcsDom::is_html_namespace` (`classify.rs:136`) |
| H-T13 | Slice a's window pins that name T3 | `git grep -n 'T3' 1ed126b4 -- crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs_window_delegating.rs` | C17 (`:39`), C18 (`:64`), C19b (`:122`), C19c (`:132`), P3 (`:172`) say "reached after T3 and slice b" or "T3 facet 3" |
| H-T14 | `<body onload>` never runs at HEAD | `git show 1ed126b4:crates/shell/elidex-shell/src/pipeline.rs \| sed -n 462,474p` | the document `load` event is dispatched at the Window entity (`:470`), non-bubbling; `<body onload>` lands on the body element (H-T2), so it never runs. Same for an initial `<body onerror\|onfocus\|onblur\|onresize\|onscroll>` against Window-targeted events |
| H-T15 | (rev 1 Q-1) One bound document and one Window per `EcsDom`; other Documents have no Window | `git grep -n 'create_window_root()' 1ed126b4 -- crates`; `git grep -n 'create_document_node()' 1ed126b4 -- crates`; `git show 1ed126b4:crates/script/elidex-js/src/vm/host_data/mod.rs \| sed -n 1690,1716p` | the Window root is created at one site, `vm_api.rs:227`, only when `HostData::window_entity` is `None`; `HostData::set_window_entity` asserts on a second set (`host_data/mod.rs:1710-1715`). Every iframe, navigation and worker gets a new `EcsDom` and `Vm` (scout, 2026-10-31: `elidex-shell/src/pipeline.rs:226`, `:735`; `content/iframe/load.rs:465-484`; `content/navigation.rs:276-290`; `app/navigation.rs:323`; workers use `create_worker_global_scope_root`). Production `create_document_node()` callers: `inert_document.rs:119` (DOMParser: a **second, unbound, windowless Document in the page `EcsDom`**, persistent) and `ecs/dom/tree/teardown.rs:301` (the fragment parser's throwaway document, destroyed at `finish_detached_fragment`, `:330-342`). On the base C1 adds a third kind, the inert template document (C1 memo D-C1), also windowless. **Caveat (a stated defect, not T3's):** `HostData::window_entity` is initialised `None` once (`host_data/mod.rs:856`) and is **not cleared on unbind** (`git grep -n -E 'window_entity\s*=\s*None' 1ed126b4 -- crates/script/elidex-js/src` → none), so a `Vm` re-bound to a different `EcsDom` would keep a Window entity of the old World. "One World per `Vm`" is a caller contract (doc comments), not enforced. No production path rebinds across `EcsDom`s (above). Unverified: whether `elidex-wasm-runtime`'s bind (`host/state.rs:50`, `:125`) shares the JS `EcsDom` |
| H-T16 | (rev 1 Q-2) Which nodes have no resolvable node document | `git show 1ed126b4:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 328,362p` and `sed -n 806,850p`; `git grep -n 'create_element_with_owner' 1ed126b4 -- crates/dom/elidex-dom-api/src/document.rs`; `git show 1ed126b4:crates/core/elidex-ecs/src/dom/tree/teardown.rs \| sed -n 310,343p` | `EcsDom::create_element` passes no owner (`:335-337`); `owner_document` falls back to the tree root (`:839-`), so a node with no `AssociatedDocument` is unresolvable only while it is detached. The VM's `createElement` passes the receiver document (`elidex-dom-api/src/document.rs:235`), so `dp.createElement('body')` is owned by `dp`. Fragment-parse and DOMParser nodes are re-homed into the context's owner document by `finish_detached_fragment` (`teardown.rs:336-338`, `EcsDom::adopt_subtree`); the adopt is skipped only for a documentless context, which then yields orphans. So an unresolvable `body` / `frameset` comes only from an owner-less `create_element` caller or a documentless fragment context, both in the bound realm. C0 adopts at insertion only, so it does not resolve a never-inserted orphan |
| H-T17 | (rev 1 Q-3) A `frameset` wrapper's prototype | `git show 1ed126b4:crates/script/elidex-js/src/vm/host/elements.rs \| sed -n 108,125p` and `sed -n 186,275p` | no `frameset` arm in `tag_specific_html_prototype` / `tag_specific_t2b_prototype`, so `HTMLElement.prototype` (`:121-123`). T-7's `f.onload` reaches `HTMLElement.prototype`'s normal pair; a WindowEventHandlers name on a `frameset` has no accessor (an ordinary property; `#11-htmlframesetelement-interface`) |
| H-T18 | (rev 1 Q-4) Foreign elements take HTML prototypes | `git show 1ed126b4:crates/script/elidex-js/src/vm/host_data/mod.rs \| sed -n 1580,1628p` | `prototype_kind_for` returns `PrototypeKind::Element` for every `TagType` entity (`:1626-1628`); `tag_matches_ascii_case` (`:1587-1596`) reads no namespace. Foreign elements carry `TagType` (`create_element_ns` → `create_element_with_owner`, `ecs/dom/mod.rs:374-381`, `:352-357`). So an SVG or MathML `body` gets `HTMLBodyElement.prototype` with the body pair (the `body` arm, `elements.rs:264-266`), engine-wide for every tag-specific prototype. `host_data/mod.rs` is 2024 lines (`wc -l`) |
| H-T19 | (rev 1 Q-5) The AttributeChange event carries no namespace | `git show 1ed126b4:crates/core/elidex-ecs/src/dom/mutation_event.rs \| sed -n 306,311p` | `AttributeChange { node, name, old_value, new_value }`; emitters `ecs/dom/attribute.rs:135`, `:321` (scout). The attribute change steps' step 1 "namespace is not null" cannot be tested at the consumer (D-T8) |
| H-T20 | (rev 1 Q-6) The shell harness runs the `load` dispatch | `git show 1ed126b4:crates/shell/elidex-shell/src/content_test_support.rs \| sed -n 377,399p`; `pipeline.rs:374`, `:432-478` | `build_main_document` → `build_pipeline_from_loaded` → `run_scripts_and_finalize` (`pipeline.rs:374`) → `dispatch_lifecycle_events` (`:432-478`), which dispatches `load` at the Window (H-T14). T-E1 observes it (scout, confidence high-medium; T-E1's red run is the check) |
| H-T21 | Template contents are owned by the inert document on the base | `git show 1ed126b4:crates/core/elidex-ecs/src/dom/mod.rs \| sed -n 540,591p`; the C1 memo D-C2 to D-C6, §4.5 item 1; `webref body html the-template-element` | at `1ed126b4` (the C2 approximation, `:552-557`) the contents report the template's own node document. **On the base**, the contents' node document is `appropriate_template_contents_owner_document(D)` = an inert document I (C1 D-C2, D-C3), parser-created contents carry it (D-C4), a clone of contents is in I (D-C6), and a node appended into `tpl.content` is adopted into I (C0-2). I's "browsing context is null" (spec), and I is never the bound document, so step 3 returns null for every node in template contents or in a clone of them |
| H-T22 | SVG elements accept every event attribute | `.claude/tools/webref body SVG2 EventAttributes` | SVG2 §15.8 Event attributes: "For every event type that the user agent supports, SVG supports that as an event attribute … The event attributes are available on all SVG elements". So `onhashchange` on an SVG `body` is an event attribute of that element (target: the element, step 1) |
| H-T23 | (rev 2: "not determined") MathML elements' event attributes and interface | `.claude/tools/webref body mathml-core attributes-common-to-html-and-mathml-elements`; `webref body mathml-core dom-and-javascript`; `webref body mathml-core mathml-elements-and-attributes`; `webref body html event-handlers-on-elements,-document-objects,-and-window-objects` | MathML Core §2.1: a `body` in the MathML namespace is an *Unknown MathML element*. §2.2.3 DOM and JavaScript: every MathML element implements `MathMLElement` (`MathMLElement : Element`, `includes GlobalEventHandlers`, `includes HTMLOrSVGOrMathMLElement`), so no WindowEventHandlers IDL attribute. §2.1.4 Attributes common to HTML and MathML elements: "All event handler content attributes noted by HTML as being supported by all HTMLElements are supported by all MathML elements as well, as defined in the MathMLElement IDL." HTML §8.1.8.2 supports the WindowEventHandlers content attributes on "body and frameset elements" only, not on all HTML elements. So a WindowEventHandlers name is **not** an event handler content attribute of any MathML element (the attribute change steps' step 1 returns), and a MathML element named `body` / `frameset` never gets HTML element semantics. The same holds for an element in any namespace other than HTML, SVG and MathML (DOM §4.5 *element interface*: `Element` unless a spec says otherwise; no event handlers) |
| H-T24 | The clone path: one production entry, a VM seam with two precedents | `git grep -n -E 'clone_node_with_shadow_honor\(\|"cloneNode"' 1ed126b4 -- crates`; `sed -n 167,235p elidex-js/vm/host/node_methods_extras.rs` | the only production element-cloning operation is `Node.prototype.cloneNode`: `native_node_clone_node` (`node_methods_extras.rs:201`) → `invoke_dom_api(ctx, "cloneNode", …)` (`:212`) → the dom-api `CloneNode` handler (`crates/dom/elidex-dom-api/src/node_methods/clone.rs:154`) → `clone_node_with_shadow_honor` (`clone.rs:227`; template contents and clonable shadow trees replicated). After the call the shim runs two clone-time seams before returning to script: custom-element upgrade reactions are **enqueued** (`:221`, drained at checkpoints) and `elidex_form::apply_clone_form_state` (`:230`), engine-independent, called with `host.dom()`. No script runs between the handler's return and the shim's return (Q-11 confirms) |
| H-T25 | The VM already marshals the policy into a session processing entry | `sed -n 665,674p elidex-js/vm/host/event_handler_attrs.rs`; `sed -n 50,57p session/event_handler_processing/mod.rs` | `marshal_before_handler_write` reads `ctx.vm.spec_level_policy` and calls `elidex_script_session::process_before_handler_write(host.dom(), element, name, policy)` (`:670-672`). `ProcessingTiming` has `Bind`, `Insertion`, `HandlerWrite` (`mod.rs:50-57`). The uncapped walker `EcsDom::for_each_uncapped_shadow_inclusive` (`ecs/dom/tree/walkers.rs:99`) takes a `TemplateContentsWalk` (`:19`); the bind pass passes `Exclude` (`mod.rs:178`) |
| H-T26 | No other element-cloning path exists | `git grep -n -i 'importNode' 1ed126b4 -- crates` → none; `sed -n 248,262p` and `sed -n 590,606p crates/dom/elidex-dom-api/src/range/mutation.rs` | `importNode` is not implemented. `Range.cloneContents` is a stub (`clone_contents` returns `None`, slot `#11-range-clone-and-surround-contents`); `extractContents` clones boundary Text only, its partially-contained element clone is deferred (B1.2d-ii plan §11) |

### §0.3 Premise errors (reported)

- **P-T1.** The ledger row's coordinate "`resolve_handler_target` (`…/event_handler_processing/mod.rs:249`) … code as of
  `fd6955e8`" mixes frames: at `fd6955e8` the function is `session/event_handler_consumer.rs:328`
  (`git show fd6955e8:crates/script/elidex-script-session/src/event_handler_processing/mod.rs` fails: the file was added by
  #563); `mod.rs:249` is the `3cd84a04` / `74e15df1` / `1ed126b4` position. §8.3 item 1 carries the correction (a
  landing-time record).
- **P-T2.** The slot records **four** facets, not three (facet (4), the reflecting set, was added 2026-10-25), and its
  owner is the EH-attrs program (re-routed sub-bullet), although its first paragraph still says IBP-sandbox.
- **P-T3.** The Reachability example (`onload` → `onhashchange`) is already corrected in the ledger as read on
  2026-10-31 (line 807 reads `setAttribute('onhashchange', …)`).
- **P-T4.** The scout's row "DOMParser / createHTMLDocument: no second document in production" is **wrong**: DOMParser
  (`elidex-js/vm/host/dom_parser.rs:292-297` → `parse_into_inert_document`) creates a second, unbound, windowless
  Document in the page `EcsDom` (`inert_document.rs:119`). It does not change Q-1's answer about bound documents and
  Windows (H-T15). `createHTMLDocument` does not exist (`git grep -n -i 'createHTMLDocument' 1ed126b4 -- crates` → none).
- **P-T5.** Rev 1 §1 said every outcome change at T3 is toward the spec except the O-1 window, and §2 A3×A8 said a
  template-contents `body` keeps HEAD's outcome. Both were wrong on rev 2's base (`74e15df1`, no C1); rev 3's base
  removes the case (R-A closed by C1, §1.2).
- **P-T6.** Rev 1 R2 said each proxy facet of step 3 gives HEAD's target or the spec's. Wrong on rev 2's base (no
  insertion adoption); rev 3's base removes the case (R-B closed by C0).
- **P-T7.** Rev 1 §3.1 said no case turns a null into a delivery except W2. Residuals R-A and R-B did; both are closed
  on rev 3's base.
- **P-T8 (rev 2).** Rev 2 §1.2 R-C and §3.1 named "`importNode` into the bound document" as a creation path; `importNode`
  does not exist in elidex (H-T26). The clone path is `cloneNode` alone.
- **P-T9 (rev 2).** Rev 2 §5.2 listed C18 as unchanged. On rev 3's base its first assertion changes: the node appended
  into `tpl.content` is in the inert document (C0-2, C1), so T3's step 3 makes `setAttribute` and the setter return, and
  `window.onhashchange` is `null` after the setter, not `fn` (§5.2). Rev 2 §8.2 C1 (iii) ("T3's cells T-17 and C18 are
  the ones C1 inverts") and the C1 memo's edge c2 ("T3 → C1 (T18 assumes T3's step 3)") and cell T18 ("rewrite (umbrella
  C18; needs T3)") assume T3 before C1. Under PM decision 153's order C1 alone keeps C18 (the base's target rule ignores
  the document), and T3 rewrites C18's first step (a downstream for the C1 memo's owner, §8.2).
- **P-T10 (rev 2).** H-T23 said whether WindowEventHandlers names are MathML event attributes is "not determined from
  webref". MathML Core §2.1.4 determines it (H-T23): they are not. Rev 2's FN-4 verdict ("✗ if MathML excludes the
  name") is therefore definite (§2.1 FN-4), and its population is every `body` outside the HTML and SVG namespaces.
- **P-T11 (rev 2).** Rev 2's FN rows omitted the insertion timing. At HEAD `is_window_delegating_pair` is tag-only, so a
  foreign `body` / `frameset` pair is re-raw'd at every insertion (X-W); at T3 it is not Window-delegating, so it follows
  the ordinary once rule (row FN-6).
- **P-T12 (rev 2).** Rev 2 §6.4 said cell T-3 kills "step 3 by `owner_document` alone". On rev 3's base the insertion in
  T-3 adopts `b` into `dp` (C0), so `owner_document` and the composed-root derivation agree there; whether they can
  differ on the base at all is C0's decision (Q-10). The mutant and its claim are withdrawn (D-T4).
- **P-T13 (brief).** The brief cites "`elements.rs` ~264 at `74e15df1`" for the `body` arm: it is `elements.rs:264`
  (`tag_specific_t2b_prototype`, `:255`) at both `74e15df1` and `1ed126b4`; `host_data/mod.rs:1587-1596` is unchanged.

### §0.4 Decisions (this PR)

Pair classes used below (all on HTML-namespace elements unless stated): **XW** = `body` × the 18 WindowEventHandlers
names, the pairs HEAD already targets at the Window; **EW** = `body` × the 6 Window-reflecting names and `frameset` × all
24 Window-delegating names, the pairs HEAD targets at the element (PM decision 129's population); **FN** = a
foreign-namespace `body` / `frameset` with any of those 24 names; **OR** = every other pair.

- **D-T1 — one function, every spec caller.** `determine_event_handler_target(dom, target, name, active) ->
  Option<Entity>` (NEW), in `session/event_handler_processing/mod.rs`, `pub`, is the only realisation of the algorithm.
  `None` is the spec's null. Callers: the attribute change steps at `process_handler_content_attributes` (every timing,
  the clone path's Creation included, D-T10) and the AttributeChange arm; the VM getter and setter, both pairs.
  `resolve_handler_target` and `is_body_element` are deleted. The body pair keeps its own receiver brand check
  (`require_body_receiver`; WebIDL `HTMLBodyElement`, namespaced by D-T11) but no longer chooses a target. The VmObject
  handler backend (`native_vm_event_handler_get` / `_set`) does not call it: its receivers are never `body` / `frameset`
  elements, so step 1 returns the receiver by type (stated in its docstring).
- **D-T2 — steps 1–2 are `is_window_delegating_pair`.** The function calls slice a's classifier for steps 1–2 and returns
  `Some(target)` when it is false. The X-W classifier and the target rule then agree by construction (H-T4's
  disagreement is the defect of facets (3) and (4)). Step 2's name test is by name: the `scope` parameter leaves target
  determination (`HandlerScope` stays the IDL-install key).
- **D-T3 — steps 3–4 over a marshalled `ActiveDocument` (NEW); PM decision 132.** `pub struct ActiveDocument { document:
  Entity, window: Entity }`: the active document of the engine's single browsing context and its relevant global object
  (H-T6, H-T15). Step 3: the target's node document (D-T4) equals `active.document`, else `None`. Step 4:
  `active.window`. The consumer receives it at construction from `Vm::bind` (`document` and the resolved Window, H-T10);
  the VM accessors and the clone seam build it from `HostData` (`document_entity_opt()`, `window_entity()`).
  `EcsDom::window_entity` loses its last production reader and is deleted (H-T11; dead-code rule). **PM decision 132:**
  marshalling is accepted; the active document and its Window are browsing-context state, not a per-element fact, so the
  side-store→component rule does not apply, and their move into ECS components is
  `#11-browsing-context-state-ecs-components`, decided with B1. *Rejected:* (A) a component relating the Document and
  its Window, written at bind (it duplicates `HostData`'s `document_entity` / `window_entity` pair, which B1 §5 req 5
  migrates as one cluster); (D) `document_root()` keying (slice a's D-5 / cell U12; the node memo's "No reader keys on
  `document_root()`"). *Relied-on caveat* (H-T15): `ActiveDocument.window` comes from `HostData::window_entity`, which
  unbind does not clear; T3 is correct only under the caller contract "one `EcsDom` per `Vm`", which every production
  bind path keeps. B1 makes it structural (§8.2).
- **D-T4 — one node-document derivation, the base's (rev 3: re-stated for C0).** Step 3 reads the target's node document
  through **the derivation N1's compile-gate clause (b) uses on T3's base**; T3 adds no second one. C0's plan decides that
  derivation (C1 memo §4.6 C0-4: the composed-root proxy "collapses" once insertion adopts). Two outcomes, re-measured at
  T3's base (§9 item 1; Q-10):
  (i) clause (b) still derives it inline (`scripting.rs:104-110`): T3 extracts it into `effective_node_document(dom,
  node) -> Option<Entity>` (NEW), a `pub` free function in `session/scripting.rs`, called by clause (b) (code-identical)
  and by D-T1's step 3 (rev 2's D-T4); N4 later moves it into `EcsDom::effective_node_document` as the node memo plans;
  (ii) C0 has collapsed or named it (`owner_document`, or a named function): T3 calls that function, and adds nothing.
  In both, unresolvable (`None`) → treated as `active.document`, the direction of clause (b)'s fail-open on the same
  ground ("such nodes belong to the bound document's realm by construction", `scripting.rs:55-58`); H-T16 measures that
  ground for `body` / `frameset` (cells XW-13, EW-18). Where C0 adopts at every insertion (C0-1) and C1 stamps template
  contents (H-T21), both derivations give the spec's node document for every node that reached its place by insertion,
  by parsing or by cloning, so the cells of §6 are chosen not to distinguish (i) from (ii); Q-10 and Q-13 confirm this
  on C0's and C1's landed code. *Rejected:* landing `EcsDom::effective_node_document` here (it edits N4's approved row);
  a derivation private to step 3 (two answers to one question, A3×A7).
- **D-T5 — step 1's class test is HTML-namespaced; `document.body` is not touched (PM decision 133; its reading
  confirmed by the PM, rev 2 O-5).** The session's `is_body_or_frameset` (`mod.rs:228`) gains `EcsDom::is_html_namespace`
  (the `is_base_element` precedent) and stays session-private; `is_body_element` is deleted (D-T1); no `EcsDom` symbol is
  added. PM decision 133 unifies with `document.body`'s predicate only if T3's step 3 or 4 uses it; neither does (step 3
  reads the node document, step 4 the global object; "the body element" of HTML §3.1.7 DOM tree accessors is not read).
  So `first_body_or_frameset_child` (`document_props.rs:169-173`) keeps its tag-only test, rev 1's cell T-13 stays
  withdrawn, and the `document.body` defect is recorded for the PM (§8.1 item 3). The namespace stays in step 1 because
  T3 itself needs it: under a tag-only test an SVG `body`'s `onfocus` would move from the element (HEAD's and the spec's
  target) to the Window (cell T-9b). The **confirmed reading** (PM, 2026-10-31): T3 keeps its own HTML-namespace "the
  body element" predicate; no tag-only test; `document.body` is not touched. The VM's interface choice for `body` is a
  separate test (D-T11).
- **D-T6 — order inside the IDL setter.** receiver brand check → determine (D-T1) → `None`: return → slice a's
  `marshal_before_handler_write` (D-6) → activate. Determination precedes D-6 per the setter's steps 1–2; the reorder
  changes nothing observable, because `None` occurs only for Window-delegating pairs, for which
  `process_before_handler_write` returns at once (`mod.rs:156`). The getter: brand check → determine → `None`: null →
  read (N1's reconcile, unchanged).
- **D-T7 — X-W ends for EW; XW keeps it until S-5 (PM decision 129); the clone path processes every class (PM decision
  153).** `process_handler_content_attributes` delivers a pair by class and timing:

  | class | Bind (T-a1) | Creation, clone path (T-c, D-T10) | Insertion (T-a2) | HandlerWrite (T-a3) | attribute change steps (AttributeChange arm), IDL setter |
  |---|---|---|---|---|---|
  | XW | delivered (X-W) | delivered | delivered at every insertion (X-W, until S-5) | not processed | T3's target |
  | EW | delivered, once per document (the bind pass's own once rule, D-5) | delivered | **not delivered** | not processed | T3's target |
  | OR | once (K-1) | delivered, and marked (K-1) | once, if unmarked (K-1) | D-8 | element |

  Every delivery goes to `determine_event_handler_target`'s result (`None` → nothing). The X-W population is named by
  `is_x_w_pair(dom, element, name)` (NEW, session-private): an HTML `body` and a `HandlerScope::Window` name. Its
  docstring names S-5 as its remover. EW's creation-time attributes on an element created after bind by a **DOMParser
  build or a fragment parse** are therefore not processed until slice b's S-1; that is the spec's outcome wherever the
  element's node document at creation is not active: the DOMParser build (cells EW-10, T-15b) and the fragment
  parser's throwaway document (a fragment parse with an `html` context creates a `body` / `frameset`, e.g.
  `document.documentElement.innerHTML = '<body onresize=…>'`; the elements are created in the throwaway document and
  only then adopted and inserted, so the spec delivers nothing, EW-10's case). EW thus satisfies K-1 to K-3 unconditionally
  from T3: nothing derived from an EW attribute is written after the bind pass except at a clone's creation, by the
  attribute change steps and by the IDL setter, which are the spec's writers.
  *Rejected:* (α) EW joining the ordinary pairs' once rule (processed at the first insertion if unmarked). The first
  insertion reads the target at the insertion, not at the element's creation, so a DOMParser or fragment-parsed `body`'s
  attribute reaches the page Window at its first insertion into the active document (W2, cell T-15b's mutant), and an
  unmarked element's attribute overwrites a `window.on<T>` written before that insertion (W1 on the D-8-failing path,
  U-T6). (β) A timing-dependent target (two target rules). Rev 1's form (X-W kept for EW, onto T3's target) is withdrawn
  by PM decision 129. Delivery at a clone's **creation** is not α: it is the spec's own processing point (D-T10), so an
  overwrite of `window.on<T>` there is the spec's outcome, not W1.
- **D-T8 — `#11-event-handler-content-attribute-applicability` is closed by slice b, not T3 (ratified; see R-D2).**
  (i) T3 does not need it to be no worse than the base for HTML and SVG elements: on every HTML element other than
  `body` / `frameset` T3's target is the base's (class OR, cell OR-1), and on SVG elements every event type is an event
  attribute (H-T22), so T3's namespaced step 1 sends an SVG `body`'s WindowEventHandlers attribute to the element, which
  is the spec's target (FN-2). It does need it for a `body` outside the HTML and SVG namespaces (H-T23, row FN-4):
  residual **R-D2** (§2.2, O-7). (ii) Step 1 of the attribute change steps is the guard of D-2's processing function,
  which slice b extends to every remaining creation path (S-1). (iii) Its attribute-namespace facet needs
  `MutationEvent::AttributeChange` to carry a namespace (H-T19) and the silent writers' namespaces, which slice b's S-7
  census re-runs.
- **D-T9 — `frameset`'s interface is compat (PM decision 130).** `#11-htmlframesetelement-interface` is registered in
  the compat layer: `frameset` is an obsolete feature (HTML §16.3.2 Frames, under §16.3 Requirements for
  implementations), so its interface belongs to a compat plugin, not core (CLAUDE.md core/compat/deprecated). Owner: none
  until the compat plugin work starts. T3 changes only target determination for a `frameset`; its interface is not
  added.
- **D-T10 — the clone path of S-1 (NEW in rev 3; PM decision 153).** WHATWG DOM §4.4 *clone a single node* step 2.5
  appends each attribute to the copy (§4.9 *append an attribute*), whose step 4 runs *handle attribute changes*, whose
  step 3 runs the attribute change steps: a clone's handler content attributes are processed **at its creation**, with
  the copy's node document. T3 realises that for the one production clone operation (H-T24, H-T26):
  - **Entry point.** `process_created_subtree(dom, root, policy, active)` (NEW), `pub`, in
    `session/event_handler_processing/mod.rs`: for every element of `root`'s shadow-including subtree with template
    contents included (`for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)`, uncapped, H-T25), it
    runs `process_handler_content_attributes` with `ProcessingTiming::Creation` (NEW variant) and marks the element
    (D-1). D-T7's Creation column applies: every class delivered to T3's target. A fresh clone is unmarked (the
    clone-policy row, D-1), so "once" holds.
  - **Call site.** The VM's `native_node_clone_node` (`node_methods_extras.rs:201`), after the dom-api call and beside
    its two clone-time seams (H-T24), marshals `ctx.vm.spec_level_policy` and `ActiveDocument` (D-T3) and calls the entry
    point on the clone root, before returning to script. This is the D-6 precedent's shape (H-T25): the algorithm is in
    `elidex-script-session`, the VM marshals. *Rejected:* a call inside the dom-api `CloneNode` handler (it holds neither
    the policy nor the active document, umbrella §9 N-6, and `elidex-dom-api` does not depend on
    `elidex-script-session`); a new `MutationEvent` variant for creation (umbrella §9 N-9, and the fragment and DOMParser
    builds detach the dispatcher, N-1, so it would not serve slice b's paths).
  - **What slice b inherits.** The entry point is the one slice b's other creation paths call (S-6 extended, umbrella
    rev 8.5). The umbrella's O9 (call-site shape) is decided for the clone path only; slice b decides it for its paths.
  - **Population closed.** Rows EW-12 (R-C), XW-11, EW-11, EW-19, OR-2 (umbrella §1.5 G5 / K3, and K2's EW half; §8.2
    defect (2), the clone half of defect (1), (3)'s EW half, (7)'s clone clause). XW's re-raw at the clone's insertion
    stays (X-W, S-5).
  - **Fallback.** Slice a's insertion processing still runs for an unmarked element, so a clone path that a future
    operation adds without calling the entry point is processed at its first insertion (HEAD-equal; umbrella §5.2 K1).
    S-4's detector is slice b's, for its end state; T3 records the entry point in the Range slot (§8.3 item 7).
- **D-T11 — `body`'s HTML interface is chosen for an HTML `body` only (NEW in rev 3; R-D1).** The VM gives a
  `TagType` entity a tag-specific HTML prototype by local name alone (H-T18). T3 makes the `body` arm of
  `tag_specific_t2b_prototype` (`elements.rs:264`) and `require_body_receiver` (`event_handler_attrs.rs:427`) also require
  the HTML namespace, read through the existing `EcsDom::is_html_namespace` (marshalling read; no `HostData` method is
  added, so `host_data/mod.rs`, 2024 lines, is not touched; Q-14). A foreign `body` then takes `HTMLElement.prototype`,
  the fallback every other foreign element already takes (H-T18). Every member that loses is one the spec does not give
  that element: `HTMLBodyElement`'s own attributes and its 18 WindowEventHandlers accessors are on neither `SVGElement`
  (SVG2) nor `MathMLElement` (H-T23), and `HTMLElement.prototype` keeps the GlobalEventHandlers accessors both foreign
  interfaces include. So the change is toward the spec at every observation (rows FN-3, FN-5; cell T-19).
  *Why not every arm (the general fix):* for a tag that is also an SVG or MathML element name (`a`, `script`, `style`,
  `title`, …) the fallback `HTMLElement.prototype` would remove members the foreign interface has (e.g. `type` on
  `SVGScriptElement`), which the HTML interface supplies today: worse than the base. The general fix needs the SVG and
  MathML interface families (the master roadmap's `M4-13:#svg` row, "SVG 全 element family + SVGSVGElement", absent);
  the ledger has no slot for foreign-element interface selection (checked 2026-10-31, rev 2 Q-9). §8.1 item 4 is its
  defect text, with the `body` arm recorded as already namespaced. *Not chosen:* every arm without a foreign namesake
  (`div`, `iframe`, …), which is toward the spec too but is not reached by T3's change (no T3 outcome depends on it);
  §8.1 item 4 lists it as the first step of the fix.

## §1. Shape and verdict

| Axis | Own invariant |
|---|---|
| A1 class (step 1) | `body` / `frameset`, HTML namespace (D-T5) |
| A2 names (step 2) | WindowEventHandlers members ∪ the reflecting six = `is_window_delegating_pair` (D-T2) |
| A3 activeness (step 3) | the base's node document (D-T4) == `active.document`, else null |
| A4 global (step 4) | `active.window` (D-T3) |
| A5 callers | attribute change steps at every timing + AttributeChange arm; getter; setter, both pairs (D-T1, D-T6) |
| A6 processing timing of Window-delegating pairs | XW: X-W until S-5; EW: bind pass, clone creation and the spec's writers only (D-T7) |
| A7 creation by clone | every element of a clone, every class, at creation (D-T10) |
| A8 `body`'s interface | HTML namespace (D-T11) |

Coupled from outside: A9 N1's compile-gate node clause (same derivation, D-T4); A10 C0's insertion adoption and A11
C1's inert template document, both **landed on the base** (H-T8, H-T21).

### §1.1 Outcome changes at T3 (toward the spec)

- WindowEventHandlers name on a `body` whose node document is not active (a DOMParser document, an inert template
  document, a cloned Document): base → page Window; T3 → null at every caller (facet 1; XW-3 to XW-5, XW-7, XW-8,
  XW-10).
- EW in the active document: base → the element; T3 → the Window (facets 3, 4; EW-1 to EW-4, EW-15). In a non-active
  document: base → the element; T3 → null (EW-5, EW-6, EW-13, EW-14, EW-16).
- EW is never re-raw'd at insertion (PM decision 129): a `window.on<T>` write survives a later insertion of the element
  (rev 1's W1, cell T-14), and an attribute set while the element was outside an active document is not delivered at
  its insertion (rev 1's W2, T-15; T-15b for a DOMParser-built attribute; T-17 for template contents).
- A clone's handler content attributes are processed at its creation (D-T10): an ordinary clone reads its function
  before any insertion (OR-2, T-20); a `body` / `frameset` cloned in the active document writes the Window at creation
  (EW-12, XW-11; T-16, T-20b).
- An initial `<body onload>` (and the other five, and an initial `<frameset>`'s 24) runs on the Window (H-T14; closes
  umbrella K-0's E-G1 (i)).
- A foreign-namespace `body` is not step 1's class (FN-1, FN-2) and does not get `HTMLBodyElement.prototype` (FN-3,
  FN-5).

### §1.2 Is T3 no worse than its base? (PM decision 129's premise)

Criterion: the umbrella's K-0, with T3's base as "HEAD" — at every observable step (an IDL getter read, a dispatch, a
read of `window.on<T>`), T3's outcome equals the base's, or equals the spec's where the base's does not. The base is the
state main is in just before T3 lands, which is what PM decision 129's option text compares against ("HEAD より悪い状態が
main に載らない"). §2.1 enumerates every pair class × timing × document kind. Rev 2's four residual classes, on rev 3's
base:

- **R-A — template contents: closed by C1 (on C0).** On the base a node in template contents or in a clone of them has
  the inert document I as its node document, by every way it gets there: parsed (C1 D-C3, D-C4), appended into
  `tpl.content` (C0-2's adoption), cloned from contents (C1 D-C6), or cloned with a template (C1 D-C6 through T3's clone
  path). I is not active, so step 3 returns null: an EW write in contents delivers nothing and its getter reads null
  (EW-13, EW-14: spec; the base wrote the element). XW in contents is now on the spec too (XW-8: the base wrote the
  Window). The node memo's derivation reads I through the node's `AssociatedDocument` (detached branch); the rev 2
  condition on C1's plan (a fragment-root rule or insertion adoption, rev 2 §8.2 C1 (i)) is met by C0-2. Cells T-17
  (red), C18 (step 1 rewritten).
- **R-B — a cross-document move then a detached state: closed by C0.** On the base an insertion adopts the node into the
  parent's node document (C0-1), so rev 2's document kinds X1 (a DOMParser node in a detached subtree of the bound
  document) and X2 (a bound node moved into a DOMParser document and removed) are A and P: XW-9 is "both", XW-10, EW-15
  and EW-16 are "spec". Cell T-18 (guard).
- **R-C — a clone created in the active document: closed in T3** (D-T10; PM decision 153). EW-12: T3 writes the Window at
  the clone's creation, as the spec does; the clone's getter reads the Window's value, which is now the attribute's
  function unless a later write replaced it (spec). Cell T-16 (red).
- **R-D — split.**
  - **R-D1 — a foreign `body`'s HTML interface: closed in T3** (D-T11). An SVG or MathML `body` takes
    `HTMLElement.prototype`, so `s.onhashchange = fn` is an ordinary property (spec), not a handler on `s` (rev 2's T3)
    nor a write to the Window (base). FN-3, FN-5; cell T-19 (red).
  - **R-D2 — a non-HTML, non-SVG `body`'s WindowEventHandlers content attribute: not closed in T3** (§2.2). FN-4.

So **PM decision 129's premise holds for T3 except one row** (FN-4, R-D2), which §2.2 sends back to the user as PM
decision 153 asks. Under O-7 (a) the count is 0.

### §1.3 Edge density

**Edge-dense** (*verdict*). Eight own axes meet in one algorithm with three spec callers and a creation path, and three
external couplings (A9–A11) change outcomes at the intersections (§2). So CLAUDE.md "Edge-dense work" applies:
plan-review is mandatory (rule (b)). **Split: none** — T3 is a narrowly-scoped per-PR slice under the approved EH-attrs
umbrella (§1.2 row T3, as edited by PM decisions 129 and 153), the rule's base case (c). Splitting by facet or by caller
would leave two target rules live (content attribute vs IDL, or body vs frameset); splitting D-T7 off would land the
target change without the timing change, which is rev 1's W1 / W2 state that PM decision 129 rejects; splitting D-T10
off would land rev 2's R-C, which PM decision 153 closes in T3; splitting D-T11 off would land rev 2's R-D1 (an SVG
`body`'s `onhashchange` setter installs a handler on it), worse than the base. Breadth: K = 2 specs, M = 11 rows (§3);
`webref coverage-map html 8.1.8.1 html 8.1.8.2 html 7.3.2 html 8.1.3.3.4 dom 4.4 dom 4.9 dom 4.5 html 16.3.2` → "ok →
single PR scope".

## §2. Coupled invariants

- **A1×A2** — steps 1–2 are one conjunction (`is_window_delegating_pair`); a frameset passes A1 for every A2 name.
- **A1×A5** — every caller reaches A1 through D-T1 only; the VM brand check and interface choice are A8 (D-T11).
- **A1×A6** — `is_x_w_pair` reads the same namespaced class (`body` half), so X-W's population is a subset of A1×A2.
- **A1×A8** — both test the HTML namespace for `body`; they are different questions (step 1's class vs the WebIDL
  interface) and stay two predicates over one component read (`EcsDom::is_html_namespace`). A foreign `body` is neither
  step 1's class nor an `HTMLBodyElement`, so they agree on every element.
- **A2×A5** — the IDL normal pair now asks A2 too: `body.onfocus` (HTMLElement accessor) → Window (cell T-5).
- **A2×A6** — every Window-delegating pair targets the Window or null; XW is re-raw'd at insertion, EW is not.
- **A3×A5** — null at each caller: content attribute → no write and no clear; getter → null; setter → return.
- **A3×A6** — an XW re-raw at an insertion into a non-active document delivers nothing (T-3); into the active one, the
  Window (C8). EW has no insertion delivery.
- **A3×A7** — a clone's node document is its source's (DOM *clone a node*, `document` = the node's node document; H-C7
  in the C1 memo: the clone stamps it), so a clone of a DOMParser `body` delivers nothing at creation (EW-11), a clone of
  a bound one the Window (EW-12), a clone of template contents nothing (EW-19).
- **A3×A9** — one derivation (D-T4): N1's compile gate and step 3 cannot disagree on a node's document.
- **A3×A10** — the node document follows the last insertion (C0): rev 2's X1 / X2 collapse to A / P (R-B closed).
- **A3×A11** — template contents and their clones are in I (C1): step 3 returns null there (R-A closed).
- **A4×A5** — `ActiveDocument` has three marshalling points (consumer construction, VM accessors, clone seam), one
  source each (`Vm::bind`'s `document` and Window; `HostData`).
- **A5×A7** — the clone path is a caller of D-2's function, not a fourth target rule: it reaches step 1–4 through D-T1.
- **A6×A7** — at creation every class is delivered; afterwards XW alone is re-raw'd at insertion (X-W), and the marker
  set at creation keeps OR from being re-processed at the first insertion (K-1).
- **A2×A9** — N1's C8h (`dd.body.setAttribute('onfocus', 'A')`) keeps its outcome (getter null, nothing runs) but by
  step 3 now: no entry is written (cell U-T3).
- **A5×A9** — the getter determines before N1's reconcile (getter step 1 before step 3).

### §2.1 Enumeration: pair class × timing × document kind (Base / T3 / spec)

Document kinds (the node document of the element at the step): **A** the bound, active document; **P** a DOMParser
document; **I** an inert template contents owner document (template contents and clones of them, C1); **U** no
resolvable node document (H-T16). On the base the node document follows the last insertion (C0); rev 2's kinds **X1**
(a DOMParser node in a detached subtree of the bound document) and **X2** (a bound node moved into a DOMParser document
and removed again) are kept as rows to show R-B's closure and are A and P respectively. Timings: **bind** (T-a1); **AC**
(`setAttribute` / `removeAttribute`, the attribute change steps); **set** / **get** (IDL); **ins→A** / **ins→P** (an
insertion into that kind, first or later); **parse** (creation by the DOMParser build); **clone** (creation by
`cloneNode`, T-c); **T-a3** (slice a's first-write processing). Verdict: **both** (T3 = base = spec), **spec** (T3 =
spec ≠ base), **base** (T3 = base ≠ spec), **✗** (neither). † = the base differs from `1ed126b4` (§0.1).

| Row | Class | Timing | Doc | Base | T3 | Spec | Verdict | Cells |
|---|---|---|---|---|---|---|---|---|
| XW-1 | XW | bind | A | Window | Window | Window | both | E4 (slice a) |
| XW-2 | XW | AC, set, get | A | Window | Window | Window | both | C20 (i), T-8a |
| XW-3 | XW | AC | P | page Window | nothing | nothing | spec | T-2 |
| XW-4 | XW | set | P | page Window | return | return | spec | T-1, C17 |
| XW-5 | XW | get | P | the Window's value | null | null | spec | T-1 |
| XW-6 | XW | ins→A | any | re-raw onto the Window over any write | same (X-W) | nothing | base | C8, C17, U2 |
| XW-7 | XW | ins→P | any | re-raw onto the page Window | nothing | nothing | spec | T-3, C19 |
| XW-8 | XW | AC, set, get | I | Window (†: I by C1; the target rule ignores it) | nothing / return / null | nothing / return / null | spec (R-A closed; rev 2: base) | C18 (step 1) |
| XW-9 | XW | AC, set | X1 ≡ A | Window | Window | Window | both (R-B closed; rev 2: ✗) | T-18 |
| XW-10 | XW | AC, set | X2 ≡ P | Window | nothing | nothing | spec (rev 2: base) | U-T1 |
| XW-11 | XW | clone, then ins→A | A | nothing at creation; re-raw at the insertion | Window at creation; re-raw at the insertion (X-W) | Window at creation; nothing at the insertion | creation: spec; insertion: base (XW-6) | T-20b |
| XW-12 | XW | parse, then ins→A | P → A | nothing at creation; re-raw onto the Window at the insertion (†: adopted by C0) | same | nothing | base (XW-6) | C17 |
| XW-13 | XW | AC, set | U | Window | Window | Window | both | U-T1 |
| XW-14 | XW | clone, then ins→A | P → A | nothing at creation; re-raw at the insertion | nothing at creation (null target); re-raw at the insertion | nothing | creation: both; insertion: base (XW-6) | U-T7 |
| EW-1 | EW | bind | A | element | Window | Window | spec | T-E1, U-T2 |
| EW-2 | EW | AC | A | element | Window | Window | spec | T-4, T-6, T-8b, C19b, C19c, P3 |
| EW-3 | EW | set (where an accessor exists: the reflecting six) | A | element | Window | Window | spec | T-5, T-7 |
| EW-4 | EW | get | A | the element's value | the Window's value | the Window's value | spec | T-4, T-5 |
| EW-5 | EW | AC, set | P | element | nothing / return | nothing / return | spec | T-6b, U-T3, C19b |
| EW-6 | EW | get | P | the element's value (null for a raw value, N1 †) | null | null | spec | N1's C8h |
| EW-7 | EW | ins→A after an AC / set | any | re-raw onto the element over any write | nothing | nothing | spec | T-14 (rev 1's W1) |
| EW-8 | EW | ins→A after an AC while in P | P → A | re-raw onto the element | nothing | nothing | spec | T-15 (rev 1's W2) |
| EW-9 | EW | ins→P | any | re-raw onto the element | nothing | nothing | spec | C19b |
| EW-10 | EW | parse, then ins→A | P → A | element at the first insertion | nothing | nothing (null at creation) | spec | T-15b |
| EW-11 | EW | clone, then ins→A | P → A | element at the first insertion | nothing (null at creation) | nothing | spec (rev 2: spec) | U-T6, U-T7 |
| EW-12 | EW | clone | A | Window: untouched. Clone's getter: null, then the attribute's function after its first insertion | Window := the attribute at creation. Clone's getter: the Window's value | Window := the attribute at creation | spec (**R-C closed**; rev 2: ✗) | T-16 |
| EW-13 | EW | AC, set | I | element (†: in I by C1 / C0) | nothing / return | nothing / return | spec (**R-A closed**; rev 2: ✗) | T-17 |
| EW-14 | EW | get; ins→A afterwards | I → A | the element's value (null for a raw value, N1 †); at the move out, re-raw onto the element | null; nothing at the move out | null; nothing at the move out | spec (**R-A closed**; rev 2: ✗) | T-17 |
| EW-15 | EW | AC, set | X1 ≡ A | element | Window | Window | spec (**R-B closed**; rev 2: ✗) | T-18 |
| EW-16 | EW | AC, set | X2 ≡ P | element | nothing | nothing | spec (**R-B closed**; rev 2: ✗) | U-T1 |
| EW-17 | EW | T-a3 | any | not processed | not processed | — | both (D-8) | C20 (i), U-T6 |
| EW-18 | EW | AC, set | U | element | Window | Window | spec | U-T1 |
| EW-19 | EW | clone (of contents, or of a template), then ins→A | I → A | nothing at creation; element at the first insertion | nothing at creation (null); nothing at the insertion | nothing | spec | U-T7 |
| FN-1 | FN: SVG `body` × the reflecting six; SVG `frameset` × any of the 24; MathML / other `body` / `frameset` × the reflecting six | AC, set | any | element | element | element (H-T22; `MathMLElement includes GlobalEventHandlers`, H-T23) | both | T-9b |
| FN-2 | FN: SVG `body` × a WindowEventHandlers name | AC | any | Window (tag-only) | element | element (H-T22) | spec | T-9 |
| FN-3 | FN: SVG, MathML or other non-HTML `body` × a WindowEventHandlers name | set, get (IDL) | any | Window (the body pair, H-T18) | an ordinary property (D-T11) | an ordinary property (no such attribute on `SVGElement` / `MathMLElement`) | spec (**R-D1 closed**; rev 2: ✗) | T-19 |
| FN-4 | FN: `body` outside the HTML and SVG namespaces (MathML, null, any other) × a WindowEventHandlers name | AC; ins (first, unmarked) | any | Window (tag-only); re-raw onto the Window at every insertion | element (namespaced step 1; not applicable is not modelled) | nothing (H-T23: not an event handler content attribute of that element) | `window.on<T>`: spec. Dispatch at the element: **✗ R-D2** (§2.2) | T-21 |
| FN-5 | FN: `frameset` outside the HTML and SVG namespaces × a WindowEventHandlers name | AC, set | any | element | element | nothing (H-T23) | base | — |
| FN-6 | FN pairs of FN-1, FN-2 (SVG, and the reflecting six elsewhere) | ins→A, ins→P | any | re-raw at every insertion (tag-only X-W; onto the Window for FN-2's pairs) | the ordinary once rule (FN pairs are not Window-delegating at T3, P-T11) | nothing at insertion | as OR-1 (FN-2's pairs leave the Window: spec there) | U-T6 |
| OR-1 | OR | every timing except clone | any | element | element (D-T1 steps 1–2) | element, or nothing where step 1 of the attribute change steps returns | base or both (D-T8) | U-T1 |
| OR-2 | OR | clone | any | nothing at creation; processed at the first insertion (once) | processed at creation (element; marked); not at the insertion | processed at creation | spec (umbrella G5 / K3; defect (2)) | T-20, C9, C5 (iii), C12 |

Rows not listed are covered by a listed one: `removeAttribute` follows its row's AC column (clear instead of set, on
the same target); a later bind processes nothing for every class (D-5); XW and EW in a non-active document at the bind
pass do not occur (the bind pass walks only the bound document, umbrella K-5); a fragment-parsed `body` / `frameset`
(an `html` context) is EW-10's and XW-12's case with the throwaway document in place of P (D-T7); a cloned Document's `body` is kind P's case with a cloned Document in place of the DOMParser
document (not active; spec null).

**Rows worse than the base: 1** (FN-4, residual R-D2; §2.2). Under O-7 (a): **0**.

### §2.2 Residuals for the user

PM decision 153: 「閉じられないものが残ったら、改めて伺います」. One residual remains.

- **R-D2 — a `body` outside the HTML and SVG namespaces, with a WindowEventHandlers content attribute (row FN-4; pin
  T-21).**
  - **Reason it is not closed in T3.** The spec's outcome is "nothing": the attribute change steps' step 1 returns,
    because a WindowEventHandlers name is not an event handler content attribute of a MathML element (MathML Core §2.1.4,
    H-T23) or of an element in any other non-HTML, non-SVG namespace (DOM §4.5 *element interface*: `Element`). That
    step is slot `#11-event-handler-content-attribute-applicability`, which the PM ratified as **closed by slice b**
    (D-T8; umbrella §8.1). Closing R-D2 in T3 means moving part of that slot into T3, which changes a ratified item;
    the designer does not accept that alone. No other T3-internal closure exists: an outcome of "the Window" (the
    base's) would need step 1 to treat a MathML `body` as the HTML body element, which is neither the spec nor PM
    decision 133's confirmed reading (D-T5).
  - **Observable effect (T3 as scoped).** `m = document.createElementNS('http://www.w3.org/1998/Math/MathML', 'body');
    m.setAttribute('onhashchange', 'X')`: the base writes `X` to the Window (`window.onhashchange` is a function; spec:
    `null`); T3 writes it on `m`, so `window.onhashchange` is `null` (spec), but a `hashchange` event dispatched at `m`
    runs `X` (spec and base: nothing). Same at `m`'s first insertion (the base re-raws onto the Window). Population:
    script-created only (`createElementNS`; the HTML parser cannot create a `body` in a foreign namespace, and there is
    no XML parser, umbrella §3); MathML `body` has no rendering or meaning. The IDL side (`m.onhashchange = fn`) is
    closed by D-T11 (FN-3).
  - **What would close it.**
    - **(a) recommended** — T3 takes the local-name half of the attribute change steps' step 1 (the slot's facets (1)
      and (3)): one predicate `is_event_handler_content_attribute_of(dom, element, name)` (NEW, session-private) used by
      `process_handler_content_attributes` (every timing) and the AttributeChange arm: for an HTML element, the
      GlobalEventHandlers names, plus the WindowEventHandlers names on `body` / `frameset` only; for an SVG element,
      every name (H-T22); for a MathML element, the names HTML supports on all HTML elements (H-T23); otherwise none.
      Every other outcome it changes is toward the spec: `div.setAttribute('onhashchange', X)` and a Document-only name
      (`onreadystatechange`, `onvisibilitychange`) on any element stop installing a handler (base: a handler on the
      element, run by a dispatch at it; spec: nothing). The slot's facet (2) (a non-null attribute namespace) stays with
      slice b (it needs `MutationEvent::AttributeChange` to carry a namespace, H-T19). Slot record: §8.3 item 8. Cost:
      one predicate, one cell set (T-21 red, U-T8), the slot split into "facets (1), (3): T3; facet (2): slice b".
    - **(b)** — keep the ratified split: R-D2 stays until slice b closes the slot; T-21 stays a residual pin.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | *determine the target of an event handler* steps 1–4 | 1 not an HTML body/frameset → self; 2 not WEH and not reflecting → self; 3 node document ≠ active document → null; 4 → active Window | `determine_event_handler_target` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs`, replacing `resolve_handler_target` (`:249`) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute getter steps 1–3 | null target → null; else get the current value (N1's reconcile) | `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` `native_event_handler_get` `:323`, `native_body_weh_get` `:439` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter steps 1–4 | null target → return; null value → deactivate; else activate | `native_event_handler_set` `:344`, `native_body_weh_set` `:461` | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | attribute change steps steps 2–5 | null target → return (no set, no clear); step 1 and step 5.1 not touched (§7; R-D2) | `crates/script/elidex-script-session/src/event_handler_consumer.rs:420`; `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:120-132` (D-T7's delivery by class and timing; `is_x_w_pair` (NEW)) | ✗ | yes |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | the WEH table and the Window-reflecting body element event handler set | content and IDL attributes on body and frameset act on the Window | `is_window_delegating_pair` `mod.rs:216`; `WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` `mod.rs:42` (unchanged) | ✓ | yes |
| WHATWG HTML §7.3.2 Browsing contexts | *active document* | in HEAD's model: the bound document (H-T6, H-T15) | `ActiveDocument` (NEW), marshalled at `crates/script/elidex-js/src/vm/vm_api.rs:292`, in `event_handler_attrs.rs` and in `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:212` | ✗ | no |
| WHATWG HTML §8.1.3.3.4 Relevant | *relevant global object* of the node document | active document → its Window (navigable's active window, §7.3.1) | `ActiveDocument::window` (NEW) | ✗ | no |
| WHATWG DOM §4.4 Interface Node | *node document*; *clone a single node* step 2.5 (attributes appended to the copy) | the base's derivation (D-T4); a clone's attributes are processed at its creation with the copy's node document | `effective_node_document` (NEW, D-T4 (i)) or the base's function (D-T4 (ii)), `crates/script/elidex-script-session/src/scripting.rs:104-110`; `process_created_subtree` (NEW) called after `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:212` | ✗ | yes |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4; *handle attribute changes* step 3 | the attribute change steps run for each appended attribute | `ProcessingTiming::Creation` (NEW) in `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:50` | ✗ | yes |
| WHATWG DOM §4.5 Interface Document | *element interface* (`Element` unless a spec says otherwise) | HTML interfaces for HTML-namespace elements only; `body`'s arm (D-T11) | `crates/script/elidex-js/src/vm/host/elements.rs:264`; `require_body_receiver` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:418` | ✗ | yes |
| WHATWG HTML §16.3.2 Frames | `frameset` element; `HTMLFrameSetElement includes WindowEventHandlers` | content attributes and inherited GlobalEventHandlers accessors → step 1; the interface itself absent (compat, D-T9) | `is_body_or_frameset` `crates/script/elidex-script-session/src/event_handler_processing/mod.rs:228` (namespaced, D-T5) | ✗ | yes |

Numbers and titles (2026-10-31): `webref heading html 8.1.8.1` (Event handlers, `#event-handler-attributes`),
`8.1.8.2` (Event handlers on elements, Document objects, and Window objects), `8.1.8.2.1` (IDL definitions), `7.3.2`
(Browsing contexts, `#windows`), `7.3.1` (Navigables), `8.1.3.3.4` (Relevant), `16.3` (Requirements for
implementations), `16.3.2` (Frames), `3.1.7` (DOM tree accessors), `4.12.3` (The template element), `7.2.2` (The Window
object), `8.1.3.4` (Enabling and disabling scripting); `webref heading dom
4.4` (Interface Node), `4.2.3` (Mutation algorithms), `4.5` (Interface Document), `4.9` (Interface Element); `webref
heading SVG2 15.8` (Event attributes, `#EventAttributes`); `webref heading mathml-core 2.1` (Elements and attributes),
`2.1.3` (Global Attributes), `2.1.4` (Attributes common to HTML and MathML elements,
`#attributes-common-to-html-and-mathml-elements`), `2.2.3` (DOM and JavaScript, `#dom-and-javascript`). `webref dfn
html`: 'determine the target of an event handler' → §8.1.8.1 `#determining-the-target-of-an-event-handler`;
'Window-reflecting body element event handler set' → §8.1.8.2; 'active document' → §7.3.2 `#active-document` (and
for=navigable §7.3.1 `#nav-document`); 'relevant global object' → §8.1.3.3.4 `#concept-relevant-global`; 'associated
Document' → §7.2.2 `#concept-document-window`; 'appropriate template contents owner document' → §4.12.3
`#appropriate-template-contents-owner-document`; 'WindowEventHandlers' → §8.1.8.2.1; 'frameset' → §16.3.2 `#frameset`;
`webref dfn dom`: 'node document' → §4.4 `#concept-node-document`, 'clone a node' → §4.4 `#concept-node-clone`, 'clone a
single node' → §4.4 `#clone-a-single-node`, 'append an attribute' → §4.9 `#concept-element-attributes-append`, 'handle
attribute changes' → §4.9 `#handle-attribute-changes`, 'attribute change steps' → §4.9
`#concept-element-attributes-change-ext`, 'element interface' → §4.5 `#concept-element-interface`, 'adopt' → §4.5
`#concept-node-adopt`; `webref dfn mathml-core`: 'MathMLElement' → §2.2.3 `#dom-mathmlelement`, 'Unknown MathML element'
→ §2.1 `#dfn-unknown-mathml-element`. Step texts: `webref body html event-handler-attributes`,
`event-handlers-on-elements,-document-objects,-and-window-objects`, `frames`, `navigables`, `relevant`,
`the-template-element`; `webref body dom mutation-algorithms`, `clone-a-single-node`,
`concept-element-attributes-append`, `handle-attribute-changes`, `concept-element-interface`; `webref body SVG2
EventAttributes`; `webref body mathml-core global-attributes`, `attributes-common-to-html-and-mathml-elements`,
`dom-and-javascript`, `mathml-elements-and-attributes`; `webref idl html HTMLFrameSetElement`, `webref idl mathml-core
MathMLElement`, `webref idl SVG2 SVGElement`.

### §3.1 User-input touch audit

Author-controlled inputs reaching the changed code: handler content attributes on `body` / `frameset` (parser-baked
and `setAttribute`; in a DOMParser document they reach the consumer only after the build, H-T9); IDL handler writes on
`body` / `frameset` / any element (`el.on<T> = …`); `DOMParser` documents; tree moves between documents (adopted by C0,
re-read at each determination); template contents (in I, C1); `createElementNS` with a foreign namespace and local
name `body` / `frameset`; `cloneNode` (shallow and deep, with clonable shadow roots and templates); the order of
`window.on<T>` writes, clones and insertions. Every change either narrows where a handler lands (null instead of the
page Window for a non-active document), moves it to the spec's target in the same realm (the active document's own
Window), processes a clone's attributes at its creation as the spec does, or removes an HTML interface from a foreign
`body`. The exception is residual R-D2 (§2.2): a WindowEventHandlers attribute on a MathML (or other non-HTML, non-SVG)
`body` lands on that element, where the spec delivers it nowhere. No case delivers a handler to a Window other than the
bound document's (H-T15; the unbind caveat is a caller contract that every production bind keeps). The clone path adds
no new delivery target: each delivery is D-T1's. Security boundaries (CSP, sandbox) are not changed; CSP stays
`#11-csp-inline-handler-enforcement`.

## §4. Mechanism

### §4.1 Home / kind

| Fact | Home / kind | Status |
|---|---|---|
| target determination | `determine_event_handler_target` (NEW), free fn, `session/event_handler_processing/mod.rs` | D-T1 |
| active-document context | `ActiveDocument` (NEW), `Copy` struct of two entities, same module; held by `EventHandlerAttributeConsumer` (constructor `EventHandlerAttributeConsumer::new(policy, active)`, replacing `with_policy`; `Default` derive removed); built per call by the VM accessors and the clone seam | D-T3 |
| node document | the base's derivation: `effective_node_document` (NEW, `session/scripting.rs`) under D-T4 (i), or C0's function under (ii) | D-T4 |
| step-1 class | `is_body_or_frameset` (existing, session-private, gains the HTML namespace) | D-T5 |
| steps 1–2 | `is_window_delegating_pair` (kept) | D-T2 |
| X-W's remaining population | `is_x_w_pair` (NEW), session-private, removed by S-5 | D-T7 |
| clone-path processing | `process_created_subtree` (NEW), `pub`, same module; `ProcessingTiming::Creation` (NEW); the element marker (D-1, existing) | D-T10 |
| `body`'s interface | the `body` arm of `tag_specific_t2b_prototype` and `require_body_receiver`, each adding `EcsDom::is_html_namespace` | D-T11 |
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
    let node_document = node_document_of(dom, target).unwrap_or(active.document); // D-T4 (i) or (ii)
    (node_document == active.document).then_some(active.window)   // steps 3–4
}

pub fn process_created_subtree(                 // (NEW), D-T10
    dom: &mut EcsDom, root: Entity, policy: SpecLevelPolicy, active: ActiveDocument,
) { /* uncapped, shadow-including, TemplateContentsWalk::Include: process at Creation, mark */ }

// in process_handler_content_attributes (D-T7), per handler content attribute:
let deliver = match (is_window_delegating_pair(dom, element, &name), timing) {
    (_, ProcessingTiming::Creation) => true,                          // every class (NEW)
    (false, _) => !marked,                                            // OR: once
    (true, ProcessingTiming::Bind) => true,                           // XW, EW
    (true, ProcessingTiming::Insertion) => is_x_w_pair(dom, element, &name), // XW only (NEW)
    (true, ProcessingTiming::HandlerWrite) => false,
};
// then: target = determine_event_handler_target(...); None → skip
```

`node_document_of` stands for D-T4's one derivation (its name is the base's). Call sites of
`determine_event_handler_target` after T3 (re-runnable: `/usr/bin/grep -rn 'determine_event_handler_target(' crates` →
exactly the five below plus the definition):
1. `process_handler_content_attributes` — `None` → `continue` (no write); signature gains `active` (so do
   `process_bound_document`, `process_before_handler_write` and `process_created_subtree`; the VM callers marshal it).
2. `handle_attribute_change` — `None` → return before set / clear.
3. `native_event_handler_get` / 4. `native_event_handler_set` — the normal pair, with `ActiveDocument` from `HostData`.
5. the body pair's shared core (`native_body_weh_get` / `_set` call one helper after `require_body_receiver`).

`process_created_subtree(` → the definition and one call (`native_node_clone_node`). The VM builds `ActiveDocument`
through one host helper (`ActiveDocument` from `host.document_entity_opt()` and `host.window_entity()`; `None` =
unbound → the accessor's existing silent no-op, and no clone processing, as for every unbound write, G7). It reads
`HostData` only (marshalling, Layering mandate); the `EcsDom::*` calls added in `vm/host/` are the namespace read of
D-T11 (a component read for interface selection, marshalling) and none other. The marked shortcut (`mod.rs:85`) is
unchanged: a marked `frameset` now has no insertion delivery, so the shortcut's "only `body` / `frameset` have
Window-delegating pairs" stays a sound over-approximation.

### §4.3 ECS-native and Layering checks

The algorithm is a free function over the ECS DOM (a system-like read: tag/namespace components, tree relation,
`AssociatedDocument`), returning an entity the caller writes `EventListeners` on, the existing component. The clone
path is one more system run over a freshly created subtree, writing the same component and the existing marker. OO →
ECS idiom: the "override the accessor on the subclass prototype" pattern (`HTMLBodyElement.prototype`'s delegation
pair) is replaced, for the target choice, by one query on the receiver entity; the prototype install remains only as
the WebIDL surface, now chosen by namespace as well as tag for `body`. "Run the attribute change steps when the clone
appends an attribute" (an observer on the attribute list in OO engines) → one processing system called at the end of the
clone operation over the created entities. `ActiveDocument` is a per-call value, not a store (D-T3 records why it is
not a component yet, PM decision 132). Layering: steps 1–4, D-T7's timing and the clone-path walk live in
`elidex-script-session`; `vm/host/` brand-checks, chooses the prototype, marshals `HostData` and the policy, and executes
the result on the listener store.

### §4.4 Docstring citation table (Axis 4)

| Item | Cites |
|---|---|
| `determine_event_handler_target` (NEW) | HTML §8.1.8.1 *determine the target of an event handler* steps 1–4; step 3 = "not an active document" in the single-browsing-context model (H-T6); B1 hand-off; the node document is the base's derivation (C0's adoption, C1's inert document) |
| `ActiveDocument` (NEW) | HTML §7.3.2 *active document*; §8.1.3.3.4 *relevant global object*; §7.3.1 (active window = active document's relevant global); `#11-browsing-context-state-ecs-components` (PM decision 132); the unbind caveat (H-T15) |
| `effective_node_document` (NEW, D-T4 (i) only) | DOM §4.4 *node document*; N4's planned home |
| `is_body_or_frameset` | HTML §8.1.8.1 step 1 ("a body element or a frameset element", HTML namespace); §16.3.2 `frameset` |
| `is_x_w_pair` (NEW) | umbrella §0.5 X-W as narrowed by PM decision 129; removed by slice b's S-5; slot `#11-eh-attrs-creation-time-call-sites` defect (7) |
| `process_created_subtree` (NEW), `ProcessingTiming::Creation` (NEW) | DOM §4.4 *clone a single node* step 2.5; §4.9 *append an attribute* step 4, *handle attribute changes* step 3; umbrella S-1 (clone path, PM decision 153); the entry point slice b's other paths call (S-6) |
| `native_event_handler_get` / `_set`, body pair | §8.1.8.1 getter steps 1–3 / setter steps 1–4 |
| the `body` arm, `require_body_receiver` | DOM §4.5 *element interface*; HTML `HTMLBodyElement`; the foreign-interface defect (§8.1 item 4) |
| `handle_attribute_change`, `process_handler_content_attributes` | §8.1.8.1 attribute change steps 2–3 (the docstring's "Not realised: step 3" at `mod.rs:64-68` is removed; the X-W paragraph `:70-75` is rewritten for D-T7) |
| `native_node_clone_node` | its clone-time seam list gains the handler processing (H-T24) |

## §5. Populations, touched files, named risks

### §5.1 Behavioural population (re-runnable)

The pairs whose target changes are exactly those `is_window_delegating_pair` accepts and HEAD's rule did not, plus the
non-active ones: `EVENT_HANDLER_ATTRS` rows with `HandlerScope::Window` (18; the slice-a ratchet test
`tests_insertion_and_write.rs:318-328` equates them with `webref idl html WindowEventHandlers`) and the six of
`WINDOW_REFLECTING_BODY_ELEMENT_EVENT_HANDLER_SET` (ratchet `:331-342`), on `body` and `frameset`. EW = 6 + 24 = 30
pairs; XW = 18. Unit cells U-T1 and U-T6 derive their tables from those two lists (not a sample). The clone path's
population is every element of a clone (D-T10); its production entry population is one (`cloneNode`, H-T24, H-T26; verified 2026-10-31 by H-T24's and H-T26's
`git grep` commands).

### §5.2 Existing tests whose expectation changes (red at T3 by design, rewritten in the same commit)

Computed from `tests_event_handler_attrs_window_delegating.rs` and `tests_event_handler_attrs_once.rs` at `1ed126b4`
(re-runnable: `git show 1ed126b4:crates/script/elidex-js/src/vm/tests/<file>`), with the base's outcome derived from N1,
C0 and C1 (re-run at the base, §9 item 1).

| Test | Base expectation | T3 expectation | Why |
|---|---|---|---|
| C17 `c17_domparser_body_weh_idl_write_then_append` (`:39`) | `true,true,true,true,true,true` | `true,false,true,true,true,true` | XW: the setter returns (null target, XW-4); the append still re-raws (XW-6, X-W until S-5). Stays a window pin for its last four values |
| C18 (`:64`) | after the setter `window.onhashchange === fn`; after the append a function `!== fn`; `window.hit === 1` | after the setter `window.onhashchange === null`; after the append a function `!== fn`; `window.hit === 1` | rev 3 (P-T9): `b` is adopted into I (C0-2), so `setAttribute` and the setter return (XW-8); the append re-raws (XW-6). Its first step becomes spec; it stays a window pin for the rest |
| C19b `c19b_body_reflecting_pair_reraw_at_insertion` (`:122`) | `true,true,true,true` | `true,true,false,true` | EW: `setAttribute('onfocus')` on a detached bound `b` writes the Window (EW-2); no re-raw at either append (EW-9, EW-7); the setter returns while `b` is in `dp` (EW-5); `b.onfocus` reads the Window's function; a non-bubbling `focus` at `b` runs nothing. Equals the spec → becomes a guard (renamed: no re-raw) |
| C19c `c19c_frameset_reflecting_pair_reraw_at_insertion` (`:132`) | same as C19b | same as C19b | facets 3 + 4, same steps; spec-equal → guard |
| P3 `p3_frameset_clone_classified_by_tag_and_name` (`:172`) | `true,true` | `true,false` | `onhashchange` on a `frameset` targets the Window (EW-2); the clone's copy is delivered to the Window at creation (EW-12) and is not re-raw'd at the append; `hashchange` at `f` runs nothing. Spec-equal |
| C9 `c9_clone_is_unprocessed_origin` (`tests_event_handler_attrs_once.rs:227`) | before insert: `c` unmarked; after: `c.onclick` is compiled A | before insert: `c` **marked**, `c.onclick` is compiled A; after: the same | the clone is processed at creation (OR-2, D-T10); the clone-policy non-copy (U6) still holds, the marker comes from the processing. Renamed (processed at creation) |
| C5 (iii), C12; C8, C19, C20 (i), C20 (ii); session U2, U3 (ii), U6, P1 (i), P1 (ii) | — | unchanged | C5 (iii): the clone in `dp` is processed and marked at creation, `c.onclick = fn` wins, the append does not re-raw: `true,true,true` as before. C12: the removal clears what creation wrote. XW pairs (C8, C19, U2, U3 (ii), P1), the T-a3 rule (C20), the ECS clone policy (U6) |

Kept green and checked: `tests_event_handler_attrs.rs` F-12 (`:235`), F-12b (`:446`), F-14 (`:263`); N1's C8h (outcome
unchanged, mechanism now step 3); `tests_call_listener.rs:120`; the shell E1–E8 (`content_eh_attrs_bind_tests.rs`); C1's
cells (the C1 memo §6). The session tests at `tests_insertion_and_write.rs:60`, `:101`, `:235` read `dom.window_entity()`
and switch to the harness's Window (`test_support::install` / `init_fresh` take `ActiveDocument`).

### §5.3 Touched files

Sizes and paths verified 2026-10-31 against `1ed126b4` by the §0.2 commands and `wc -l` on each file.

- `crates/script/elidex-script-session/src/event_handler_processing/mod.rs` — D-T1/D-T2/D-T3/D-T5/D-T7/D-T10; deletions;
  harness (`test_support::install` / `init_fresh` take `ActiveDocument`). 410 lines + T3's (< 1000; if it nears 1000
  at the base, the clone-path walk is the seam to split out first, as a standalone prereq).
- `crates/script/elidex-script-session/src/event_handler_consumer.rs` — constructor, both arms (545 lines).
- `crates/script/elidex-script-session/src/scripting.rs` — D-T4 (i) extraction only (293 lines).
- `crates/script/elidex-script-session/src/lib.rs` — exports.
- `crates/core/elidex-ecs/src/dom/mod.rs` — delete `EcsDom::window_entity` (930 lines; a deletion, no growth).
- `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` — accessors, `require_body_receiver` (745 lines + N1's
  changes; < 1000).
- `crates/script/elidex-js/src/vm/host/elements.rs` — the `body` arm (425 lines).
- `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` — the clone seam (293 lines).
- `crates/script/elidex-js/src/vm/consumer_dispatcher.rs`, `crates/script/elidex-js/src/vm/vm_api.rs` — marshal
  `ActiveDocument` at bind.
- tests: `session/event_handler_processing/tests_target.rs` (NEW), `tests_bind.rs`, `tests_insertion_and_write.rs`
  (harness); `elidex-js/vm/tests/tests_event_handler_target.rs` (NEW), `tests_event_handler_attrs_window_delegating.rs`
  and `tests_event_handler_attrs_once.rs` (§5.2); `crates/shell/elidex-shell/src/content_eh_attrs_bind_tests.rs` (cell
  T-E1; 346 lines).
- docs: the umbrella (§8.4, in this PR).

Not touched: `crates/core/elidex-ecs/src/dom/classify.rs` and `crates/dom/elidex-dom-api/src/char_data/document_props.rs`
(D-T5, PM decision 133); `crates/script/elidex-js/src/vm/host_data/mod.rs` (D-T11; Q-14);
`crates/dom/elidex-dom-api/src/node_methods/clone.rs` (D-T10's rejected site).

### §5.4 Named risks

- **R1 — the residual R-D2** (§2.2): pinned (T-21), O-7.
- **R2 — merge contact with N1, C0 and C1** in `event_handler_attrs.rs`, `scripting.rs` (N1 narrows clause (b)'s
  arguments to `Entity`; C0 may collapse its derivation, D-T4) and the clone code (C1 D-C6 changes
  `replicate_template_contents`); T3 is written on their landed base (§9 item 1).
- **R3 — `ActiveDocument` staleness.** The consumer's copy is fixed at construction; it is rebuilt at every bind (slice a
  H13), and each production `Vm` binds one `EcsDom`'s one bound document (H-T15). The unbind caveat (H-T15) is the
  relied-on contract; a cross-`EcsDom` rebind would make step 4 return the old World's Window (B1 removes the
  possibility).
- **R4 — `onerror` on `<body>`** now lands on the Window and compiles with the one-argument form, where the spec gives a
  Window `onerror` five parameters: `#11-event-handler-attribute-edge-cases` facet 2 becomes reachable through
  `<body onerror>` (§8.3 item 4).
- **R5 — clone-path cost.** A deep clone of n elements adds one walk of n and one attribute scan per element (the walk
  is the bind pass's, H-T25). `cloneNode` already walks the subtree twice (ECS clone, form state); no cap is introduced.
- **R6 — a future clone path that bypasses the entry point** (Range element clones, H-T26): processed at its first
  insertion by slice a's fallback (HEAD-equal); the Range slot records the entry point (§8.3 item 7).

## §6. Test plan

Red-first: each **red** cell is committed failing on T3's base before the fix commit. Every VM cell uses
`with_page` / `run` (`elidex-js/vm/test_helpers.rs:166`, `:191`; `dp` is a DOMParser document with an empty body, `el` a
connected element, `fn` counts its calls in `fnRan`). *Base* values are the base's (§0.1).

### §6.1 VM integration (`tests_event_handler_target.rs`, NEW)

| Cell | Kind | Script | Base | T3 (= spec unless marked) |
|---|---|---|---|---|
| T-1 | red (facet 1, IDL; XW-4, XW-5) | `dd = new DOMParser().parseFromString('<body></body>','text/html'); dd.body.onhashchange = fn; r.push(window.onhashchange === null, dd.body.onhashchange === null)` | `false,false` (`fn` on the Window) | `true,true` |
| T-2 | red (facet 1, attribute change steps; XW-3) | `dd.body.setAttribute('onhashchange','window.hit = 1'); window.dispatchEvent(new Event('hashchange')); r.push(window.onhashchange === null, window.hit === undefined)` | `false,false` | `true,true` |
| T-3 | red (facet 1, XW re-raw into a non-active document; XW-7) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; dp.body.append(b); r.push(window.onhashchange === fn)` | `false` (re-raw) | `true` |
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
| T-16 | **red (R-C closed; EW-12)** | `b = document.createElement('body'); b.setAttribute('onfocus','window.hit = 1'); window.onfocus = fn; c = b.cloneNode(); r.push(window.onfocus === fn, c.onfocus === window.onfocus); el.append(c); r.push(window.onfocus === fn)` | `true,false,true` | `false,true,false` (rev 2's T3: `true,true,true`) |
| T-17 | **red (R-A closed; EW-13, EW-14)** | `b = document.createElement('body'); tpl = document.createElement('template'); tpl.content.append(b); b.setAttribute('onfocus','window.hit = 1'); r.push(window.onfocus === null); document.body.append(b); r.push(b.onfocus === null, window.onfocus === null)` | `true,false,true` (raw on `b` in I reads null by N1; the move out re-raws onto `b`, which compiles) | `true,true,true` |
| T-18 | guard (R-B closed; XW-9) | `p = document.createElement('div'); dd = parseFromString('<body></body>','text/html'); b = dd.body; p.append(b); b.setAttribute('onhashchange','window.hit = 1'); r.push(typeof window.onhashchange === 'function')` | `true` | `true` (rev 2's T3 on `74e15df1`: `false`); kills a derivation that ignores C0's adoption |
| T-19 | **red (R-D1 closed; FN-3; D-T11)** | `s = document.createElementNS('http://www.w3.org/2000/svg','body'); s.onhashchange = fn; s.dispatchEvent(new Event('hashchange')); r.push(fnRan === 0, window.onhashchange === null, s instanceof HTMLBodyElement); m = document.createElementNS('http://www.w3.org/1998/Math/MathML','body'); m.onhashchange = fn; r.push(window.onhashchange === null); try { Object.getOwnPropertyDescriptor(HTMLBodyElement.prototype,'onhashchange').get.call(s); r.push(false) } catch (e) { r.push(e instanceof TypeError) }` | `true,false,true,false,false` | `true,true,false,true,true` (rev 2's T3: `false,true,true,…`) |
| T-20 | red (OR-2; umbrella G5 / K3) | `t = document.createElement('div'); t.setAttribute('onclick','globalThis.a = 1'); c = t.cloneNode(); r.push(typeof c.onclick === 'function'); c.dispatchEvent(new Event('click')); r.push(globalThis.a === 1)` | `false,false` | `true,true` |
| T-20b | red (XW-11) | `b = document.createElement('body'); b.setAttribute('onhashchange','window.hit = 1'); window.onhashchange = fn; c = b.cloneNode(); r.push(window.onhashchange === fn)` | `true` | `false` (the clone's attribute is written to the Window at creation) |
| T-21 | **residual pin R-D2** (FN-4; O-7) | `m = document.createElementNS('http://www.w3.org/1998/Math/MathML','body'); m.setAttribute('onhashchange','window.hit = 1'); m.dispatchEvent(new Event('hashchange')); r.push(window.hit === undefined, window.onhashchange === null)` | `true,false` | T3 as scoped: `false,true`. Spec: `true,true`. Under O-7 (a): red, `true,true` |

T-21 asserts T3's outcome with a comment naming the closer (`#11-event-handler-content-attribute-applicability`), unless
O-7 (a) turns it into a red cell.

### §6.2 Unit (session, `tests_target.rs`, NEW)

| Cell | Kind | Shape | Oracle |
|---|---|---|---|
| U-T1 | red (full enum) | for every `EVENT_HANDLER_ATTRS` name and every reflecting name × tag {HTML `body`, HTML `frameset`, `div`, SVG `body`} × node-document class {the active document (connected; detached owned), a non-active Document (a `create_document_node()` document: connected; detached owned), the inert document I (a node in a template's contents, C1), unresolvable}: `determine_event_handler_target` | the spec table: not Window-delegating → `Some(target)`; Window-delegating and active (or unresolvable) → `Some(window)`; else `None`. The base's rule fails every `frameset`, reflecting, foreign-namespace and non-active WEH row |
| U-T2 | red (bind pass, facet 4; EW-1) | pre-bind page with `<body onload=A>`; `init_fresh` | the Window's `load` entry is raw `A`; the body has no `load` entry. Base: the reverse |
| U-T3 | red (C8h's store; EW-5) | foreign document's `body`, `setAttribute('onfocus', 'A')` through the consumer | no `focus` entry on the body and none on the Window. Base: raw `A` on the body |
| U-T4 | guard (D-5 keying) | U12's shape: bound document `d2 = create_document_node()` ≠ `document_root()`; a `body` in `d2` with `onhashchange=A`; `active = {d2, w}` | delivered to `w`; a `document_root()`-keyed step 3 returns `None` |
| U-T5 | guard (D-T4 refactor) | `scripting.rs` tests (`:117-`) | unchanged and green (D-T4 (i)); under (ii), C0's tests |
| U-T6 | red (D-T7 delivery matrix) | for class ∈ {XW, EW, OR, FN (SVG `body` × a WEH name)} (derived from the two lists and a `div` name) × timing ∈ {Bind, Creation, first Insertion of an unmarked element, later Insertion of a marked element, HandlerWrite}: an element carrying one such attribute plus one ordinary attribute, its target's entry taken (`take_uncompiled`) after the first delivery | XW: delivered at Bind, Creation and every Insertion, never at HandlerWrite. EW: delivered at Bind and Creation only. OR and FN: Creation, or once. Base: EW and FN delivered at every Insertion |
| U-T7 | red (D-T10 coverage) | `process_created_subtree` over a clone root whose subtree has (i) a chain deeper than `MAX_ANCESTOR_DEPTH` and a parent wider than it (the U10 / U11 precedents), (ii) a clonable shadow root with `<button onclick=A>`, (iii) a `<template>` whose contents hold `<div onclick=A>` and a `<body onfocus=B>`, built with each owner (active, a non-active Document, I) | every element of (i)–(iii) is marked; ordinary entries raw on their elements; the contents' `<body>` delivers nothing (I, EW-19); a non-active owner's `<body>` delivers nothing (EW-11, XW-14) |
| U-T8 | red, under O-7 (a) only (R-D2) | `is_event_handler_content_attribute_of` over every `EVENT_HANDLER_ATTRS` name × element {HTML `body`, HTML `frameset`, HTML `div`, SVG `body`, SVG `rect`, MathML `body`, MathML `mi`, null-namespace `body`}, through the AttributeChange arm | HTML: GlobalEventHandlers names everywhere, WindowEventHandlers names on `body` / `frameset` only, Document-only names nowhere; SVG: every name; MathML: GlobalEventHandlers names only; null namespace: none. The base: every name installs |

### §6.3 End to end (shell)

| Cell | Kind | Markup | Base | T3 |
|---|---|---|---|---|
| T-E1 | red (H-T14, EW-1) | `build_main_document("<body onload=\"document.body.dataset.l=1\">…")`, then `probe_body_attr(&pipeline, "data-l")` (the E4 pattern, `content_eh_attrs_bind_tests.rs:165-181`) | `None` | `Some("1")` (H-T20) |

### §6.4 Mutation guards

| Mutant | Killed by |
|---|---|
| step 3 dropped | T-1, T-2, T-3, T-15, T-17, U-T1, U-T3 |
| step 3 keyed on `document_root()` | U-T4 |
| step 3 reading a derivation that ignores the inert document or C0's adoption | T-17, T-18, U-T1 (I rows) |
| reflecting set dropped from step 2 | T-4, T-5, T-8b, T-14, U-T1, U-T2 |
| `frameset` dropped from step 1 | T-6, T-7, P3, U-T1 |
| class test without namespace | T-9, T-9b, U-T1 |
| an IDL accessor bypassing D-T1 | T-1 (body pair), T-5 / T-7 (normal pair) |
| unresolvable → `None` | U-T1 (unresolvable row) |
| X-W not narrowed (EW re-raw'd at insertion, rev 1's form) | T-14, T-15, T-17, U-T6 |
| EW under the ordinary once rule (α) | T-15b, U-T6 (first-insertion row) |
| X-W narrowed too far (XW not re-raw'd) | C8, U2, U-T6 |
| clone path absent, or skipping a class | T-16, T-20, T-20b, U-T6 (Creation row), U-T7 |
| clone path walking light tree only, or capped, or excluding template contents | U-T7 |
| clone path not marking (re-processed at insertion) | C9, C5 (iii), U-T7 |
| `body` arm or `require_body_receiver` tag-only | T-19 |

Withdrawn (rev 2): "step 3 by `owner_document` alone" (P-T12; D-T4).

## §7. Out of scope and slots

| Item | Why not T3 | Home |
|---|---|---|
| X-W for XW (`body` × WEH re-raw at insertion); the D-8 restriction | slice b's S-5 (PM decision 129 moved only EW's part) | slice b; `#11-eh-attrs-creation-time-call-sites` defect (7) (§8.3 item 2) |
| creation-time processing on S-1's other paths (fragment build, DOMParser build, declarative shadow content, pre-bind template contents) | slice b's S-1 (PM decision 153 moved only the clone path) | slice b S-1 |
| foreign elements' HTML interfaces other than `body`'s | the general fix needs the SVG / MathML interface families (D-T11) | proposed defect text (§8.1 item 4; PM to route; `M4-13:#svg` row as candidate home) |
| **the `HTMLFrameSetElement` interface**: no `HTMLFrameSetElement.prototype`, so its 18 WindowEventHandlers IDL attributes and `cols` / `rows` read `undefined` | an interface surface; compat (PM decision 130, D-T9) | `#11-htmlframesetelement-interface` (§8.1 item 1) |
| **attribute change steps step 1** (applicability, attribute namespace, MathML) | D-T8 (ratified: slice b); residual R-D2 asks the user (§2.2) | `#11-event-handler-content-attribute-applicability`, closed by slice b (§8.1 item 2), unless O-7 (a) |
| `document.body`'s tag-only class test (H-T12) | PM decision 133: T3's steps 3–4 do not use "the body element" | proposed defect text (§8.1 item 3; PM to route) |
| attribute change steps step 5.1 (CSP) | not modelled | `#11-csp-inline-handler-enforcement` |
| multi-realm step 4; the unbind caveat | B1 | hand-off (§8.2) |
| Range element clones (`cloneContents`, `extractContents`' partial element clone) | not implemented (H-T26) | `#11-range-clone-and-surround-contents` (§8.3 item 7) |
| the ASCII-case-insensitive tag compare (an XML-document `createElementNS(html, 'BODY')` counts as `body`) | engine-wide tag-predicate convention (`is_base_element` precedent) | not changed; noted |

Own deferrals: **0**. The one residual (R-D2) is the user's decision (§2.2, O-7).

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
   by slice b (D-T8). Text (as ratified): "pre-existing. The attribute change steps (HTML §8.1.8.1 Event handlers) step 1
   returns when the attribute's namespace is not null, or its local name is not the name of an event handler content
   attribute on the element. elidex realises neither test: (1) the consumer looks the name up in `EVENT_HANDLER_ATTRS`
   regardless of the element (`event_handler_attr_lookup`,
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
   b's plan. ⚠ No remedy reviewed." Rev 3 adds a landing-time record for facet (3) (§8.3 item 8): MathML Core §2.1.4
   determines it (H-T23).
3. **`document.body`'s class test** (pre-existing; PM to route; PM decision 133 keeps it out of T3). Text: "HTML §3.1.7
   DOM tree accessors: *the body element* is the first child of the `html` element that is either a body element or a
   frameset element, both HTML-namespace elements. `first_body_or_frameset_child`
   (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:169-173` at `74e15df1`; the `document.body` getter and the
   VM focus fallback) compares the tag only, so an SVG `body` child of `<html>` placed before the HTML body is returned
   as `document.body`. Since T3 the event-handler step-1 test is namespaced and this one is not: two predicates for one
   class test. **Trigger:** the next PR touching `document.body` or `elidex-dom-api` tree accessors. ⚠ No remedy
   reviewed."
4. **Foreign elements take HTML interfaces** (pre-existing, engine-wide; PM to route; the ledger has no row for it,
   checked 2026-10-31; candidate home: the master roadmap's `M4-13:#svg` row). Text (rev 3): "The VM chooses an element
   wrapper's prototype by tag name alone: `prototype_kind_for` returns `PrototypeKind::Element` for every `TagType`
   entity (`crates/script/elidex-js/src/vm/host_data/mod.rs:1626-1628` at `1ed126b4`), and the per-tag selection
   (`crates/script/elidex-js/src/vm/host/elements.rs:186-`) uses `tag_matches_ascii_case`
   (`host_data/mod.rs:1587-1596`), which reads no namespace, as do the per-interface receiver checks that call it
   (e.g. `html_style_proto.rs:90`). So an SVG or MathML element whose local name is an HTML tag (`a`, `script`,
   `style`, `title`, `div`, …) gets that HTML interface's prototype, and every other foreign element gets
   `HTMLElement.prototype`; no `SVGElement` / `MathMLElement` interface is exposed (WHATWG DOM §4.5 *element interface*;
   MathML Core §2.2.3; SVG2). Since T3 (#<PR>, `<squash>`) the `body` arm and `HTMLBodyElement`'s receiver check also
   require the HTML namespace (T3 memo D-T11), so a foreign `body` takes `HTMLElement.prototype`. The rest is ordered by
   what it needs: (i) arms whose local name is not an SVG or MathML element name can require the HTML namespace with
   no new interface (toward the spec at every member); (ii) arms whose local name is also a foreign element name
   (`a`, `script`, `style`, `title`, …) need the foreign interfaces first, since `HTMLElement.prototype` lacks members
   the foreign interface has (e.g. `SVGScriptElement`'s `type`). **Trigger:** an SVG / MathML DOM interface PR, the
   `M4-13:#svg` program's plan, or a WPT `svg/` / `mathml/` subset. ⚠ No remedy reviewed."

### §8.2 Hand-offs

- **B1** (`docs/plans/2026-06-agent-scoped-ecsdom-world.md` §5 req 5 / req 7): `ActiveDocument` is the single-browsing-
  context form of steps 3–4. Under a multi-realm World, "active" and "relevant global" become per document; B1's plan
  replaces `ActiveDocument`'s three marshalling points (the consumer's construction in `Vm::bind`, the VM accessor
  helper, the clone seam) with a per-document resolution, and `determine_event_handler_target`'s body; no caller
  changes. BFCache (a document kept non-active with its Window) is the case that makes this a real distinction. The
  unbind caveat (H-T15) becomes structurally impossible under "1 agent = 1 World = 1 Vm". The ECS home of the pair is
  `#11-browsing-context-state-ecs-components` (PM decision 132).
- **N4** (node memo §0.4 D8, §4 "Node document"): under D-T4 (i), `effective_node_document` exists from T3 as a session
  function with both readers, and N4's move into `EcsDom` relocates one function and its two calls; under (ii), N4's row
  is C0's to re-state.
- **Slice b**: S-5 removes X-W for XW and the D-8 restriction; it inverts C8, U2, C17's last four values and C18's
  later steps. S-1's remaining paths (fragment build, DOMParser build, declarative shadow content, pre-bind template
  contents) call T3's entry point `process_created_subtree` (or extend it), reading `ActiveDocument` at each creation
  (S-6 extended, umbrella rev 8.5). Slice b also closes `#11-event-handler-content-attribute-applicability` (D-T8),
  unless O-7 (a) moves its facets (1) and (3) into T3.
- **C0** (lands before T3): T3 relies on C0-1 (every insertion adopts), C0-2 (template contents' fragment as parent
  document) and C0-3 (before the Insert event, so X-W's re-raw at T-a2 reads the adopted document); and on C0-4's
  outcome for D-T4. If C0's plan changes any of them, D-T4 and rows XW-9/10, EW-15/16, XW-12 are re-derived.
- **C1** (lands before T3): T3 relies on D-C2 to D-C6 (every node in template contents or a clone of them resolves to
  I). Downstream for the C1 memo's owner (not edited here): the C1 memo's base statements "N1, T3, C0 landed" (§0.1, §1
  K-0 lens, §6 column header) and "C1's implementation is cut after N1, T3 and C0 land" read "N1 and C0" under PM
  decision 153; its edge c2 (T3 → C1), cell T18 ("rewrite umbrella C18; needs T3"), A8×A9 and R-C4 ("C18 changes to
  T3's W2 shape") assume T3 first: C18 is rewritten by T3, after C1 (P-T9, §5.2); its edge note "T3 and C0 are
  independent of each other unless C0's plan collapses T3's `effective_node_document`" becomes x9 (C0 → T3) with D-T4's
  two branches; its R-1 ("add a C1 row to the umbrella §1.2") is done by this PR (umbrella rev 8.5).

### §8.3 Records for existing slots (for the PM, exact texts; all at T3's landing unless stated)

1. `#11-event-handler-target-determination` — **close**, with this record: "Closed by T3 (#<PR>, `<squash>`): facets
   (1)–(4) realised by `determine_event_handler_target`, the only caller of steps 1–4 (attribute change steps, IDL getter
   and setter, a clone's creation); facet (2) in HEAD's single-browsing-context model (B1 replaces the source of
   `ActiveDocument`). T3 landed after C0 (`#11-cross-document-adopt-on-insert`) and C1
   (`#11-template-contents-owner-document`), so step 3 reads the spec's node document for template contents and after
   cross-document moves. Related defects homed elsewhere: the `frameset` interface → `#11-htmlframesetelement-interface`;
   foreign-element interfaces other than `body`'s → <the PM's home for §8.1 item 4>; a non-HTML, non-SVG `body`'s
   WindowEventHandlers content attribute → `#11-event-handler-content-attribute-applicability` <unless O-7 (a)>." In the
   same edit, the row's first paragraph: "(`…/event_handler_processing/mod.rs:249`) … code as of `fd6955e8`" →
   "(`crates/script/elidex-script-session/src/event_handler_consumer.rs:328` at `fd6955e8`;
   `event_handler_processing/mod.rs:249` since #563)", "in three ways" → "in four ways (facet (4) below)", and
   "**Owner**: the IBP-sandbox program (Layout lane), since its N-series rewrites the handler path." → "**Owner**: the
   EH-attrs-at-creation program (Layout lane), T3 (PM decision 25; sub-bullet below)." (P-T1, P-T2; a landing-time
   record, not applied before T3 lands.) The row's scheduling note (PM decisions 122/123) → "Order (PM decision 153):
   N1 → C0 → C1 → T3 → slice b".
2. `#11-eh-attrs-creation-time-call-sites` — (a) defect (7): replace "(7) a `<body>` / `<frameset>` handler that
   delegates to the Window (X-W) is re-raw'd at every insertion over any write, and a `<body>` outside an active document
   delivers it to the page Window." by "(7) a `<body>` handler for a WindowEventHandlers name (X-W, narrowed by T3) is
   re-raw'd onto the Window at every insertion of the element into the active document, over any write (since T3 the
   Window-reflecting names on `<body>` and every Window-delegating name on `<frameset>` are not re-raw'd, PM decision
   129, and an insertion into a non-active document delivers nothing)." (rev 2's clause on a clone's Window-reflecting
   attribute is dropped: T3 processes clones at creation, PM decision 153). (b) Defect (2) "a never-inserted clone of an
   element with a handler content attribute reads `null` from `on<T>`" → closed by T3 (struck, with "closed by T3, the
   clone path of S-1, PM decision 153"). (c) Defect (3) → "after `c = document.body.cloneNode()`, a `window.on<T>`
   assignment for a WindowEventHandlers name is overwritten by `c`'s attribute at each insertion of `c` (X-W; since T3
   the attribute is also written at `c`'s creation, as the spec does, and the Window-reflecting names are not
   overwritten)". (d) Defect (1): append "(since T3 not for a clone: a clone is processed and marked at its creation)".
   (e) The classification sentence: drop "and (7)'s pairs that HEAD targets at the element (both to the spec's
   `null`)"; "three" becomes "two". (f) **Closes**: "which removes the restriction, X-W and processing at insertion"
   stays; the closing path reads "slice a lands → N1 → C0 → C1 → T3 → slice b" (PM decision 153). This is the text the
   umbrella §8.2 carries from rev 8.5 (§8.4 item 13).
3. ~~`#11-cross-document-adopt-on-insert` — consumer list~~ — withdrawn (rev 3): C0 closes that slot before T3, so T3
   records nothing there. **At PM decision 153 (not T3's landing):** its sub-bullet "Order: N1 implementation → T3 → C0 →
   C1 → slice b" (PM decision 143) → "Order (PM decision 153): N1 implementation → C0 → C1 → T3 → slice b".
4. `#11-event-handler-attribute-edge-cases` — append to facet (2): "Reachable through `<body onerror>` since T3 (the
   reflecting set now targets the Window)."
5. ~~`#11-template-contents-owner-document` — consumer note~~ — withdrawn (rev 3): C1 closes that slot before T3. **At
   PM decision 153:** its sub-bullet "Trigger completed 2026-10-31" order "N1 implementation → T3 → C1 → slice b" → "N1
   implementation → C0 → C1 → T3 → slice b (PM decision 153)".
6. Umbrella K-0's E-G1 (i) and the ledger row's facet (4) bullet: closed by T3 (the record of item 1 covers both).
7. `#11-range-clone-and-surround-contents` — add: "When Range clones elements (`cloneContents`, `extractContents`' clone
   of a partially-contained element), each clone's handler content attributes are processed at its creation through
   `elidex_script_session::process_created_subtree` (NEW: T3, #<PR>; DOM §4.4 *clone a single node* step 2.5); until then a
   Range clone falls back to processing at its first insertion (slice a)."
8. `#11-event-handler-content-attribute-applicability` — facet (3) record: "MathML Core §2.1.4 (Attributes common to
   HTML and MathML elements) admits on MathML elements the event handler content attributes HTML supports on all HTML
   elements, as defined in the `MathMLElement` IDL (which includes `GlobalEventHandlers` only); WindowEventHandlers names
   are not among them (HTML §8.1.8.2: body and frameset elements only). The facet's population is every `body` outside
   the HTML and SVG namespaces (DOM §4.5 *element interface*); since T3 such a `body`'s WindowEventHandlers attribute
   lands on that element (T3 memo R-D2, cell T-21)." Under O-7 (a) instead: "Facets (1) and (3) closed by T3 (#<PR>,
   `<squash>`, `is_event_handler_content_attribute_of`); facet (2) (attribute namespace) remains, closed by slice b."

### §8.4 Umbrella in-place edits carried by this PR (PM decisions 129, 131, 153)

Each edit keeps history in the same style as PM decisions 93 / 105 (umbrella revs 8.1 to 8.3): a changelog entry names
the PM decisions and this memo, and every edited clause says what it said before ("until rev 8.4 / 8.5: …") where the
old statement was decided text. Rev 8.4 (PM decisions 129–131) and rev 8.5 (PM decision 153) are carried together.

Rev 8.4 (from rev 2):
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

Rev 8.5 (this rev 3; PM decision 153):
15. Changelog: *Rev 8.5* entry.
16. Header: the program's nested PRs include C0 and C1; T3 is no longer "independent".
17. §0.6 lead (slice b's base) and S-1 (the clone path is T3's), S-6 (T3's entry point is kept and extended).
18. §1.1 row I4 (slice b column: the clone at T3).
19. §1.2: rows **C0** and **C1** added; rows T3 (scope: S-1's clone path; `body`'s interface; prerequisites) and slice b
    (scope without the clone path; prerequisites); the **Order** line (N1 → C0 → C1 → T3 → slice b).
20. §1.3: x5 (C1 decided as branch A, PM decision 123), x6 (satisfied transitively); new **x7** (C0 → C1), **x8** (C1 →
    T3), **x9** (C0 → T3), **x10** (N1 → C0, scheduling); the acyclicity line.
21. §1.4: the interim's order sentence and the closing path.
22. §1.5: heading; bullets K-2 / K-3, G5 / K3, K2, G2 rest and X-W (C18's later steps).
23. §1.6: C1's handling decided (branch A, PM decision 123).
24. §2.1 rows I1×I4 and I4×I5 (the clone at creation from T3).
25. §5.1 row `cloneNode` (T3).
26. §7 G5 (the clone half at T3).
27. §8.1 bullets `#11-template-contents-owner-document` and `#11-cross-document-adopt-on-insert` (C1 and C0 of this
    program); the first bullet's closing sentence; the applicability bullet (a pointer to R-D2, no change to its
    closure).
28. §8.2 defects (1), (2), (3), (7), the closing path, the trigger and the confidence line (as §8.3 item 2).
28b. §9 N-6 (the clone's call site is T3's).
29. §10.2 and §10.3 (base and order).
30. §11: O9 decided for the clone path by T3's plan.

## §9. Definition of done

1. T3's branch is cut from main after N1's implementation, C0 and C1 land (PM decision 153); every `1ed126b4`
   coordinate in §0.2 is re-measured there by its command, D-T4's branch ((i) or (ii)) is fixed from C0's landed code,
   and the memo's coordinates and *Base* columns are updated before implementation starts.
2. `/elidex-plan-review` passes on this memo (mandatory, §1.3), and O-7 is decided by the user (through the PM) before
   it.
3. Red-first: T-1–T-9, T-14, T-15, T-15b, T-16, T-17, T-19, T-20, T-20b, U-T1–U-T3, U-T6, U-T7, T-E1 (and T-21, U-T8
   under O-7 (a)) committed failing; the fix makes them and §5.2's rewritten cells green; the guards (T-6b, T-8a, T-9b,
   T-18, U-T4, U-T5) and the residual pin (T-21, under O-7 (b)) are green at the head.
4. `/usr/bin/grep -rn 'resolve_handler_target\|is_body_element' crates` and `/usr/bin/grep -rn 'fn window_entity'
   crates/core/elidex-ecs` → 0 hits (`HostData::window_entity` and the `ScriptEngine` trait method stay);
   `determine_event_handler_target(` → the definition and the five call sites of §4.2; `is_x_w_pair(` → the definition
   and one call; `process_created_subtree(` → the definition and one call; `/usr/bin/grep -n
   'tag_matches_ascii_case(entity, "body")' crates/script/elidex-js/src/vm/host/elements.rs
   crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` → each hit paired with the namespace read (D-T11).
5. `cargo test -p elidex-script-session -p elidex-js -p elidex-ecs -p elidex-shell --all-features` green; `mise run ci`
   before push.
6. §8.1 / §8.3 texts handed to the PM in the PR description.
7. **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
   docs/plans/2026-10-eh-attrs-t3-target-determination.md` and the same on the umbrella exit 0.

## §10. Open questions

**Decided since rev 2:**
- **O-6** → PM decision 153 (user, 2026-10-31): order N1 → C0 → C1 → T3 → slice b; R-A closed by C1 (on C0), R-B by C0;
  R-C closed in T3 by moving S-1's clone path (D-T10); R-D considered in T3: R-D1 closed (D-T11), R-D2 back to the
  user (§2.2, O-7).
- **O-5** → confirmed by the PM (reading of PM decision 133): T3 keeps its own HTML-namespace "the body element"
  predicate; no tag-only test; `document.body` not touched (D-T5).

**Decided since rev 1:** O-1 → PM decision 129 (D-T7; §8.4); O-2 → PM decision 130 (D-T9; §8.1 item 1); O-3 → PM
decision 131 (§8.4); O-4 → PM decision 132 (D-T3).

**For the user (through the PM):**
- **O-7 — residual R-D2** (§2.2): (a) T3 takes the local-name half of the attribute change steps' step 1 (applicability
  facets (1) and (3); facet (2) stays with slice b), so §2.1 has 0 rows worse than the base; or (b) keep the ratified
  "closed by slice b", and FN-4 stays worse than the base at a dispatch on the element until slice b. Recommended: (a).

**Facts needed before plan-review (scout):**
- **Q-7** — at T3's base: does a non-bubbling `Event('focus')` / `Event('hashchange')` dispatched at an element run a
  non-capture handler on the Window (it must not; C19b's and P3's T3 expectations assume it does not)?
- **Q-8** — does `parseFromString('<frameset></frameset>', 'text/html')` produce a `frameset` as `dd.body` (T-6b)?
- **Q-10** — C0's plan (when drafted): does C0 adopt at the `EcsDom` tree-mutation level or at the DOM *insert*
  algorithm's callers (which ECS-level insertions, if any, bypass it), and which node-document derivation does it leave
  clause (b) (C0-4)? Fixes D-T4 (i) / (ii) and U-T1's constructions.
- **Q-11** — at T3's base: between `invoke_dom_api(ctx, "cloneNode", …)` returning and `native_node_clone_node`
  returning, can any script run (a custom-element reaction drained, a MutationObserver microtask)? D-T10 assumes not
  (H-T24).
- **Q-12** — does `for_each_uncapped_shadow_inclusive(root, TemplateContentsWalk::Include, …)` reach a clone's
  replicated template contents and clonable shadow trees, nested at any depth (U-T7 (ii), (iii))?
- **Q-13** — on C1's landed code: is the inert document I ever the composed tree root of a node in template contents
  (it should not be: I has no children unless a script appends some), so that the composed-root branch of the
  derivation cannot return I for a connected-looking node?
- **Q-14** — can the `body` arm in `tag_specific_t2b_prototype(&self)` read `EcsDom::is_html_namespace` without a new
  `HostData` method (the aliasing contract of `HostData`'s `dom_ptr` readers)? If not, the method's home and the
  touch-time split of `host_data/mod.rs` (2024 lines) become a standalone prereq PR before T3.

Answered since rev 2: **Q-9** (ledger row for foreign elements taking HTML interfaces): none; the master roadmap has
`M4-13:#svg` ("SVG 全 element family + SVGSVGElement", absent) (D-T11, §8.1 item 4). Rev 1's Q-1 to Q-6 are answered
(H-T15 to H-T20); Q-1's "unverified: `elidex-wasm-runtime` bind" stays open in H-T15 and does not affect T3 (no Wasm path
reaches the handler consumer at `1ed126b4`: `git grep -n 'EventHandlerAttributeConsumer' 1ed126b4 --
crates/script/elidex-wasm-runtime` → none).

No browser measurement is needed: every outcome in §6 is spec-defined, including FN-4's (MathML Core §2.1.4, H-T23).
