# C1-pre — every Document entity's wrapper is a Document (per-PR plan-memo)

Prerequisite PR **C1-pre** of C1 (`EH-attrs-at-creation-C1`, plan
`docs/plans/2026-10-eh-attrs-c1-template-owner-document.md`, rev 3.4, branch `layout-eh-c1-template-owner-plan`, not on
main; "the C1 memo") in the standalone program `EH-attrs-at-creation` (umbrella
`docs/plans/2026-10-event-handler-attrs-at-creation.md`, approved by #551 `b9122157`; "the umbrella"). It discharges
slot `#11-bridge-document-post-install` (ledger `project_open-defer-slots.md`; origin #157 `270680cc`). Siblings cited
by ID: the C0 program memo `docs/plans/2026-10-eh-attrs-c0-insertion-adoption.md` rev 3.6+ ("the C0 draft") and its
per-PR memo `docs/plans/2026-11-eh-attrs-c0c-adopt-notification-adoptnode.md` ("the C0c memo"), both on branch
`layout-dom-c0-insert-adopt-plan`, not on main; the inline-box predicate umbrella
`docs/plans/2026-09-inline-box-predicate.md` (on main; "the IBP umbrella").

**Placement (evidence).**
- PM decision 328 (relayed by the Layout lane driver, 2026-11-04; the option text was not shown to this memo's author):
  scope = every Document entity's JS wrapper gets the Document surface at the bridge, in one canonical place, whichever
  path first wraps it; per-entity Document facts are read from `this`'s entity, not from VM-global state; vm/host stays
  install / marshalling, any algorithm goes to the engine-independent side. The slot's trigger fired (C1 makes a new
  Document reachable through the generic wrapper path); its owner moves to the Layout lane. C1-pre lands before C1
  (edge c-pre, §8).
- **PM decision 335** (relayed, 2026-11-04; option text not shown): (1) gave C1-pre the per-entity document URL —
  **withdrawn by PM 339**; (2) bounds the scope: every per-entity Document fact the installed members read is
  enumerated by property, each with C1-pre's writer and reader or a slot (§5.1 P-11, §8 Records) — kept; (3) a URL leg
  for C1's T1 — withdrawn with (1) (R-2).
- **PM decision 339** (relayed, 2026-11-04; the user's decision (A); option text not shown): the document URL's home
  goes back to the `#11-document-base-url` program. Ground: #526 withdrew a `DocumentUrl` component of the same shape
  (IBP umbrella rev 61, Fork 1 (A)) because navigation rewrote it while the per-VM `current_url` also existed — two
  representations — against the user's 2026-06-11 ruling (`current_url` per-VM until B1) and d5's discriminator
  (a per-Document component holds a fact immutable per Document by spec, written only at Document creation; IBP
  umbrella §0.5). Rule for P-11: C1-pre keeps only facts written
  once, at Document creation; a fact that navigation, `pushState` or any later event rewrites goes to the program
  (URL family) or its own slot (§8 Records).
- The motivating findings are C1's plan-review R1 (frame `8ea74ae4`): aggregate F1 (I's wrapper is not a Document:
  Ax1 IMP, Ax2 IMP-2, Ax5 IMP-1) — C1-pre's — and F2 (`URL` ignores the receiver: Ax3 IMP, Ax1 MIN) — the program's
  under PM 339 (R-10).

**Scope.**
1. One `Document.prototype` interface prototype object (NEW) carrying today's Document member suite, chosen for every
   wrapper of a `NodeKind::Document` entity at the bridge's single allocation step, `create_element_wrapper` (D-1).
2. Removal of the three path-specific installs, the per-entity installer and its `HostData` side-store (D-2).
3. One browsing-context predicate for Document receivers, replacing six copies, and the members that must consult it
   but do not (D-4, D-5, D-6).
4. Each Document member except the URL pair answers from `this`: the receiver-ignoring natives `readyState` and
   `getSelection` are brand-checked and routed (D-5, D-7, D-8). Every per-entity Document fact a member reads is
   enumerated with its writer and reader, or a slot (§5.1 P-11).

**Not in scope.** The document URL — its store, every writer and every reader (`URL`, `documentURI`, `cookie`'s URL,
DOMParser's *relevantDocument's URL*, `location`, `history`, fetch / API base, origin, storage, workers):
`#11-document-base-url` (PM 339, R-10); `native_document_get_url` stays HEAD's, so the URL pair keeps ignoring `this`.
The `Document` interface object (no DOM node interface object exists in the VM, §7); the browsing-context cluster's
migration to ECS components (B1 / `#11-browsing-context-state-ecs-components`); new Document members (`characterSet`,
`implementation`, `location`, …); multi-realm (B1).

**Changelog.** *Rev 1* (2026-11-04): first draft, pre-plan-review; PM 335 applied before the first commit. *Rev 2*
(2026-11-04, PM 339 (user (A))): the URL part removed to `#11-document-base-url` (§10). *Rev 2.2* (2026-11-05):
plan-review R1 fixes (§10). *Rev 2.5* (2026-11-08, PM 534): re-measured at `43e350c7`; H-13 rewritten (C0c's
adopted-reaction flush), W22 added (§10).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`43e350c7`** coordinate (this branch's head: the plan commits merged with origin/main
`0a5a77ba`, which is `git merge-base HEAD origin/main`; `git diff --stat 0a5a77ba 43e350c7` touches only this memo, so
the code read is `0a5a77ba`'s). Rev 2.5 re-read every cite at `43e350c7` (2026-11-08). Since rev 2.4's frame
`f9cf8c40`: C0 D-14 (b) landed (#599 `e8b6a24d`), moving the accessors to `vm/host/document_accessors.rs` and the node
factories to `vm/host/document_factories.rs`; RC-2a (#596), C0b (#608) and C0c (#611 `0a5a77ba`) changed
`node_methods_extras.rs`, the ECS cloner and spawn, and `document_base.rs`, and C0c added `adoptNode` and the
adopted-reaction flush (H-13); #603 changed the focus writers (D-5). Symbols are cited by name; re-find lines
with `/usr/bin/grep -n '<symbol>' <file>`. Code reads are `git grep` / `/usr/bin/grep -n` / `sed -n`. Every § number ↔
title pair and every step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`, `idl`, `coverage-map`), run
2026-11-04, and rev 2.5's lookups for W22, run 2026-11-08; commands in §3.

### §0.2 Premises re-measured (the brief's claims, checked)

| id | claim | evidence (at `43e350c7`) |
|---|---|---|
| H-1 | The per-entity installer is reached from three paths | `VmInner::install_document_methods_for_entity` `crates/script/elidex-js/src/vm/host/document.rs:317`; direct callers `document.rs:301` (inside `Vm::install_document_methods_if_needed` `:290`, called by `install_document_global` `crates/script/elidex-js/src/vm/host/globals.rs:49`), `crates/script/elidex-js/src/vm/host/dom_parser.rs:305`, `install_document_methods_if_cloned_doc` `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:233-249` (call `:247`; called from `native_node_clone_node` `:218`). Command: `git grep -n 'install_document_methods' -- crates` (3 call paths, plus definitions, docs and two test comments) |
| H-2 | Document members are own properties of each wrapper; there is no `Document.prototype` | installer comment `document.rs:329-337`; the bridge's prototype dispatch `crates/script/elidex-js/src/vm/host/elements.rs:85-157` gives Document → `Node.prototype` (`:95-96`, arm `OtherNode` `:154-156`); `PrototypeKind` `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:18` has `DocumentType`, `DocumentFragment`, `ShadowRoot`, no `Document`. Document is the only DOM node kind whose wrapper gets no interface prototype object |
| H-3 | `ownerDocument` wraps through the generic path without install | `native_node_get_owner_document` `node_methods_extras.rs:42-51` → `invoke_dom_api(.., "ownerDocument.get", ..)` → phase 3 `create_element_wrapper(e)` `crates/script/elidex-js/src/vm/host/dom_bridge.rs:519-521`. (The brief's `dom_bridge.rs:75`, `wrap_entity_or_null`, is a sibling helper on the same allocator, not this getter's path) |
| H-4 | One allocator makes every DOM-node wrapper | `VmInner::create_element_wrapper` `elements.rs:74` (alloc `:158-164`, cache `:170-173`). Population P-1 (§5.1): 7 `HostObject` allocation sites, of which `elements.rs:159` is the only one for DOM node entities |
| H-5 | `URL` / `documentURI` ignore `this` | `native_document_get_url` `crates/script/elidex-js/src/vm/host/document_accessors.rs:118-129` (`_this` `:120`) returns `ctx.vm.navigation.current_url` (`:126`); module doc `:19-20` ("navigation state, not DOM") |
| H-6 | The dom-api has an unused `document.URL.get` stub | `GetDocumentUrl` `crates/dom/elidex-dom-api/src/char_data/document_props.rs:14-30` returns `"about:blank"`; registered `crates/dom/elidex-dom-api/src/registry.rs:91`; no VM caller (`git grep -n 'document\.URL\.get' -- crates`). **Also unused**: `document.readyState.get` (`GetReadyState` `document_props.rs:80-98`, reads `SessionCore::document_ready_state`, written by the shell `crates/shell/elidex-shell/src/pipeline.rs:488`) and `document.compatMode.get` (`GetCompatMode` `document_props.rs:101-117`); the VM answers both from its own constants (`document_accessors.rs:144-153`, `:187-203`) |
| H-7 | Six natives decide "has a browsing context" by comparing `this` with the bound document, each with its own copy | `/usr/bin/grep -n 'bound_doc != Some(doc)' crates/script/elidex-js/src/vm/host/document_accessors.rs` → `:232` defaultView, `:282` cookie get, `:327` cookie set, `:374` referrer, `:524` hidden, `:555` visibilityState (6; the same command on `document.rs` / `document_factories.rs` → 0) |
| H-8 | Two Document members read VM-global state without consulting `this` at all | `readyState` `document_accessors.rs:144-153` (`_this`, constant `"complete"`); `getSelection` = `super::window::native_window_get_selection` (table entry `document.rs:429`; `crates/script/elidex-js/src/vm/host/window.rs:803-816`, `_this`, the VM's Selection singleton). `hasFocus` `document_accessors.rs:479-497` reads `this`'s focus but gates on the VM-global `is_tab_hidden` (`:487-494`), not on `this`'s browsing context |
| H-9 | No DOM node interface object exists in the VM | the global-name population: `git grep -n '=> "\(Document\|Node\|Element\|DocumentFragment\|ShadowRoot\|HTMLDocument\)"' -- crates/script/elidex-js/src` → 0; `HTMLElement` is the only DOM interface name (`crates/script/elidex-js/src/vm/well_known.rs:1546`, custom elements). So `x instanceof Document` throws `ReferenceError` (code reading, not run) |
| H-10 | The VM's spec-level policy is fixed at construction | `spec_level_policy` set in `crates/script/elidex-js/src/vm/init.rs:177-185`; `installs` / `installs_dom` `crates/script/elidex-js/src/vm/globals_install.rs:163-175` read it. A once-per-VM install on a prototype therefore applies the same gates as today's per-wrapper install |
| H-11 | Prototype-level install of the same mixins has precedents | `install_parent_node_mixin` on `Element.prototype` (`crates/script/elidex-js/src/vm/host/element_proto.rs:108`) and `DocumentFragment.prototype` (`crates/script/elidex-js/src/vm/host/document_fragment_proto.rs:48`); `install_event_handler_attrs` on `HTMLElement.prototype` (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:134`) |
| H-12 | `document.rs`'s 1000-line split has landed | C0 D-14 (b) (reviewed, PM 258 / 318) landed as #599 `e8b6a24d`, a pure move: the node factories to `crates/script/elidex-js/src/vm/host/document_factories.rs` (162 lines) and the accessors to `crates/script/elidex-js/src/vm/host/document_accessors.rs` (590); `document.rs` keeps `document_receiver`, the lookups, C0c's `adoptNode` native, the installer and the tables (534 lines). `wc -l` of the three files |
| H-13 | Every script-reachable Document is installed before a generic-path wrap of it can run; the generic paths that wrap a Document entity include C0c's adopted-reaction flush (rev 2.4's list omitted it) | Installed: the bound document, DOMParser and `document.cloneNode` documents (H-1). `document.implementation` / `createHTMLDocument` do not exist in the VM (`git grep -n 'createHTMLDocument\|"implementation"' -- crates/script/elidex-js/src` → 0). `adoptNode` is a Document member since C0c (`native_document_adopt_node` `document.rs:253`, table entry `:432`; handler `AdoptNode` `crates/dom/elidex-dom-api/src/document.rs:318`, registered `registry.rs:79`); it throws for a Document argument and returns its argument (DOM §4.5 `adoptNode()` steps 1, 4), so it never returns a Document. **The flush**: for each custom element an *adopt* re-homes (DOM §4.5 *adopt* step 3.3.3, « oldDocument, document »), `handle_adopt` (in `impl CustomElementReactionConsumer`, `crates/dom/elidex-custom-elements/src/consumer.rs:201`) enqueues `CustomElementReaction::Adopted { old_document, new_document }` (`consumer.rs:243`), and `crates/script/elidex-js/src/vm/host/custom_elements/flush.rs:143-144` wraps both Documents through `create_element_wrapper` with no Document step, so a Document whose wrapper the flush allocates first has `Node.prototype` and no Document member at HEAD. At `43e350c7` that needs a Rust fixture (W22): the flush runs at reaction checkpoints (`git grep -n 'flush_ce_reactions()' -- crates/script/elidex-js/src`: `engine.rs:165`, `vm/interpreter.rs:54`, `vm/natives_timer.rs:281`, `custom_elements/define.rs:208`), the clone and DOMParser natives install before they return, and a template's contents stay in its own node document (`attach_template_contents(entity, dom.node_document_for_new_nodes(entity))`, `crates/dom/elidex-dom-api/src/document.rs:270`; `EcsDom::adopt`'s doc, "into the same document here"). C1's I is the first script-reachable one, by two paths: `ownerDocument`, and this flush (on C1's head `tpl.content`'s node document is I, so inserting a custom element into `tpl.content` adopts it into I, DOM §4.2.3 *insert* step 7.1, and runs `adoptedCallback(document, I)`) |
| H-19 | No Rust code reads a Document wrapper's own properties except the generic object model | `git grep -n 'storage\.get(' -- 'crates/script/elidex-js/src/vm/*.rs' \| /usr/bin/grep -v '/tests\|tests\.rs\|/tests_\|wasm'` → 7 at `43e350c7`, all generic: prototype walk `vm/coerce.rs:917`, `:973`; `vm/natives_object/descriptor.rs:245`, `:390`; delete `vm/ops_property.rs:364`; define `vm/shape_ops.rs:542`, `vm/dispatch_objects.rs:291`. So moving the members changes only the JS-visible own-property answers (§5.4 R-1, P-6). `natives_promise.rs:526` only allocates the wrapper |
| H-20 | The fragment-parse throwaway Document is never script-reachable | `EcsDom::begin_detached_fragment` (`crates/core/elidex-ecs/src/dom/tree/teardown.rs:286`, spawn `:295`) is called only by the two parser crates (`crates/dom/elidex-html-parser/src/lib.rs:143`, `crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs:180`; neither depends on `elidex-js`) and by `elidex-ecs` tests, with the mutation dispatcher taken (`teardown.rs:294`), and is torn down in the same call (`lib.rs:155`; `mod.rs:260`, or `:234` on the strict error path) |

### §0.3 Premise errors in the brief (reported)

- **P-1 (split).** The brief asks for a new "C1-pre0" split of `vm/host/document.rs`. That file's split is C0 D-14 (b)
  (H-12), already reviewed; a second seam for the same file would leave two split decisions for one file. C1-pre
  takes D-14 (b) as its code predecessor and adds no seam to `document.rs` (D-10). The split of the other file it
  touches, `vm/globals.rs`, is C1-pre0 (D-11).
- **P-2 (URL ownership), resolved by PM 339 (user (A)).** The document URL is a facet of `#11-document-base-url`
  (IBP umbrella §0.6 item 18; `docs/plans/2026-09-inline-box-predicate.md:1659`: "either home answer for the document
  URL is the user's to ratify"); the user's ruling that `current_url` stays per-VM until B1 is of 2026-06-11, and the
  2026-09-28 update (option (iv), memory `project_browsing-context-state-ecs-components.md`) moved only the sandboxing
  flags and restated "current_url stay per-VM until B1 as ruled" (rev 1 dated the ruling 2026-09-28: wrong). PM 339
  returns the URL to the program; C1-pre writes and reads no document URL (R-10).
- **P-3 (paths).** `document.implementation.createHTMLDocument` does not exist; `adoptNode` returns a Document never;
  `cloneNode` of a Document is one of the three install sites already (H-13). The populations that matter are
  defined by property in §5.1.
- **P-4 (slot remedy).** The slot's remedy direction ("a step of the bridge layer"; review readings: "post-install keyed
  on `NodeKind::Document`") was checked against the code: the bridge already has the step — prototype selection by node
  kind (H-2) — and Document is the only node kind without an interface prototype object. A post-install keeps own
  properties per wrapper and the `document_methods_installed` side-store. D-1 adopts the remedy in its WebIDL form.
- **P-5 (C1's T1).** C1's T1 second leg `I instanceof Document` throws `ReferenceError` on any head (H-9); hand-off R-2.
- **P-6 (owner).** The ledger entry still says "Owner: core VM (driver absent)"; PM 328 moves it to Layout (record R-1).
- **P-7 (C1's H-C10).** C1's premise "A Document entity is spawned in one place" misses the cloner's spawn. The two
  sites: `EcsDom::create_document_node` `crates/core/elidex-ecs/src/dom/mod.rs:474-480`, which spawns `TreeRelation`,
  `NodeKind::Document` and `DocumentBaseUrl(about_blank_url())` (unchanged since `f9cf8c40`), and, for a Document
  source, `clone_node_shallow_unchecked` `crates/core/elidex-ecs/src/dom/tree_clone.rs:253-371` (spawn `:370`), whose
  copy carries no `DocumentBaseUrl` (`tree_clone.rs:31`: out-of-domain) and no `AssociatedDocument` stamp (a Document is
  its own node document, `:193-202`). It no longer matters to C1-pre (no per-spawn write); hand-off R-2 only.

### §0.4 Decisions (this PR)

- **D-1 — `Document.prototype` at the bridge.** `VmInner::document_prototype: Option<ObjectId>` (NEW) is allocated once
  per VM by `register_document_prototype` (NEW), with `[[Prototype]]` = `Node.prototype` (Web IDL §3.7.3 step 3:
  Document inherits Node). `PrototypeKind::Document` (NEW) is returned by `HostData::prototype_kind_for` for
  `NodeKind::Document`, and `create_element_wrapper` maps it to `document_prototype` (Web IDL §3.8 *internally create a
  new object implementing the interface*: prototype = the interface prototype object). This is the one canonical place:
  every path that wraps a Document entity — the bound global, DOMParser, `cloneNode`, `ownerDocument`, `parentNode`,
  `getRootNode`, event paths, Range / TreeWalker containers, `natives_promise.rs:526`, C1's I, C0c's `adoptNode`, C0c's
  adopted-reaction flush (`adoptedCallback`'s oldDocument / newDocument, `custom_elements/flush.rs:143-144`, H-13) —
  gets it by construction, whichever wraps first. Rejected: (a) the slot's post-install at the bridge (own properties per
  wrapper, contrary to Web IDL §3.7.6 / §3.7.7 "regular attributes / operations are exposed on the interface prototype
  object"; keeps the side-store; installs ~70 properties per Document wrapper); (b) a fourth file-local hook.
- **D-2 — members and their order.** `Document.prototype` receives, in today's order, the tables
  `DOCUMENT_METHODS_PRE_LIVE_COLLECTION`, `DOCUMENT_LIVE_COLLECTION_METHODS` (gated by `installs_dom(live_collection_spec_level())`),
  `DOCUMENT_METHODS_POST_LIVE_COLLECTION` (whose last entry is C0c's `adoptNode`, `document.rs:432`),
  `document_traversal::FACTORIES`, `DOCUMENT_RO_ACCESSORS`,
  `DOCUMENT_RW_ACCESSORS`, `DOCUMENT_COOKIE_RW_ACCESSOR` (`compat-webapi` + `installs(document_cookie_spec_level())`),
  the ParentNode mixin and the event-handler IDL attributes (`Global`, `DocumentElement`, `DocumentOnly` scopes)
  (`document.rs:329-387`). The A1 contract "install order = enumeration order" (Codex R9) moves to
  `Object.getOwnPropertyNames(Object.getPrototypeOf(document))` (cell W3). No Document member remains an own property;
  `[LegacyUnforgeable] location` would be the one own property and is not implemented (§7).
- **D-3 — identity unchanged.** One wrapper per Document entity per VM through `HostData`'s wrapper cache (CLAUDE.md
  side-store rule (a), interim; no migration). The prototype is fixed at first allocation; a wrapper allocated while
  unbound gets `Node.prototype` (`prototype_kind_for` returns `OtherNode` when unbound,
  `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:145-147`), the fallback every node kind
  shares (named risk R-4).
- **D-4 — one browsing-context predicate.** `document_has_browsing_context(ctx, doc) -> bool` (NEW, in
  `vm/host/document_accessors.rs` after D-14 (b)) = "`doc` is the bound document" (`HostData::document`). It is host
  marshalling of the VM's single browsing-context ↔ Document association, stated once; in this VM a Document's node
  navigable (HTML §7.3.1) is non-null exactly when it is the bound document. Its six copies (H-7) become
  calls; its storage home stays per-VM until B1 (`#11-browsing-context-state-ecs-components`).
- **D-5 — members that must consult it.** `hasFocus` gains the has-focus steps' step 1 (no navigable → `false`) before
  the tab-hidden check (HTML §6.6.4); Document `getSelection` gets its own native `native_document_get_selection`
  (NEW): brand-check `this`, no browsing context → `null` (Selection API §4.1 returns `Selection?`), else the Window's
  selection (today's behaviour). `getSelection` on Window is unchanged. `hasFocus`'s step 1 is spec alignment with no
  script-reachable change: no script path focuses an element of a non-bound Document (`set_focus_bit`'s callers gate
  on bound-document membership, the contract stated at `crates/dom/elidex-dom-api/src/focus/sot.rs:100-103`), so W15
  sets the state from Rust. Re-checked at `43e350c7` (`git grep -n 'set_focus_bit(' -- crates`, non-test callers):
  the two that set the bit gate on `is_in_document(dom, _, doc)` with the bound document — VM `HTMLElement.focus()`
  (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:302-305`) and `focus_update_steps`'s step 4.1
  (`crates/dom/elidex-dom-api/src/focus/update_steps.rs:303`), whose only caller with a `Some` target is
  `focusing_steps`, gated at `:142` (the shell sink's document is the pipeline's, `elidex-shell/src/content/focus.rs:128-129`);
  the rest only clear the bit (`sot.rs:147` `blur`; `update_steps.rs:287`, also reached from #603's
  `focus_fixup_steps` through `focusing_steps_for_viewport`, which passes no target).
- **D-6 — the no-browsing-context values are the spec's.** `defaultView` `null`, `cookie` `""` / no-op set
  (cookie-averse), `referrer` `""`, `hasFocus()` `false`, `getSelection()` `null`, `hidden` `true` and `visibilityState`
  `"hidden"` (HTML §6.2: "visibility state … initially set to "hidden""; two §6.2 algorithms change it, *set the initial
  visibility state*, which *initialize the navigable* (HTML §7.3.1) step 6 calls only for a navigable's document, and
  *update the visibility state*; neither runs for a Document without a browsing context, so its value stays the
  initial "hidden").
  Three of these change an observable value of today's DOMParser and cloned Documents (`hidden`, `visibilityState`,
  `getSelection`; cell W6).
- **D-7 — `compatMode` reads the dom-api handler.** The VM native routes through `invoke_document_accessor(..,
  "document.compatMode.get", "")` and its constant is deleted (values identical; one answer instead of two, H-6).
- **D-8 — `readyState` is brand-checked; its value source is not changed.** It brand-checks `this` through
  `document_receiver`. Its value stays `"complete"` for every Document: the spec default for a Document without a
  browsing context (HTML §3.1.5) and today's answer for the bound one. The bound document's readiness has two answers
  today (H-6); routing it to `GetReadyState` changes the bound document's observable readiness during load — slot
  proposal R-4.
- **D-10 — `document.rs` split = C0 D-14 (b).** No new seam (§0.3 P-1). D-14 (b) landed as #599 (H-12) before C0c
  and C1-pre (edge e-14b).
- **D-11 — C1-pre0 (`vm/globals.rs` pure move): landed as #592 `f9cf8c40`**; follow-up
  `#11-vm-globals-registration-structure`; C1-pre adds one call and does not fire that slot's trigger (no new phase or
  install loop; `vm/globals.rs` stays < 1000 unless `vm-p4-rooting-carved` lands first, §5.3; PM 371). `vm/mod.rs`
  (2785) gains one field and is
  **not** split: it is one struct definition (`VmInner`, `:184-2773`), and its only seam, grouping fields into
  sub-structs, is a non-pure-move refactor deferred in memory `vm-inner-substruct-deferral`.
- **D-12 — Layering / ECS-native.** vm/host keeps prototype install, brand check and marshalling only (D-4 consolidates
  an existing association check; D-7 removes a host answer in favour of the dom-api handler; `hasFocus`'s steps stay
  in host because their inputs, the predicate and `is_tab_hidden`, are per-VM host state). The per-entity
  side-store `HostData::document_methods_installed` (`HashSet<Entity>`, `crates/script/elidex-js/src/vm/host_data/mod.rs:106-120`)
  is deleted, not migrated: it records a per-VM fact about a wrapper, which the prototype makes unnecessary. Idiom
  table: OO "class Document owns its methods" → Web IDL interface prototype object (JS side), discriminated by the
  existing `NodeKind::Document` component. C1-pre adds no per-Document component (§5.1 P-11).

## §1. Shape and verdict

**Axes (§2): 6** — B1 wrapper shape, B2 identity, B3 member surface and gates, B4 per-entity facts, B5 bind lifecycle,
B6 consumers (DOMParser, clone, C0c, C1). Rev 1's B7 (the Document URL fact) left with PM 339. ≥ 3 intersecting axes →
edge-dense: plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). It is a per-PR slice for the approved EH-attrs
umbrella, whose §1.2 row its own PR adds (R-5), so a terminal unit after plan-review (base case (c)).

**Split decision (re-run for rev 2): three units, unchanged.** C1-pre0 (`vm/globals.rs` pure move, D-11, landed) →
C1-pre (this memo), with C0 D-14 (b) (`document.rs` pure move, D-10) also before C1-pre, still needed after the
removal: C1-pre still touches `vm/host/document.rs` (the installer, the class-B natives, `hasFocus`, `getSelection`,
`compatMode`, `readyState`; 1197 lines at `f9cf8c40`). Both predecessors have landed (#592, #599): at `43e350c7` the
installer and tables are in `document.rs` (534 lines) and the natives in `document_accessors.rs` (590). D-1 to D-8 and D-10 to D-12 stay one PR: the prototype without D-4 to D-8
would make I's surface reachable with the receiver-ignoring answers PM 328 rejects; D-4 to D-8 could land alone first, but they are
the receiver half of the same member suite. Rev 1's second commit
(the URL work) is gone. Preflight: SPLIT-RECOMMENDED (K = 4, M = 15; rev 1: K = 4, M = 19) — K counts the Selection
API row, a single member (`getSelection`), and M's rows are the spec homes of the one member suite, so a further split
would separate a member from its prototype rather than shrink scope.

**K-0 lens (no step worse than HEAD).** Every changed outcome moves from HEAD's value to the spec's: own-property
enumeration of `document` (W2), brand checks (W9, W10), non-bound Documents' `hidden` / `visibilityState` /
`getSelection` (W6), and every member of a generic-path Document (`ownerDocument` or the adopted-reaction flush;
reachable at `43e350c7` only through a Rust fixture, H-13; W1, W22). `URL` / `documentURI` keep HEAD's answer (the
page's current URL) for every Document; a generic-path Document goes from `undefined`
to that answer, not to its own URL — R-10's defect, not a C1-pre step.

### §1.1 Expected values

Spec column from the cited steps; HEAD column from code reading at `fab44a1d`, re-read at `43e350c7` (not run).
`dp = new DOMParser().parseFromString('<p></p>', 'text/html')`; `tpl = document.createElement('template')`.

| id | expression | spec (derivation) | elidex HEAD | elidex C1-pre head |
|---|---|---|---|---|
| X1 | `Object.getOwnPropertyNames(document)` | `["location"]` (§3.7.6 / §3.7.7; `location` is `[LegacyUnforgeable]`) | every installed member | `[]` (`location` absent, §7) |
| X2 | `Object.getPrototypeOf(dp) === Object.getPrototypeOf(document)` | `true` (both Document) | `true` (both `Node.prototype`) | `true` |
| X3 | `Object.getPrototypeOf(document).hasOwnProperty('createElement')` | `true` (HTML §7.2.2: `HTMLDocument` is the Document interface object, so no intermediate prototype) | `false` | `true` |
| X4 | `[dp.hidden, dp.visibilityState]` | `[true, "hidden"]` (§6.2 initial value) | `[false, "visible"]` | `[true, "hidden"]` |
| X5 | `dp.getSelection()` | `null` (Selection API §4.1; prose not extractable by `webref body` on 2026-11-04, see §3) | a `Selection` | `null` |
| X6 | `[dp.defaultView, dp.cookie, dp.referrer, dp.hasFocus(), dp.readyState, dp.compatMode]` | `[null, "", "", false, "complete", "CSS1Compat"]` | same | same |
| X7 | `dp.activeElement === dp.body` | spec-silent for a Document without a navigable (§6.6.6 step 1 reads "this's node document's focused area"); current behaviour kept | `true` | `true` (unchanged) |
| X10 | `I = tpl.content.ownerDocument; [typeof I.createElement, I.defaultView, I.hidden, I.getSelection(), I.activeElement]` | `["function", null, true, null, null]` (C1 memo M3; §6.6.6 steps 4–7) | n/a (I is `document`) | C1's head: the spec column |

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| B1 shape | Every wrapper of a `NodeKind::Document` entity in a VM has `[[Prototype]]` = that VM's `Document.prototype`, chosen in `create_element_wrapper`; no other code decides a Document wrapper's shape |
| B2 identity | One wrapper per Document entity per VM (wrapper cache); the prototype choice never allocates or replaces a wrapper |
| B3 surface | `Document.prototype` carries exactly HEAD's member names in HEAD's order under HEAD's gates; no Document member is an own property of a wrapper; brand checks keep `document_receiver`'s contract (unbound / non-platform → fallback, wrong kind → `TypeError`) |
| B4 facts | Each member except `URL` / `documentURI` answers from `this`: tree facts through dom-api with `this`; browsing-context facts through the one predicate (D-4); every other fact per §5.1 P-11. The URL pair stays HEAD's (`#11-document-base-url`, R-10) |
| B5 lifecycle | Bind / unbind / rebind change no wrapper's prototype and run no per-bind install; a newly bound document's wrapper gets the same prototype; no per-entity install state exists |
| B6 consumers | DOMParser, `cloneNode`, C0c's `adoptNode` and adopted-reaction flush, C1's I carry no path-specific Document step; C1's T5, T1's third leg and M3's non-URL legs hold on C1's head (the URL legs are R-10's, R-2) |

### §2.2 All pairs (15), with intersection

| pair | intersect? | intersection |
|---|---|---|
| B1×B2 | yes | The prototype is chosen at first allocation and kept by every cache hit; a Document wrapper allocated while unbound would keep `Node.prototype` forever (R-4) |
| B1×B3 | yes | Members live on the prototype, so every Document wrapper has them; order is the prototype's (W2, W3) |
| B1×B4 | yes | The same function objects serve every Document; answers differ only through `this` (W1, W6) |
| B1×B5 | yes | A rebind to a new document allocates its wrapper with the same prototype; `install_document_global` no longer installs (W11) |
| B1×B6 | yes | DOMParser / clone / `ownerDocument` / C0c's flush / I all reach `create_element_wrapper`; P-1's DoD grep (W1, W5, W22) |
| B2×B3 | yes | Members returning objects keep today's identity behaviour (`forms` allocates per call, a pre-existing `[SameObject]` defect, §7); the prototype adds no identity |
| B2×B4 | yes | Two Documents have two wrappers whose answers differ (`dp.body !== document.body`; W1) |
| B2×B5 | yes | The wrapper cache is the only per-entity VM state left for Documents once `document_methods_installed` is gone (W11) |
| B2×B6 | yes | `p.ownerDocument === p.ownerDocument`; `dp.documentElement.parentNode === dp` (W4) |
| B3×B4 | yes | `readyState` / `getSelection` gain the brand check the rest have (W9, W10); `URL` / `documentURI` do not (R-10) |
| B3×B5 | yes | Gates are evaluated once at registration; the policy is fixed at VM construction (H-10), so the gated set equals today's per-wrapper set (W12) |
| B3×B6 | yes | C0c appended `adoptNode` to `DOCUMENT_METHODS_POST_LIVE_COLLECTION` (landed, #611); under D-1 it moves to the prototype with the rest (D-2, W3; edge e-c0c) |
| B4×B5 | yes | The predicate reads `HostData::document` at call time; after a rebind the old document reads "no browsing context" (unchanged from HEAD's copies) |
| B4×B6 | yes | DOMParser and cloned Documents: `hidden` / `visibilityState` / `getSelection` change (W6); I gets the same answers (C1's T1, M3) |
| B5×B6 | yes | A DOMParser document's wrapper survives unbind with its prototype; no re-install is needed on rebind (W11 variant) |

Count: 15 pairs; 15 intersect (rev 1's one non-intersecting pair, B2×B7, left with B7). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| B[0-9]×B[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-11-04): `.claude/tools/webref coverage-map webidl 3.7.3 webidl 3.8 webidl 3.7.6 webidl 3.7.7 dom 4.5 dom 4.4
html 7.2.2 html 7.3.2 html 3.1.4 html 3.1.5 html 6.2 html 6.6.4 html 6.6.6 html 4.12.3 selection-api 4.1` (skeleton);
`heading` for each pair; `idl dom Document`, `idl html Document` (`[SameObject]` collections, `location`
`[LegacyUnforgeable]`), `idl selection-api Document` (`Selection? getSelection()`); step text from `body webidl
dfn-interface-prototype-object`, `body webidl js-attributes`, `body webidl js-operations`, `body webidl
internally-create-a-new-object-implementing-the-interface`, `body dom concept-document-url` (the mode sentence), `body
html dom-document-defaultview`, `body html dom-document-referrer`, `body html the-document's-referrer`, `body html
dom-document-cookie`, `body html current-document-readiness`, `body html visibility-state`, `body html
has-focus-steps`, `body html dom-documentorshadowroot-activeelement`, `body html htmldocument`, `body html
appropriate-template-contents-owner-document`. `body selection-api dom-document-getselection` failed ("anchor … not
encountered"; possible multipage drift); the §4.1 row rests on the IDL and the dfn only. R-10's cites (run
2026-11-04): `heading dom 4.5`, `heading html 2.4.3`, `body dom concept-document-url`, `body dom
dom-domimplementation-createhtmldocument`, `body dom clone-a-single-node`, `body html dom-domparser-parsefromstring`,
`body html document-base-url`, `body html fallback-base-url`. Rev 2.5's lookups (run 2026-11-08), for H-13 and W22:
`dfn html hidden` (→ §6.2 Page visibility, `#dom-document-hidden`), `dfn html visibilityState` (→ §6.2,
`#dom-document-visibilitystate`), `dfn selection-api getSelection` (→ §4.1 Extensions to Document interface,
`#dom-document-getselection`), `heading html 6.2`, `heading selection-api 4.1`, `heading dom 4.5`; step text from
`body html dom-document-hidden` ("return true if this's visibility state is "hidden", otherwise false"), `body html
dom-document-visibilitystate` ("return this's visibility state"), `body html visibility-state` ("initially set to
"hidden""); rev 2.5.1's, for D-6: `dfn html "set the initial visibility state"` (→ §6.2 Page visibility,
`#set-the-initial-visibility-state`), `dfn html "update the visibility state"` (→ §6.2, `#update-the-visibility-state`),
`dfn html "initialize the navigable"` (→ §7.3.1 Navigables, `#initialize-the-navigable`), `heading html 7.3.1`, `body
html set-the-initial-visibility-state` (step 1 "Set document's visibility state to visibilityState"), `body html
initialize-the-navigable` (step 6 "Set the initial visibility state of documentState's document to …"); `idl selection-api Document` (`Selection? getSelection()`); `body selection-api
dom-document-getselection` and `body selection-api extensions-to-document-interface` failed again ("anchor … not
encountered"), so W22's `getSelection` leg, like X5, rests on the IDL and the dfn. For the flush: `dfn dom adopt` (→
§4.5 Interface Document, `#concept-node-adopt`), `body dom concept-node-adopt` (step 3.3.3: enqueue
`adoptedCallback` with « oldDocument, document »), `body dom dom-document-adoptnode` (steps 1–4), `dfn dom insert` (→
§4.2.3 Mutation algorithms), `body dom concept-node-insert` (step 7.1 "Adopt node into parent's node document").

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Web IDL §3.7.3 Interface prototype object | creation steps 1–3 | 3: interface inherits → proto = inherited interface's prototype object (Node) | `register_document_prototype` (NEW) in `crates/script/elidex-js/src/vm/host/document.rs` (D-1) | ✓ | no |
| Web IDL §3.8 Platform objects implementing interfaces | *internally create a new object implementing the interface*: prototype step | prototype = interface prototype object for the primary interface in realm | `create_element_wrapper` `crates/script/elidex-js/src/vm/host/elements.rs:74` via `PrototypeKind::Document` (NEW) `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:169` | ✓ | no |
| Web IDL §3.7.6 Attributes | regular attributes on the interface prototype object | unforgeable / `[Global]` → on the instance (Document: `location` only) | `DOCUMENT_RO_ACCESSORS`, `DOCUMENT_RW_ACCESSORS`, `DOCUMENT_COOKIE_RW_ACCESSOR`, event-handler attributes → prototype (D-2) | ✗ (`location` not implemented, §7) | no |
| Web IDL §3.7.7 Operations | regular operations on the interface prototype object | unforgeable / `[Global]` → instance (none for Document) | method tables, `document_traversal::FACTORIES`, ParentNode mixin → prototype (D-2) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | `compatMode` getter steps | quirks → "BackCompat", else "CSS1Compat" | `GetCompatMode` `crates/dom/elidex-dom-api/src/char_data/document_props.rs:101` via `invoke_document_accessor` (D-7) | ✗ (mode not modelled, §5.1 P-11, R-9) | no |
| WHATWG DOM §4.4 Interface Node | `cloneNode` / `ownerDocument` returning a Document | Document copy / owner | `native_node_clone_node` `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:206` (hook removed, D-2); the copy's wrapper and `ownerDocument`'s through `create_element_wrapper` (D-1) | ✓ | yes |
| WHATWG HTML §7.2.2 The Window object | `defaultView` getter step 1; `HTMLDocument` = the Document interface object | browsing context null → null | `native_document_get_default_view` `crates/script/elidex-js/src/vm/host/document_accessors.rs:217` via the predicate (D-4) | ✓ | no |
| WHATWG HTML §7.3.2 Browsing contexts | a Document's browsing context | bound document ↔ the VM's browsing context; any other → null | `document_has_browsing_context` (NEW) (D-4) | ✓ | no |
| WHATWG HTML §3.1.4 Resource metadata management | cookie-averse Document; `cookie` get / set; `referrer` | no browsing context → cookie-averse; the cookie URL stays HEAD's `current_url` (R-10) | `document_accessors.rs:262`, `:310`, `:356` via the predicate (D-4, D-6) | ✓ | yes |
| WHATWG HTML §3.1.5 Reporting document loading status | current document readiness, initially "complete"; `readyState` getter | Documents without a browsing context keep the default | `native_document_get_ready_state` `document_accessors.rs:144` (brand check, D-8) | ✗ (bound document's readiness, R-4) | no |
| WHATWG HTML §6.2 Page visibility | visibility state initially "hidden"; `hidden` / `visibilityState` getters | no browsing context → the initial "hidden" | `document_accessors.rs:506`, `:540` via the predicate (D-6) | ✓ | no |
| WHATWG HTML §6.6.4 Processing model | *has focus steps* 1–2 | 1 node navigable null → false; 2 no system focus → false | `native_document_has_focus` `document_accessors.rs:479` (D-5) | ✓ | no |
| WHATWG HTML §6.6.6 Focus management APIs | `activeElement` steps 1–7; `hasFocus()` | candidate a Document → body, else document element, else null | `native_document_get_active_element` `document_accessors.rs:427` (unchanged; X7) | ✗ (step 1 for no navigable is spec-silent) | no |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document* 1.1.1 (browsing context null) | I has no browsing context | none in C1-pre (I's wrapper gets D-1; its no-browsing-context answers come from D-4 / D-6) | ✓ | no |
| Selection API §4.1 Extensions to Document interface | `Selection? getSelection()` | no browsing context → null | `native_document_get_selection` (NEW) in `vm/host/document_accessors.rs` (D-5) | ✗ (rests on the IDL `Selection?` only) | no |

## §4. Mechanism

### §4.1 The prototype (D-1, D-2)

1. `vm/mod.rs`: field `document_prototype: Option<ObjectId>` (NEW) beside `document_fragment_prototype` (`:509`);
   `vm/init.rs`: its `None` initializer beside `:302`.
2. `vm/host/document.rs` (where D-14 (b) left the installer and tables): `install_document_methods_for_entity`
   becomes `VmInner::register_document_prototype` (NEW): allocate an ordinary object with `prototype: node_prototype`,
   run the install sequence of `document.rs:329-387` against it unchanged, store it. `Vm::install_document_methods_if_needed`
   is deleted.
3. `vm/globals.rs`: one call in `register_globals`'s Window branch (the scope fork, `:533`; a worker VM binds no
   document). The install sequence needs only `Function.prototype` (`register_prototypes` `:118`) and `node_prototype`
   (`register_node_prototype` `:175`): the `FACTORIES` natives look up `range_prototype` etc. at call time
   (`crates/script/elidex-js/src/vm/host/document_traversal.rs:77-80`), and the gates read `spec_level_policy`, set
   before `register_globals` runs (`vm/init.rs:172-185`).
4. `entity_kind.rs`: `Some(NodeKind::Document) => PrototypeKind::Document` in `prototype_kind_for`'s `match`
   (`:169`); `PrototypeKind::Document` (NEW) documented like its siblings.
5. `elements.rs`: arm `PrototypeKind::Document => self.document_prototype.expect(..)`, with no `.or(node_prototype)`
   fallback (§5.4 R-5); the comment `:95-96` loses "Document".
6. Deleted: `install_document_methods_if_cloned_doc` and its call (`node_methods_extras.rs:218`, `:222-249`), the
   DOMParser install (`dom_parser.rs:303-305`), the installer call in `install_document_global` (`globals.rs` host
   `:45-49`), `HostData::document_methods_installed` and its initializer (`host_data/mod.rs:106-120`, `:617`). Doc
   comments naming them: `node_methods_extras.rs:19-21`, `:202-205`, `dom_parser.rs:32`, `host_data/mod.rs:83`,
   `crates/script/elidex-js/src/vm/host/media_query.rs:124` (cites the set as a precedent; re-pointed), the slot name in
   `node_methods_extras.rs:230-232`. Stale stub / defer markers on touched members, rewritten:
   `document_accessors.rs:21-22`, `:150-151` (`readyState` "PR6" → slot proposal R-4), `:156` ("PR4f C6"), `:249`
   ("stubs"), `document.rs:489-490` ("PR4f C7 stubs", PR6 / PR5b); the `hidden` / `visibilityState` comments calling
   a non-bound Document "visible" (`document_accessors.rs:502-505`, `:511-515`, `:545-546`); the installer comments
   "no shared `Document.prototype`" (`document.rs:329-337`, `:372-378`).

### §4.2 Per-entity facts (D-4 to D-8)

| member | HEAD source | C1-pre |
|---|---|---|
| `defaultView`, `cookie` (get, set), `referrer`, `hidden`, `visibilityState` | own copy of `bound_doc != Some(doc)` (H-7) | `document_has_browsing_context` (D-4); `hidden` / `visibilityState` no-BC values → `true` / `"hidden"` (D-6); `cookie`'s URL stays `current_url` (R-10) |
| `hasFocus` | tab-hidden (VM-global) then `this`'s focus | predicate first (step 1), then tab-hidden, then `this`'s focus (D-5) |
| `getSelection` | Window's native, `_this` | `native_document_get_selection` (NEW): brand check, predicate → `null`, else the Window's selection (D-5) |
| `compatMode` | VM constant after brand check | `GetCompatMode` through `invoke_document_accessor` (D-7) |
| `readyState` | `_this`, constant | brand check, constant (D-8; R-4) |
| `URL`, `documentURI` | `_this`, `current_url` | unchanged (moves to `Document.prototype` with the rest; `#11-document-base-url`, R-10) |
| the other 29 members (§5.1 P-4, class T) | dom-api / engine-indep with `this` | unchanged |

Natives of class B live in `vm/host/document_accessors.rs` (since D-14 (b), #599), with the predicate.

### §4.3 What C1 gets

On C1's base (C1-pre landed): `tpl.content.ownerDocument` (I, allocated by `ownerDocument`'s generic path, spawned by
`create_document_node`) has `Document.prototype`, so C1's T5, T1's third leg and M3's non-URL legs read the spec values
with no C1 code. I's `URL` / `documentURI` read the page's URL until `#11-document-base-url` lands (R-10), so C1's T1
fourth and fifth legs and M3's `URL` / `documentURI` legs are that program's, not C1-pre's (R-2). C1's §4.1 / §4.8
statement "the VM touches none of it" becomes true for the wrapper; R-C5's precedent text becomes "every Document
wrapper has `Document.prototype` (C1-pre D-1)" (R-2).

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

`NT='/tests/\|tests\.rs\|/tests_\|test_helpers'`.
- **P-1 wrapper allocations** (property: a VM object whose kind is `HostObject` is created):
  `git grep -nE '(kind: |\.kind = )ObjectKind::HostObject' -- 'crates/script/elidex-js/src/*.rs' | /usr/bin/grep -v "$NT"`
  → 7 at `43e350c7`: `vm/globals.rs:68` (globalThis), `vm/host/canvas/mod.rs:334`, `:829` (2D contexts via
  `intern_wrapper`), `vm/host/elements.rs:159` (DOM nodes), `vm/host/offscreen_canvas/mod.rs:365`, `:405`,
  `vm/host/worker.rs:395`. Only `elements.rs:159` wraps DOM node entities, so it is the only Document wrapper
  allocator. One-time DoD grep (§9 item 2): the count of DOM-node allocators is 1.
- **P-2 path-specific installs**: `git grep -n 'install_document_methods\|document_methods_installed' -- crates` →
  0 hits at C1-pre's head.
- **P-3 Document wrapper first-wrap paths** (callers of the allocator that can receive a Document entity):
  `git grep -n 'create_element_wrapper(\|wrap_entity_or_null(\|wrap_entities_as_array(' -- 'crates/script/elidex-js/src/*.rs' | /usr/bin/grep -v "$NT"`
  plus `invoke_dom_api`'s phase 3 → 93 lines at `43e350c7` (phase 3, `dom_bridge.rs:521`, is one of them). Not enumerated one by one: D-1 makes the allocator decide,
  so the population's size is not load-bearing; its use is that none of them may add a Document step (P-2's DoD grep).
  Three members always receive a Document entity: the bound global (`host/globals.rs:44`), DOMParser
  (`dom_parser.rs:303`) and, since C0c, the adopted-reaction flush (`custom_elements/flush.rs:143-144`, oldDocument /
  newDocument; H-13, W22).
- **P-4 Document members by fact source** (property: the member's native and what it reads). Source: the tables
  `document.rs:402-534` plus `document_traversal::FACTORIES` (`crates/script/elidex-js/src/vm/host/document_traversal.rs:235-239`);
  for each, `/usr/bin/grep -n 'fn <native>' -A12` over `vm/host/document*.rs`. Count (one hit per table entry, whether
  the entry is on one line or split across lines):
  `sed -n '/^const DOCUMENT_METHODS_PRE_LIVE_COLLECTION/,$p' crates/script/elidex-js/src/vm/host/document.rs | /usr/bin/grep -cE '^\s*\(?"[A-Za-z]+",'`
  plus `sed -n '/const FACTORIES/,/^];/p' crates/script/elidex-js/src/vm/host/document_traversal.rs | /usr/bin/grep -cE '^\s*\(?"[A-Za-z]+",'`
  → 37 + 3 = **40** table entries at `43e350c7` (the same command at `f9cf8c40`, `git show f9cf8c40:<path> | …`, gives
  36 + 3 = 39, rev 2.4's count; the one more is C0c's `adoptNode`):
  class **T** (tree facts of `this`, 29): `getElementById`, `querySelector`, `querySelectorAll`,
  `getElementsByTagName`, `getElementsByClassName`, `getElementsByName`, `createElement`, `createTextNode`,
  `createComment`, `createDocumentFragment`, `createRange`, `createTreeWalker`, `createNodeIterator`,
  `documentElement`, `head`, `body`, `firstElementChild`, `lastElementChild`, `children`, `childElementCount`,
  `baseURI`, `doctype`, `forms`, `images`, `links`, `activeElement`, `styleSheets`, `title`, `adoptNode` (adopts its
  argument into `this`);
  class **B** (browsing-context facts, 6): `defaultView`, `cookie`, `referrer`, `hidden`, `visibilityState`,
  `hasFocus`; class **G** (receiver ignored or constant, 5): `URL`, `documentURI`, `readyState`, `getSelection`,
  `compatMode`. The ParentNode mixin and the event-handler attributes are class T (they act on `this`).
- **P-5 receiver-ignoring natives**: `/usr/bin/grep -n '_this: JsValue' crates/script/elidex-js/src/vm/host/document.rs crates/script/elidex-js/src/vm/host/document_accessors.rs crates/script/elidex-js/src/vm/host/document_factories.rs`
  → `document_accessors.rs:120` (URL / documentURI), `:146` (readyState); plus the mapped `window.rs:805` (getSelection). At C1-pre's head
  among Document members (`document*.rs`): 1, `native_document_get_url` (the program's, R-10).
- **P-6 tests reading Document members as own properties**:
  `/usr/bin/grep -rn "getOwnPropertyDescriptor(document\|getOwnPropertyNames(document\|document\.hasOwnProperty\|Object\.keys(document" crates/script/elidex-js/src/vm/tests`
  → 10 in 4 files: `tests_cookie_referrer.rs:552`, `:587`, `:633`, `:669`, `:706`, `:743`;
  `tests_document_members.rs:125`; `tests_cssom_sheet.rs:712`; `tests_html_element_proto.rs:372`, `:373`. All
  rewritten to read the descriptor from `Object.getPrototypeOf(document)`. Complement (`for … in document`,
  `Reflect.ownKeys(document)`): `/usr/bin/grep -rn 'in document)\|ownKeys(document' crates/script/elidex-js/src/vm/tests` → 0.
- **P-7 tests pinning the install mechanism**: `tests_document_methods.rs` fn `document_methods_install_per_document_entity`
  (`:601`), `tests_clone_node.rs:242-280`. Rewritten to the prototype form (W11, W5).
- **P-11 every per-entity Document fact the Document members read** (PM 335 (2), PM 339; property: a Document state item of
  DOM §4.5 / HTML §3.1 / §6 / §7 that a P-4 member's native or dom-api handler reads). Population command: the VM-side
  reads `/usr/bin/grep -n 'ctx\.vm\.navigation\|is_tab_hidden\|cookie_jar\|selection\|custom_elements::\|host_data' crates/script/elidex-js/src/vm/host/document.rs crates/script/elidex-js/src/vm/host/document_accessors.rs crates/script/elidex-js/src/vm/host/document_factories.rs crates/script/elidex-js/src/vm/host/window.rs`
  plus, for the dom-api side, each handler named by `/usr/bin/grep -n 'invoke_dom_api\|invoke_document_accessor' crates/script/elidex-js/src/vm/host/document.rs crates/script/elidex-js/src/vm/host/document_accessors.rs crates/script/elidex-js/src/vm/host/document_factories.rs`
  resolved through `crates/dom/elidex-dom-api/src/registry.rs` and read for the components it touches.

  **Rule (PM 339)**: C1-pre keeps only a fact written once, at Document creation; a fact that navigation, `pushState`
  or any later event rewrites goes to `#11-document-base-url` (URL family) or its own slot. Classes: **kept** (C1-pre
  answers it; no store added), **existing** (an ECS component C1-pre reads with `this` and does not write; not a
  per-Document fact store C1-pre could own), **program**, **own slot**. Result: C1-pre owns no per-Document store; its
  kept answers are the creation-time values of Documents without a browsing context (D-5, D-6) and HEAD's constants
  (D-7, D-8).

  | fact (spec) | members | store at HEAD | written (spec) | class → C1-pre / home |
  |---|---|---|---|---|
  | URL (DOM §4.5) | `URL`, `documentURI`, `cookie`'s URL | `current_url` (per-VM), none per Document | at creation, then by the *URL and history update steps* (`pushState`, `replaceState`, fragment navigation) | **program** → unchanged in C1-pre; R-10 |
  | document base URL (HTML §2.4.3) | `baseURI` | `DocumentBaseUrl` (ECS, a derived cache): spawned as `about:blank` by `create_document_node` (`crates/core/elidex-ecs/src/dom/mod.rs:474-480`), absent on a cloned Document (`tree_clone.rs:31`, read as `about:blank` by `document_base_url`, `crates/dom/elidex-dom-api/src/element/document_base.rs:58-63`), then rewritten by `BaseUrlMaintainer` | re-derived whenever a `<base>` or the URL changes | **program** → unchanged (`BaseUrlMaintainer` → `GetDocumentBaseURI`); R-10 |
  | node tree: children, descendants, elements by id / class / name / tag, `title`, `doctype`, ParentNode (DOM §4.2) | 23 of class T | `TreeRelation`, `TagType`, `Attributes`, `TextContent`, `DocTypeData` (ECS) | every mutation | **existing** → dom-api handlers with `this` (unchanged) |
  | node document of created and adopted nodes (DOM §4.5 `createElement` …, Range / traversal owner; *adopt* step 3.1) | `create*` (7), `adoptNode` | `AssociatedDocument` (ECS, on the node; `pub(crate)` in `crates/core/elidex-ecs/src/components.rs:585`, written through C0b's `NewNodeOwner`, which the dom-api handlers obtain as `EcsDom::node_document_for_new_nodes(this)`, `crates/core/elidex-ecs/src/dom/node_document.rs:23`, `:66`) | at each node's creation; rewritten by *adopt* (`EcsDom::adopt`, `crates/core/elidex-ecs/src/dom/tree/mutation.rs:252`) | **existing** → unchanged (`this` is the owner / adopt target) |
  | focused area (HTML §6.6) | `activeElement`, `hasFocus` | `ElementState::FOCUS` (ECS), read by `elidex_dom_api::focus::current_focus(dom, this)` | every focus update | **existing** → unchanged; `hasFocus` also reads the browsing context (row below) |
  | style sheets (CSSOM) | `styleSheets` | ECS (collected by `collect_document_stylesheets`) | every sheet insertion / removal | **existing** → unchanged |
  | browsing context (HTML §7.3.2) | `defaultView`, `cookie` (averse), `referrer`, `hidden`, `visibilityState`, `hasFocus`, `getSelection` | `HostData::document` (per-VM) | bind / unbind / navigation replace the association | **own slot** → the predicate (D-4) reads it; home `#11-browsing-context-state-ecs-components` (B1); R-11 |
  | visibility state (HTML §6.2) | `hidden`, `visibilityState` | bound: `HostData::is_tab_hidden` (per-VM, the traversable's); others: none | initially "hidden"; rewritten by *update the visibility state* | **kept** for non-bound Documents (the initial value, never rewritten: D-6, no store); **own slot** for the bound one → R-11 |
  | the document's selection (Selection API) | `getSelection` | `HostData::selection_state` (per-VM; `crates/script/elidex-js/src/vm/host_data/engine_feature/bind.rs:315`, `:349`) | the selection's range changes with every selection change | **kept** for non-bound Documents (`null`, D-5, no store); **own slot** for the bound one → R-11 |
  | referrer (HTML §3.1 *the document's referrer*) | `referrer` | `NavigationState::referrer` (per-VM), read only for the bound document | "can be set when the Document is created" — once, at creation | written once at creation, so d5-eligible for a creation-stamped component; C1-pre adds none (its only non-empty writer is the shell's navigation pipeline) → **own slot** → R-8 |
  | current document readiness (HTML §3.1.5) | `readyState` | VM constant; `SessionCore::document_ready_state` (session, shell-written) for the bound document | initially "complete"; rewritten by *update the current document readiness* during load and by the *document open steps* | **own slot** → D-8 keeps HEAD's constant; **R-4 still applies** |
  | mode (DOM §4.5) | `compatMode` | none (constant "CSS1Compat") | "only ever changed from the default for documents created by the HTML parser … and by a new browsing context" (DOM §4.5), i.e. after creation; set to no-quirks by the *document open steps* step 15 | **own slot** → D-7 routes to `GetCompatMode` (same constant); **R-9 still applies** |
  | custom element registry (DOM §4.5) | `createElement` (and C0c's `adoptNode`) | the VM's registry, used whatever `this` is (`native_document_create_element` `crates/script/elidex-js/src/vm/host/document_factories.rs:30-101`; C0c memo T15r) | at creation; set later by `CustomElementRegistry.initialize()` step 2 (HTML §4.13.4) when null | **own slot** → existing `#11-document-null-registry-adopt-reset` (scope (1): null for DOMParser documents and the inert template document) — no C1-pre change |
  | cookies (the jar) | `cookie` | `HostData::cookie_jar` | — | not a per-entity fact (CLAUDE.md side-store exception (b)) |

  Facts no Document member reads at `43e350c7` (encoding, content type, origin, type, allow declarative shadow roots,
  about base URL) are outside the population; a member that starts reading one must add a row.

### §5.2 Existing tests and docstrings whose expectation changes

P-6's 10 sites (descriptor lookups move to the prototype; values unchanged); P-7's two tests (their observable is kept,
their mechanism text changes). The `document.URL` tests (`tests_document_methods.rs:565-596` and the rest) are
untouched: the URL native keeps HEAD's source. Docstrings: §4.1 item 6.

### §5.3 Touched files

Line counts `wc -l` at `43e350c7`; re-measured at the cut (§9 item 1). Every row is in `elidex-js`.

| file | lines | change | 1000-line decision |
|---|---|---|---|
| `crates/script/elidex-js/src/vm/host/document.rs` | 534 (1197 at `f9cf8c40`; D-14 (b) landed, #599) | installer → `register_document_prototype`; the `getSelection` table entry | < 1000 (split by C0 D-14 (b), D-10) |
| `crates/script/elidex-js/src/vm/host/document_accessors.rs` (D-14 (b)) | 590 | the predicate, class-B natives, `native_document_get_selection` (NEW), `compatMode`, `readyState` | < 1000 |
| `crates/script/elidex-js/src/vm/globals.rs` | 972 | one registration call | < 1000 (C1-pre0, D-11) unless `vm-p4-rooting-carved` (+33) lands first; then > 1000 and the verdict is the rooting PR's (ledger `#11-vm-operand-rooting-by-construction` touch-time note); its +33 is re-counted after it rebases onto #592 |
| `crates/script/elidex-js/src/vm/mod.rs` | 2785 | one field | not split (D-11) |
| `crates/script/elidex-js/src/vm/init.rs` | 858 | one initializer | < 1000 |
| `crates/script/elidex-js/src/vm/host/elements.rs` | 425 | one arm, comment | < 1000 |
| `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs` | 201 | `PrototypeKind::Document` | < 1000 |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 729 | delete the set | < 1000 |
| `crates/script/elidex-js/src/vm/host/globals.rs` | 57 | drop the install call | < 1000 |
| `crates/script/elidex-js/src/vm/host/dom_parser.rs` | 410 | drop the install | < 1000 |
| `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` | 277 | drop the hook | < 1000 |
| `crates/script/elidex-js/src/vm/host/media_query.rs` | 647 | one comment | < 1000 |
| `crates/script/elidex-js/src/vm/tests/tests_document_wrapper.rs` (NEW) | — | W cells | new file |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 207 | the new module's `mod` line | < 1000 |
| `crates/script/elidex-js/src/vm/tests/tests_document_methods.rs` | 997 (3 lines of headroom) | one test rewritten in place (P-7, `:601`), net ≤ 0 lines | < 1000; if the rewrite needs more lines it moves to the new file instead |
| `tests_cookie_referrer.rs` 900, `tests_document_members.rs` 210, `tests_cssom_sheet.rs` 984, `tests_html_element_proto.rs` 655, `tests_clone_node.rs` 296 (in `crates/script/elidex-js/src/vm/tests/`) | as listed | P-6 / P-7 rewrites, net 0 lines | < 1000 |

### §5.4 Named risks (and the observable change each is)

- **R-1 own-property enumeration of `document`** → empty (X1, W2).
- **R-2 non-bound Documents' visibility and selection** → `hidden` `true`, `visibilityState` `"hidden"`,
  `getSelection()` `null` for DOMParser and cloned Documents (W6).
- **R-3 brand checks** → the `readyState` getter called on a non-Document platform object throws `TypeError` (W9,
  W10); the `URL` / `documentURI` getters do not (R-10).
- **R-4 a Document wrapper allocated while unbound** would keep `Node.prototype` (B1×B2; D-3).
- **R-5 registration order** — a missing `register_document_prototype` panics at the first Document wrap (loud;
  §4.1 items 3, 5).

## §6. Test plan

Columns: *HEAD* = `43e350c7`; *C1-pre* = spec. **Red-first**: every red cell
runs on the base before the change with its control green; the failures go in the PR description. Fixture shape (W1,
W15, W22): a second Document entity in the bound `EcsDom` made by `dom.create_document_node()` (the spawn C1's I will
use), not bound, with an element appended and that element exposed as a global through
`vm.inner.create_element_wrapper` + `vm.set_global` (`tests_custom_elements.rs:1185` idiom) — the shape C1's I has (a
Document in the page `EcsDom` reached through `ownerDocument`).

### §6.1 VM integration (`tests_document_wrapper.rs`, NEW)

| cell | kind | script | HEAD | C1-pre = spec |
|---|---|---|---|---|
| W1 | red (fixture) | `d = p.ownerDocument;` then separately `typeof d.createElement`, `d.defaultView === null`, `'createElement' in Object.getPrototypeOf(d)`, `d.createElement('i').ownerDocument === d`, `d.body` | `"undefined"`, `false`, `false`, throws `TypeError`, `undefined` | `"function"`, `true`, `true`, `true`, `null` |
| W2 | red | `[Object.getOwnPropertyNames(document).length, 'createElement' in Object.getPrototypeOf(document)]` | `[N > 0, false]` | `[0, true]` |
| W3 | red (order transfer) | `Object.getOwnPropertyNames(Object.getPrototypeOf(document))` equals the list `Object.getOwnPropertyNames(document)` printed on the base (pinned as a literal; on a base at or after `0a5a77ba` it has `adoptNode` right after `getSelection`, `document.rs:429-432`) | `Node.prototype`'s names | the pinned list |
| W4 | guard | `p.ownerDocument === p.ownerDocument`; `dp.documentElement.parentNode === dp`; `document.documentElement.parentNode === document` | all `true` | all `true` |
| W5 | guard | `[Object.getPrototypeOf(dp), Object.getPrototypeOf(document.cloneNode(true))].every(x => x === Object.getPrototypeOf(document))`; `typeof dp.createElement`, `typeof document.cloneNode(true).createElement` | `true`; `"function"` ×2 | same |
| W6 | red (D-6) | `[dp.hidden, dp.visibilityState, dp.getSelection()]`; the same for `document.cloneNode()` | `[false, "visible", Selection]` | `[true, "hidden", null]` |
| W7 | guard | `[dp.defaultView, dp.cookie, (dp.cookie = 'a=b', dp.cookie), dp.referrer, dp.hasFocus(), dp.readyState, dp.compatMode]` | `[null, "", "", "", false, "complete", "CSS1Compat"]` | same |
| W8 | guard (bound document unchanged) | `[document.defaultView === globalThis, document.hidden, document.visibilityState, document.getSelection() === getSelection(), document.compatMode, document.readyState]` | `[true, false, "visible", true, "CSS1Compat", "complete"]` | same |
| W9 | red | `Object.getOwnPropertyDescriptor(P, 'body').get.call(document.body)` with `P = Object.getPrototypeOf(document)` (HEAD: `P = document`) — message | HEAD's message (run on the base) | `TypeError` "Illegal invocation" |
| W10 | red | `Object.getOwnPropertyDescriptor(P, 'readyState').get.call(document.body)`; HEAD with `P = document` | returns a string | `TypeError` "Illegal invocation" |
| W11 | red (rewrite of `document_methods_install_per_document_entity`) | bind doc1, record `Object.getPrototypeOf(document)` in a global; unbind; bind doc2; `[typeof document.getElementById, Object.getPrototypeOf(document) === saved, document.hasOwnProperty('getElementById')]` | `["function", true, true]` | `["function", true, false]` |
| W12 | guard (gates) | `'getElementsByTagName' in document`; with `compat-webapi` + BrowserCompat `'cookie' in document`, in an App-mode VM `'cookie' in document` | `true`; `true`; `false` | same |
| W15 | red (Rust-set state, D-5) | as W1, with `ElementState::FOCUS` set on the fixture element (Rust) and the tab visible: `document.hasFocus.call(d)` | `true` | `false` |
| W22 | red (fixture, C0c's flush; H-13) | W1's fixture (its Document entity, here D, and its element `p`); nothing wraps D before the script. `r = []; customElements.define('x-a', class extends HTMLElement { adoptedCallback(o, n) { r.push([o, n]); } }); x = document.createElement('x-a'); p.appendChild(x); document.adoptNode(x);` — two *adopt*s, (`document`, D) then (D, `document`); the flush allocates D's first wrapper as the first callback's newDocument. Then, in a second `eval`, with `a = r[1][0]` (the second callback's oldDocument): `[r.length, a === r[0][1], typeof a.createElement, 'createElement' in Object.getPrototypeOf(a), a.hidden, a.visibilityState, typeof a.getSelection == 'function' ? a.getSelection() : 'absent']`. **Control** (green on both): `[r[1][1] === document, typeof r[1][1].createElement]` | `[2, true, "undefined", false, undefined, undefined, "absent"]` (by code reading at `43e350c7`, not run); control `[true, "function"]` | `[2, true, "function", true, true, "hidden", null]` (HTML §6.2 `hidden` / `visibilityState` getters with the initial "hidden"; Selection API §4.1 `Selection?`; D-6); control same |

### §6.2 Mutation guards

| mutation | killed by |
|---|---|
| no `PrototypeKind::Document` arm (Document → `Node.prototype`) | W1, W2, W5, W22 |
| members installed on the prototype **and** on wrappers | W2, W11 |
| install sequence reordered | W3 |
| a class-B native keeps a private copy of the check / the predicate inverted | W6, W7, W8 |
| `hasFocus` without step 1 | W15 |
| Document `getSelection` delegates unconditionally | W6 |
| gate evaluated per wrapper / dropped | W12 |
| DOMParser or clone install left behind | P-2's DoD grep, W11 |
| `readyState` without brand check | W10 |

### §6.3 Kept green

`tests_document_methods.rs`, `tests_document_members.rs`, `tests_dom_parser.rs`, `tests_clone_node.rs`,
`tests_cookie_referrer.rs`, `tests_cssom_sheet.rs`, `tests_html_element_proto.rs`, `tests_parent_node_mixin.rs`,
`tests_base_href.rs`, the event-handler attribute suites. Commands: `cargo test -p elidex-js --all-features`, then
`mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| the document URL (store, writers, every reader) and the base URL's dependency on it | `#11-document-base-url` (R-10) | every Document's `URL` / `documentURI` answers the page's current URL; with no `<base href>`, `baseURI` and relative-URL resolution use `about:blank`, not the document's URL |
| DOM node interface objects (`Node`, `Element`, `Document`, …; `HTMLDocument` alias) | `#11-navigator-interface-object-branding` (the VM-wide interface-object program; R-6) | `x instanceof Document` throws `ReferenceError`; `new Document()` is unavailable |
| `[LegacyUnforgeable] location` on Document | R-13 | `document.location` is absent |
| `[SameObject]` `forms` / `images` / `links` | R-14 | each read allocates a new collection |
| referrer, readiness, mode | R-8 (append), R-4, R-9 (slot proposals) | §5.1 P-11 rows |
| browsing-context facts as ECS components (the association, the bound document's visibility state and selection) | B1 / `#11-browsing-context-state-ecs-components` (R-11) | the predicate's input is per-VM `HostData::document`; visibility and selection are per-VM |
| the per-Document custom element registry | `#11-document-null-registry-adopt-reset` | `I.createElement` / `dp.createElement` use the VM's registry |
| `activeElement` for a Document without a navigable | §1.1 X7 (spec-silent; current behaviour kept) | spec-silent step 1 |
| `document_receiver`'s silent fallback for non-platform receivers | pre-existing elidex policy | Web IDL would throw `TypeError` |
| multi-realm `Document.prototype` | B1 (H-1 below) | one prototype per VM |

Own deferrals: 2 slot proposals (R-4, R-9), each a pre-existing defect C1-pre does not widen; R-6, R-8, R-10 and R-11
append pre-existing defects to existing slots; R-13 and R-14 are for the PM to register or decline; the other rows
are pre-existing or another unit's scope.

## §8. Edges, hand-offs and records

**Edges.**
- e-14b: **C0 D-14 (b) → C1-pre** (code predecessor; D-10): landed as #599 `e8b6a24d`, before C0c and C1-pre.
- e-c0c: **C0c → C1-pre** (no semantic dependency): C0c landed first, #611 `0a5a77ba`. Its `adoptNode` entry moves to
  `Document.prototype` with the rest (D-2, W3), and its adopted-reaction flush is a first-wrap path D-1 covers (H-13,
  W22).
- c-pre: **C1-pre → C1** (semantic: C1's T5, T1's second and third legs and M3's non-URL legs need D-1, D-4 to D-8).
  C1's T1 fourth / fifth legs and M3's `URL` / `documentURI` legs need `#11-document-base-url` instead (R-2, R-10).
- e-rc2a: **L3's RC-2a → C1-pre** (landing order, PM 371; common function `native_node_clone_node`). RC-2a landed as
  #596 `17b2a84d` with the `install_document_methods_if_cloned_doc` call kept (`node_methods_extras.rs:218`), as its
  plan `docs/plans/2026-11-range-rc2a-clone-created.md` `:167-169`, §4.6, said; C1-pre lands second and
  deletes the call and the helper (§4.1 item 6) from RC-2a's version of the function (R-12).
- e-dbu: **C1-pre ↔ `#11-document-base-url`**, either order (no data edge after PM 339). Common code: the URL and
  `cookie` natives, whose install C1-pre moves to the prototype without changing them: rebase only.
- VM P4 (driver's check, re-done here): `vm-is-constructor-move` landed as #588 (`4fd85f4e`); its only file in common
  with C1-pre is `vm/mod.rs` (one `mod` line near the top vs one field near `:509`: rebase only).
  `vm-construct-isconstructor-check` landed as #591 (`145927c9`; no C1-pre file); `vm-p4-slice-1a-plan` is docs-only
  (`git diff --stat $(git merge-base origin/main <branch>) <branch>`); slice 1aa cites `dom_bridge.rs:567` as an idiom
  only. The parked `vm-p4-rooting-carved` (`37a97d49`) adds 33 lines to `vm/globals.rs` and touches `vm/mod.rs` and
  `vm/tests/mod.rs`; if it lands first, `vm/globals.rs` passes 1000 lines (§5.3).

**Hand-offs.**
- H-1 to B1: one `Document.prototype` per realm.
- H-2 to C1: §4.3; R-2.
- H-3 to C0c: none (C0c landed first; R-3).

**Records (for the PM; exact texts).**
- **R-1 ledger `#11-bridge-document-post-install`**: "Owner: Layout lane (PM 328). Discharged by C1-pre (plan
  `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`): every Document wrapper gets `Document.prototype` at
  `create_element_wrapper` (prototype selection by `NodeKind`, the bridge step the other node kinds already have); the
  three path-specific installs and `HostData::document_methods_installed` are deleted. Closes at C1-pre's landing. The
  document-URL facet of PM 328's disposition went to `#11-document-base-url` (PM 339; C1-pre R-10)."
- **R-2 C1 memo**: T1's second leg `I instanceof Document` → `Object.getPrototypeOf(I) === Object.getPrototypeOf(document)`
  (no `Document` interface object, H-9); T1's fourth / fifth legs (`I.URL`, `I.documentURI` `=== 'about:blank'`) and
  M3's `URL` / `documentURI` legs rest on `#11-document-base-url`, not C1-pre (PM 339; R-10), so C1's edge c-pre
  text "I gets C1-pre's default about:blank at creation" is withdrawn; §4.1 / §4.8 / §5.3: the VM wrapper work is
  C1-pre's (edge c-pre); R-C5's precedent text per §4.3; Q-S10 answered by §5.1 P-4 / P-11; H-C10 lists both Document
  spawn sites (§0.3 P-7); C1's PR writes the c-pre edge (C1-pre → C1) and lists C1-pre as its row's prerequisite.
- **R-3 C0c memo**: no edit. C0c landed first (#611), so its D-13 / H-c6 ("installed as an own property") describe
  the code C1-pre starts from; C1-pre's D-2 moves the entry to `Document.prototype` with the rest.
- **R-4 slot proposal `#11-document-readiness-one-source`**: **Defect**: the bound document's `readyState` has two
  answers — the VM constant `"complete"` (`document_accessors.rs:144-153`) and the dom-api `GetReadyState`, which reads the
  shell-driven `SessionCore::document_ready_state` (`pipeline.rs:488`) and which the VM never calls; a script running
  before load completes reads `"complete"`. **Why deferred**: readiness is rewritten after creation (HTML §3.1.5
  *update the current document readiness*), so PM 339's rule gives it its own slot; changing it changes the bound
  document's observable readiness during load; the per-Document home of readiness belongs with the browsing-context
  cluster. **Trigger**: a PR that runs scripts before the shell marks the document complete, or B1.
  **Re-eval**: 2027-01-31. **Owner**: Layout (default; the PM may assign).
- **R-5 umbrella, applied by C1-pre's PR** (O-1, PM 223 O-1 / 232: one owner per §1.2 row; each PR adds only its own,
  in landing order; the umbrella revision number is assigned at landing): §1.2 gains the row "| **C1-pre** (owner:
  Layout lane; PM 328 / 335 / 339) | slot `#11-bridge-document-post-install`: every Document wrapper is a Document | C0
  D-14 (b) and RC-2a landed |"; the Order line gains C1-pre before "C1's handling decided". §1.3 is not
  touched (no C1 node exists yet; edge c-pre goes to C1's PR, R-2).
- **R-6 ledger `#11-navigator-interface-object-branding` (append)**: "DOM node interface objects (`Node`, `Element`,
  `Document`, …; `HTMLDocument` alias) are absent: `x instanceof Document` throws `ReferenceError` (C1-pre H-9). The
  trigger widens to DOM-node interface objects."
- **R-8 ledger `#11-browsing-context-state-ecs-components` (append; B1 §5 req 5,
  `docs/plans/2026-06-agent-scoped-ecsdom-world.md:447-455`, subsumes it and covers `host/navigation.rs`)**:
  "**Defect**: a Document's referrer (HTML §3.1 *the document's referrer*: "can be set when the Document is created")
  is held per-VM (`NavigationState::referrer` `crates/script/elidex-js/src/vm/host/navigation.rs:184`, written only by
  the shell through `Vm::set_navigation_referrer` `crates/script/elidex-js/src/vm/vm_api.rs:186`), not on the Document
  entity. Other Documents read `""`, their spec value; a second Document with a non-empty referrer (iframes, B1) would
  read the bound one's. Written once, at creation, so a creation-stamped component meets d5; deferred because its only
  non-empty writer is the shell navigation pipeline."
- **R-9 slot proposal `#11-document-mode`**: **Defect**: a Document's mode (DOM §4.5: no-quirks / quirks /
  limited-quirks) is not stored; `compatMode` answers `"CSS1Compat"` for every Document, including one whose doctype
  puts it in quirks mode, and layout reads no mode. **Why deferred**: neither parser tier computes the mode at
  `43e350c7`; giving it a store without a writer would be a stub. **Trigger**: a PR that implements quirks-mode
  detection in either parser tier, or quirks-mode layout. **Re-eval**: 2027-01-31. **Owner**: L1/L2 (PM assigns).
- **R-10 ledger `#11-document-base-url` (append; PM 339, user (A))**: "**Defect**: a Document's URL (DOM §4.5:
  "Unless stated otherwise, a document's … URL is "about:blank"") is stored for no Document; `URL` / `documentURI`
  answer the VM's `navigation.current_url` whatever `this` is (`native_document_get_url`
  `crates/script/elidex-js/src/vm/host/document_accessors.rs:118-129`, `_this`, at `43e350c7`). This is silent-wrong until this
  program lands: (a) the inert template document I (HTML §4.12.3 *appropriate template contents owner document*: a new
  Document, so its URL is the default) returns the page URL instead of `about:blank`, observable once C1 lands (C1's
  T1 fourth / fifth legs and M3's `URL` / `documentURI` legs); (b) a `createHTMLDocument` document would return the
  page URL instead of `about:blank` (DOM §4.5.1 `createHTMLDocument()` sets type and content type, not the URL), but
  `document.implementation` / `createHTMLDocument` do not exist in the VM (`git grep -n
  'createHTMLDocument\|"implementation"' -- crates/script/elidex-js/src` → 0 at `43e350c7`); (c) a cloned Document's
  URL is its source's (DOM §4.4 *clone a single node* step 5, Document: "Set copy's encoding, content type, URL, …
  to those of node"), not `about:blank`, and elidex returns the page's current URL instead — wrong once a `pushState`
  / fragment navigation follows the clone (`c = document.cloneNode(); history.pushState(null, '', '?y'); c.URL` reads
  `…?y`); (d) likewise a DOMParser document, whose URL is relevantDocument's at parse (HTML §8.5.1 step 3). The
  `cookie` natives read `current_url` too (correct today: only the bound document has a browsing context).
  **Input for this program's plan** (C1-pre rev 1.1's D-17, PM 338): elidex's `DocumentBaseUrl`
  (`crates/core/elidex-ecs/src/components.rs:131-137`; spawned as `about:blank` by `create_document_node`,
  `crates/core/elidex-ecs/src/dom/mod.rs:474-480`, and absent on a cloned Document, `tree_clone.rs:31`) is a cache
  derived from HTML §2.4.3 *document base URL* (step 1
  "If document has a descendant base element that has an href attribute, then return the frozen base URL of the first
  such base element in tree order", step 2 the base URL override, step 3 the *fallback base URL*), which
  `BaseUrlMaintainer` overwrites with a `<base>`'s frozen URL (`recompute_document_base`
  `crates/dom/elidex-dom-api/src/element/document_base.rs:134-163`, insert `:162`; called by
  `initialize_base_url_for_document` (`:189`, itself called by the bind initializer `:270`) and directly by the Insert /
  Remove / AttributeChange arms `:302`, `:346`, `:379`, each passing `about_blank_url()` as the
  fallback, DOMParser alone its caller URL, `crates/dom/elidex-form/src/inert_document.rs:200`). It is not
  the document's URL (DOM §4.5): the two differ whenever a `<base href>` exists (`document.URL` the page,
  `document.baseURI` the `<base>`'s frozen URL) or an about base URL applies (*fallback base URL* step 1 "If document's
  URL matches about:blank or matches about:srcdoc, and document's about base URL is non-null, then return document's
  about base URL"; step 2 "Return document's URL"). So `DocumentBaseUrl` cannot hold the URL; the URL is its input,
  through the fallback. Also input: the dom-api's unused `GetDocumentUrl` stub
  (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:14-30`, returns `"about:blank"`, no VM caller), and the
  URL pair's missing brand check (C1-pre leaves the native as HEAD's). C1-pre rev 1.1's withdrawn design (a second
  component with writers at spawn, clone, DOMParser, navigation commit and the *URL and history update steps*) is in
  that memo's history (`6b35d16c`), not a decision."
- **R-11 ledger `#11-browsing-context-state-ecs-components` (append)**: "**Defect**: three per-Document facts the
  Document members read are held per-VM and answered only for the bound document — the browsing-context association
  (`HostData::document`, read by C1-pre's `document_has_browsing_context`), the visibility state (HTML §6.2;
  `HostData::is_tab_hidden`, rewritten by *update the visibility state*) and the selection (Selection API;
  `HostData::selection_state`, one per VM). Other Documents answer the no-browsing-context values (C1-pre D-5 /
  D-6), their spec values, so nothing reads wrong today; a second Document with a browsing context (iframes, B1) would
  read the bound one's. Bind, navigation and visibility updates rewrite all three after creation, so d5 excludes a
  creation-stamped component; they belong with this cluster."
- **R-12 L3 (RC-2a), for information (PM 371)**: RC-2a landed first (#596) with `install_document_methods_if_cloned_doc`'s
  call kept (`node_methods_extras.rs:218`), as its plan said; C1-pre's PR then deletes that call, the helper and the doc bullet on Document clones in
  `native_node_clone_node` (§4.1 item 6), and leaves RC-2a's other changes as landed.
- **R-13 PM**: `[LegacyUnforgeable] location` on Document is absent (§7), for the PM to register or decline.
- **R-14 PM**: `[SameObject]` `forms` / `images` / `links` allocate per read (§7; the live-collection family, A1 gate
  B0 / B1), for the PM to register or decline.

## §9. Definition of done

1. Coordinates re-measured at the cut (D-14 (b) and RC-2a landed; `native_node_clone_node` re-read); §5.1 populations
   re-run and recorded; open VM P4 branches re-checked for `vm/globals.rs`, `vm/mod.rs`, `vm/init.rs`,
   `vm/host/elements.rs`, `vm/host_data/**`, `vm/tests/mod.rs`.
2. D-1 to D-8 and D-10 to D-12 implemented; P-1's DOM-node allocator count 1; P-2 0 hits; P-5 1 among Document
   members (`native_document_get_url`, R-10).
3. §6.1 cells present; every red cell (W15 and W22 included) run red on the base with its control green, recorded in
   the PR description.
4. §5.2 rewrites done; `git grep -n 'install_document_methods\|document_methods_installed\|bridge-document-post-install' -- crates` → 0.
5. `cargo test -p elidex-js --all-features` (§6.3) and `mise run ci` green.
6. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`
   exits 0 (run, not trusted).
7. The PR applies R-5. The PM has applied R-1, R-2, R-6, R-8, R-10, R-11 and relayed R-12 (or recorded why
   not), and registered or declined R-4, R-9, R-13 and R-14 (R-3 needs no edit and O-3 is moot: C0c landed first).

## §10. Revision history

*Rev 1* (2026-11-04): first draft (PM 328 brief), with PM 335 applied before the first commit: D-9 rewritten (the
component instead of "value per O-1"), D-13 to D-16 added, §1 axis B7, §2.2 to 21 pairs, §4.3, §5.1 P-8 to P-11, §6.1
W16 / W18 to W20, §6.2, §6.3, R-4 / R-7 to R-9; the draft's O-1 (URL home) is decided by PM 335. *Rev 1.1*
(2026-11-04, PM 338): D-17 (`DocumentUrl` vs the existing `DocumentBaseUrl`: the base URL is derived from the URL,
HTML §2.4.3, so the existing component is not reused), H-18, §5.1 P-12 (its writers and readers), the §3 row HTML
§2.4.3, D-14's fallback read-back, W21, the §6.4 row, the §7 row, R-7's addendum.
*Rev 2* (2026-11-04, PM 339 (user (A))): the URL part removed to `#11-document-base-url` (the store would be rewritten
by navigation while `current_url` also exists — two representations, against the 2026-06-11 ruling and d5, as #526
found). Removed: D-9, D-13 to D-17; H-14 to H-18 (H-14's two sites kept inline in §0.3 P-7 for R-2); axis B7 and its
6 pairs (§2.2 now 15, all intersecting); §1.1 X8, X9 and X10's URL leg; the §3 rows HTML §7.5.1, §2.4.3, §7.4.4,
§8.5.1 and the URL parts of DOM §4.5 / §4.4, HTML §3.1.4 / §4.12.3; rev 1's §4.3 (the URL store) and §6.2 (shell) /
§6.3 (unit), so §4.4 → §4.3 and §6.4 / §6.5 → §6.2 / §6.3; §5.1 P-8, P-9, P-10, P-12; the URL rows of §5.2 and
§5.3 (11 files outside `elidex-js`, plus `history.rs` / `navigation.rs`); risks R-6, R-7; W10's URL legs, W16 to W21;
7 mutation rows; edge C0b; record R-7; O-1, Q-1, Q-S5, Q-S6. Changed: P-11 re-classified by PM 339's rule (C1-pre
owns no per-Document store); R-2 (C1's URL legs are the program's), R-4 / R-8 why-deferred, R-5; §1 split decision
re-run (still three units: C1-pre0 and D-14 (b) still needed). Added: R-10 (the program's defect and D-17's
finding as its input), R-11. §0.3 P-2 corrects the ruling's date (2026-06-11).
*Rev 2.1* (2026-11-05): rebased on `f9cf8c40` (C1-pre0 landed as #592); cites into the moved code re-pinned; D-11
reduced to a pointer; removed edge e-pre0, §9 item 2 (C1-pre0) and O-2.
*Rev 2.2* (2026-11-05): plan-review R1 (frame `0927317b`) F1–F18 and PM 371. Removed: D-3's guard, Q-S1 to Q-S4
(Q-S1 → §4.1 item 3, Q-S2 / Q-S4 → H-19 / H-20, Q-S3 with the guard), the W6 gate on §1.1, D-6's uncited clause,
item 5's `.or` fallback, the "ratchet" wording. Changed: W15 to a guard; R-5 to the PR's own umbrella edit (O-1); R-6
and R-8 to appends; §3 Selection row to ✗; the referrer fact to HTML §3.1; the selection store to `selection_state`.
Added: D-4's node-navigable clause, D-5's W15 note, D-11's trigger clause (PM 371), D-12's `hasFocus` clause, §4.1
item 6's stale markers, edge e-rc2a and R-12, R-13, R-14, the rooting-branch note (§8, §5.3), the `vm/tests/mod.rs` row.
*Rev 2.3* (2026-11-05): plan-review re-check N1–N8 (PM 372); plan-review closed. R-5 leaves §1.3 alone (edge c-pre
→ R-2); W15 back to red; §4.1 item 6 gains five stale comment sites; R-6 trigger clause; R-8 why-deferred clause; e-rc2a
path and version; the X7 gate removed from §1.1; §5.3 `vm/globals.rs` row points to the rooting PR.
*Rev 2.4* (2026-11-06, PM 422): no browser measurement; §1.1's Chromium and other-engine columns and DoD item 3
deleted (C1-pre implements the spec column; X7 is spec-silent, current behaviour kept).
*Rev 2.5* (2026-11-08, PM 534): re-measured at `43e350c7` (`0a5a77ba`'s code: D-14 (b) #599, RC-2a #596, C0b #608,
#603 and C0c #611 landed since `f9cf8c40`). §0.1's frame and every cite re-pinned; cites into the code D-14 (b) moved
now name `document_accessors.rs` / `document_factories.rs`. **Premise corrected (H-13)**: rev 2.4's list of the
generic paths that wrap a Document omitted C0c's adopted-reaction flush (`custom_elements/flush.rs:143-144`); H-13
rewritten, the flush added to D-1's path list, §1's K-0 lens, §2.1 B6, §2.2 B1×B6 and §5.1 P-3, and §6.1 W22 added
(red by code reading, with Rust fixture; its control is green on both), with §6.2's first row and DoD 3. **`adoptNode` is a
Document member (C0c)**: D-2's list, the W3 note, P-4 (a reproducible count: 40 entries, class T 29), §4.2's count,
§2.2 B3×B6; R-3 reduced to "no edit", edge e-c0c, hand-off H-3, O-3 marked moot, DoD 7. Text corrected, decisions
unchanged: §0.3 P-7 (both spawn sites; the cloner's copy has no `DocumentBaseUrl`), the P-11 base-URL and
node-document rows (C0b's `NewNodeOwner`, `adoptNode`), R-10's input (`document_base.rs` callers), H-12, D-10, edges
e-14b / e-rc2a and R-12 (D-14 (b) and RC-2a landed). D-5's `set_focus_bit` gate re-checked: holds (#603's paths only
clear the bit). §3 gains rev 2.5's lookups; §5.3's line counts re-measured; the P-5 / P-11 commands cover the three
`document*.rs` files.
*Rev 2.5.1* (2026-11-08, PM 534): focused re-check MINs, decisions unchanged. W1's third leg and W22's fourth leg
`Object.getPrototypeOf(x) === Object.getPrototypeOf(document)` did not discriminate (true at HEAD: a Document entity
resolves to `PrototypeKind::OtherNode`, `Node.prototype`, like the bound document); replaced by `'createElement' in
Object.getPrototypeOf(x)` (`false` at HEAD, `true` at C1-pre); W22 stays red by code reading through its other legs.
Edge e-rc2a's RC-2a plan cite re-pinned to `:167-169`. R-10: `:270` calls `initialize_base_url_for_document`, which
calls `recompute_document_base` (`:189`); the direct callers are `:302`, `:346`, `:379`. D-6 names both §6.2 writers
(*set the initial visibility state* from *initialize the navigable* step 6, HTML §7.3.1, and *update the visibility
state*); §3 gains the lookups.

## §11. Open questions

**For the PM / user.**
- **O-3 Order with C0c (e-c0c).** Moot since rev 2.5: C0c landed first (#611 `0a5a77ba`) and D-14 (b) before both
  (#599). No open question remains for the PM.
