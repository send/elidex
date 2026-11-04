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
- **PM decision 335** (relayed, 2026-11-04; option text not shown): widens the scope. (1) C1-pre owns the per-entity
  document URL, store and read: an ECS component on the Document entity (CLAUDE.md side-store→component rule),
  default `about:blank`; one source — the main document's URL is written into that component where navigation sets
  it; the reader never falls back to `navigation.current_url`; writer and algorithm on the engine-independent side,
  vm/host marshals only. (2) The scope is bounded: every per-entity Document fact the installed members read is
  enumerated by property, each with C1-pre's writer and reader or a slot (§5.1 P-11, §8 Records). (3) C1 adds only a
  URL leg to its T1 (`I.URL === 'about:blank'`); C1-pre's test plan pins I's URL through the generic `ownerDocument`
  path and the main document's URL after navigation (W16, W17).
- The motivating findings are C1's plan-review R1 (frame `8ea74ae4`): aggregate F1 (I's wrapper is not a Document:
  Ax1 IMP, Ax2 IMP-2, Ax5 IMP-1) and F2 (`URL` ignores the receiver: Ax3 IMP, Ax1 MIN), both routed to the PM.

**Scope.**
1. One `Document.prototype` interface prototype object (NEW) carrying today's Document member suite, chosen for every
   wrapper of a `NodeKind::Document` entity at the bridge's single allocation step, `create_element_wrapper` (D-1).
2. Removal of the three path-specific installs, the per-entity installer and its `HostData` side-store (D-2).
3. One browsing-context predicate for Document receivers, replacing six copies, and the members that must consult it
   but do not (D-4, D-5, D-6).
4. The document URL as one per-Document ECS component, `DocumentUrl` (NEW), with every writer (creation, clone,
   DOMParser, navigation commit, URL update) and every reader among the Document members (`URL`, `documentURI`,
   `cookie`, DOMParser's *relevantDocument's URL*) (D-9, D-13 to D-16).
5. Each Document member answers from `this`: the receiver-ignoring natives are brand-checked and routed (D-7, D-8).
   Every other per-entity Document fact a member reads is enumerated with its writer and reader, or a slot (§5.1 P-11).

**Not in scope.** The `Document` interface object (no DOM node interface object exists in the VM, §7); re-pointing
`current_url`'s non-Document readers (`location`, `history`, fetch / API base, origin, storage, workers) to the
component and deleting the mirror (`#11-document-base-url`, R-7); the browsing-context cluster's migration to ECS
components (B1 / `#11-browsing-context-state-ecs-components`); new Document members (`characterSet`, `implementation`,
`location`, …); multi-realm (B1).

**Changelog.** *Rev 1* (2026-11-04): first draft, pre-plan-review; PM 335 applied before the first commit (O-1 of the
draft is decided by it: §11).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fab44a1d`** coordinate (this branch's base, `git merge-base HEAD origin/main`). Since then
`origin/main` gained #586 (`660e594c`, tools) and **#588** (`4fd85f4e`, VM P4's `vm-is-constructor-move`), which adds
one line (`mod construct_admission;`) near the top of `crates/script/elidex-js/src/vm/mod.rs`; no other cited file
changed (`git diff --stat fab44a1d origin/main -- crates/script/elidex-js/src/vm/host crates/script/elidex-js/src/vm/globals.rs crates/script/elidex-js/src/vm/host_data crates/core crates/dom crates/shell`).
Symbols are cited by name; re-find lines with `/usr/bin/grep -n '<symbol>' <file>`. Code reads are `git grep` /
`/usr/bin/grep -n` / `sed -n` over the `fab44a1d` tree, run 2026-11-04. Every § number ↔ title pair and every step
comes from `.claude/tools/webref` (`heading`, `dfn`, `body`, `idl`, `coverage-map`), run 2026-11-04; commands in §3.

### §0.2 Premises re-measured (the brief's claims, checked)

| id | claim | evidence (at `fab44a1d`) |
|---|---|---|
| H-1 | The per-entity installer is reached from three paths | `VmInner::install_document_methods_for_entity` `crates/script/elidex-js/src/vm/host/document.rs:983`; direct callers `document.rs:967` (inside `Vm::install_document_methods_if_needed`, called by `install_document_global` `crates/script/elidex-js/src/vm/host/globals.rs:49`), `crates/script/elidex-js/src/vm/host/dom_parser.rs:305`, `install_document_methods_if_cloned_doc` `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:249-264` (call `:263`). Command: `git grep -n 'install_document_methods' -- crates` (3 call paths, plus definitions, docs and two tests) |
| H-2 | Document members are own properties of each wrapper; there is no `Document.prototype` | installer comment `document.rs:996-1003`; the bridge's prototype dispatch `crates/script/elidex-js/src/vm/host/elements.rs:85-157` gives Document → `Node.prototype` (`:95-96`, arm `OtherNode` `:154-156`); `PrototypeKind` `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:18` has `DocumentType`, `DocumentFragment`, `ShadowRoot`, no `Document`. Document is the only DOM node kind whose wrapper gets no interface prototype object |
| H-3 | `ownerDocument` wraps through the generic path without install | `native_node_get_owner_document` `node_methods_extras.rs:42-51` → `invoke_dom_api(.., "ownerDocument.get", ..)` → phase 3 `create_element_wrapper(e)` `crates/script/elidex-js/src/vm/host/dom_bridge.rs:519-521`. (The brief's `dom_bridge.rs:75`, `wrap_entity_or_null`, is a sibling helper on the same allocator, not this getter's path) |
| H-4 | One allocator makes every DOM-node wrapper | `VmInner::create_element_wrapper` `elements.rs:74` (alloc `:158-164`, cache `:170-173`). Population P-1 (§5.1): 7 `HostObject` allocation sites, of which `elements.rs:159` is the only one for DOM node entities |
| H-5 | `URL` / `documentURI` ignore `this` | `native_document_get_url` `document.rs:466-477` (`_this`) returns `ctx.vm.navigation.current_url`; module doc `:42-43` ("navigation state, not DOM") |
| H-6 | The dom-api has an unused `document.URL.get` stub | `GetDocumentUrl` `crates/dom/elidex-dom-api/src/char_data/document_props.rs:14-30` returns `"about:blank"`; registered `crates/dom/elidex-dom-api/src/registry.rs:90`; no VM caller (`git grep -n 'document\.URL\.get' -- crates`). **Also unused**: `document.readyState.get` (`GetReadyState` `document_props.rs:82-99`, reads `SessionCore::document_ready_state`, written by the shell `crates/shell/elidex-shell/src/pipeline.rs:488`) and `document.compatMode.get` (`GetCompatMode` `document_props.rs:103-119`); the VM answers both from its own constants (`document.rs:492-501`, `:535-551`) |
| H-7 | Six natives decide "has a browsing context" by comparing `this` with the bound document, each with its own copy | `/usr/bin/grep -n 'bound_doc != Some(doc)' crates/script/elidex-js/src/vm/host/document.rs` → `:580` defaultView, `:630` cookie get, `:675` cookie set, `:722` referrer, `:872` hidden, `:903` visibilityState (6) |
| H-8 | Two Document members read VM-global state without consulting `this` at all | `readyState` `document.rs:492-501` (`_this`, constant `"complete"`); `getSelection` = `super::window::native_window_get_selection` (`document.rs:1095`; `crates/script/elidex-js/src/vm/host/window.rs:803-816`, `_this`, the VM's Selection singleton). `hasFocus` `document.rs:827-849` reads `this`'s focus but gates on the VM-global `is_tab_hidden`, not on `this`'s browsing context |
| H-9 | No DOM node interface object exists in the VM | the global-name population: `git grep -n '=> "\(Document\|Node\|Element\|DocumentFragment\|ShadowRoot\|HTMLDocument\)"' -- crates/script/elidex-js/src` → 0; `HTMLElement` is the only DOM interface name (`crates/script/elidex-js/src/vm/well_known.rs:1546`, custom elements). So `x instanceof Document` throws `ReferenceError` (code reading, not run) |
| H-10 | The VM's spec-level policy is fixed at construction | `spec_level_policy` set in `crates/script/elidex-js/src/vm/init.rs:177-185`; `installs` / `installs_dom` `crates/script/elidex-js/src/vm/globals.rs:1157-1169` read it. A once-per-VM install on a prototype therefore applies the same gates as today's per-wrapper install |
| H-11 | Prototype-level install of the same mixins has precedents | `install_parent_node_mixin` on `Element.prototype` (`crates/script/elidex-js/src/vm/host/element_proto.rs:108`) and `DocumentFragment.prototype` (`crates/script/elidex-js/src/vm/host/document_fragment_proto.rs:48`); `install_event_handler_attrs` on `HTMLElement.prototype` (`crates/script/elidex-js/src/vm/host/html_element_proto.rs:134`) |
| H-12 | `document.rs`'s 1000-line split is already decided | C0 draft D-14 (b) (reviewed, PM 258 / 318): before C0c, a pure move of the node factories (`document.rs:278-421`) to `vm/host/document_factories.rs` and the accessors (`:423-938`) to `vm/host/document_accessors.rs`; the installer and tables stay in `document.rs` (≈ 537 lines). Coordinates unchanged at `fab44a1d` (`git diff --stat 27b50dfe fab44a1d -- crates/script/elidex-js/src/vm/host/document.rs` is empty) |
| H-13 | Today's script-reachable Document entities all get the install | bound document (H-1), DOMParser (H-1), `document.cloneNode` (H-1). `document.implementation` / `createHTMLDocument` do not exist in the VM (`git grep -n 'createHTMLDocument\|"implementation"' -- crates/script/elidex-js/src` → 0); `adoptNode` is not in the VM yet (C0c adds it; it throws for a Document argument and returns its argument otherwise). A Document entity first wrapped by the generic path is reachable at `fab44a1d` only through a Rust fixture; C1's I is the first script-reachable one |
| H-14 | A Document entity is spawned at exactly two sites | `EcsDom::create_document_node` `crates/core/elidex-ecs/src/dom/mod.rs:410-416` (spawns `TreeRelation`, `NodeKind::Document`, `DocumentBaseUrl(about_blank_url())`; callers `create_document_root` `:395`, the fragment throwaway `crates/core/elidex-ecs/src/dom/tree/teardown.rs:301`, DOMParser `crates/dom/elidex-form/src/inert_document.rs:119`; C1's I will be a third caller) and the shallow cloner's builder `clone_node_shallow_unchecked` `crates/core/elidex-ecs/src/dom/tree_clone.rs:270-379` for a Document source (spawn `:379`). Population P-8 |
| H-15 | No Document URL is stored anywhere | `crates/core/elidex-ecs/src/lib.rs:15-17` ("no real `DocumentUrl` ECS state yet"); `inert_document.rs:182-197` (DOMParser's caller URL seeds only `DocumentBaseUrl`); the clone-policy table `tree_clone.rs:13-40` has no URL row |
| H-16 | The main document's URL is committed by the shell, and updated synchronously by `history` | the engine boundary `HostDriver::set_current_url` (`crates/script/elidex-script-session/src/engine.rs:360`) → `ElidexJsEngine::set_current_url` (`crates/script/elidex-js/src/engine.rs:556-558`) → `NavigationState::set_current_url` (`crates/script/elidex-js/src/vm/host/navigation.rs:216-218`, `None` → `about:blank`). Shell production callers (P-9): `crates/shell/elidex-shell/src/pipeline.rs:258` (pipeline build), `crates/shell/elidex-shell/src/app/navigation.rs:139`, `:346`, `:662`, `crates/shell/elidex-shell/src/content/navigation.rs:345`, `:423`, `:726` (7). The VM-side write is the *URL and history update steps* in `pushState` / `replaceState` (`crates/script/elidex-js/src/vm/host/history.rs:248`). The service-worker thread's `vm/sw_thread.rs:196` writes a worker script URL, not a Document's |
| H-18 | The existing `DocumentBaseUrl` is the document **base** URL, a derived cache — not the document's URL | `DocumentBaseUrl` `crates/core/elidex-ecs/src/components.rs:131-137` ("Derived document base URL — first `<base>`'s frozen URL OR the fallback URL"); population P-12: one spawn writer (`create_document_node` `mod.rs:414`, `about:blank`), one maintainer writer (`recompute_document_base` `crates/dom/elidex-dom-api/src/element/document_base.rs:126-155`, insert `:154`; reached from `initialize_base_url_for_document` `:202-205` and the `BaseUrlMaintainer` arms `:300`, `:314`, `:366`, `:396`), every arm passing `about_blank_url()` as the fallback except DOMParser's caller URL (`inert_document.rs:196`); one reader, `document_base_url` `document_base.rs:50-62`, used by `GetDocumentBaseURI` (`document_props.rs:51`), `GetNodeBaseURI` (`:76`) and `href_accessor.rs:51` |
| H-17 | Every shell commit site has the page `EcsDom` and its Document entity in hand | `PipelineResult { dom, document, runtime, url, .. }` `crates/shell/elidex-shell/src/lib.rs:199-215`; the build-time site runs inside `run_scripts_and_finalize(dom, document, ..)` (`pipeline.rs:189-205`) |

### §0.3 Premise errors in the brief (reported)

- **P-1 (split).** The brief asks for a new "C1-pre0" split of `vm/host/document.rs`. That file's split is C0 D-14 (b)
  (H-12), already reviewed; a second seam for the same file would leave two split decisions for one file. C1-pre
  takes D-14 (b) as its code predecessor and adds no seam to `document.rs` (D-10). C1-pre does need a split of a
  different file it touches, `vm/globals.rs` (1533 lines); this memo names **that** prerequisite C1-pre0 (D-11).
- **P-2 (URL ownership), resolved by PM 335.** The document URL was a facet of `#11-document-base-url` (IBP umbrella
  §0.6 item 18: "their inputs: the document URL and about base URL"; `docs/plans/2026-09-inline-box-predicate.md:1659`:
  "either home answer for the document URL is the user's to ratify"), and the user ruled on 2026-09-28 that
  `current_url` stays per-VM until B1 (memory `project_browsing-context-state-ecs-components.md`, "RULING UPDATED BY
  THE USER (2026-09-28), option (iv)"). PM 335 moves the store and the Document-member reads into C1-pre; `current_url`
  is not deleted (its non-Document readers keep it, R-7), so the ruling's subject stays per-VM. Whether PM 335 needs the
  user's ratification under the IBP umbrella's sentence is §11 O-1.
- **P-3 (paths).** `document.implementation.createHTMLDocument` does not exist; `adoptNode` returns a Document never;
  `cloneNode` of a Document is one of the three install sites already (H-13). The populations that matter are
  defined by property in §5.1.
- **P-4 (slot remedy).** The slot's remedy direction ("a step of the bridge layer"; review readings: "post-install keyed
  on `NodeKind::Document`") was checked against the code: the bridge already has the step — prototype selection by node
  kind (H-2) — and Document is the only node kind without an interface prototype object. A post-install keeps own
  properties per wrapper and the `document_methods_installed` side-store. D-1 adopts the remedy in its WebIDL form.
- **P-5 (C1's T1).** C1's T1 second leg `I instanceof Document` throws `ReferenceError` on any head (H-9); hand-off R-2.
- **P-6 (owner).** The ledger entry still says "Owner: core VM (driver absent)"; PM 328 moves it to Layout (record R-1).
- **P-7 (C1's H-C10).** C1's premise "A Document entity is spawned in one place" misses the cloner's spawn (H-14); it
  matters to C1-pre because every Document spawn must write `DocumentUrl` (D-13). Hand-off R-2.

### §0.4 Decisions (this PR)

- **D-1 — `Document.prototype` at the bridge.** `VmInner::document_prototype: Option<ObjectId>` (NEW) is allocated once
  per VM by `register_document_prototype` (NEW), with `[[Prototype]]` = `Node.prototype` (Web IDL §3.7.3 step 3:
  Document inherits Node). `PrototypeKind::Document` (NEW) is returned by `HostData::prototype_kind_for` for
  `NodeKind::Document`, and `create_element_wrapper` maps it to `document_prototype` (Web IDL §3.8 *internally create a
  new object implementing the interface*: prototype = the interface prototype object). This is the one canonical place:
  every path that wraps a Document entity — the bound global, DOMParser, `cloneNode`, `ownerDocument`, `parentNode`,
  `getRootNode`, event paths, Range / TreeWalker containers, `natives_promise.rs:526`, C1's I, C0c's `adoptNode` — gets
  it by construction, whichever wraps first. Rejected: (a) the slot's post-install at the bridge (own properties per
  wrapper, contrary to Web IDL §3.7.6 / §3.7.7 "regular attributes / operations are exposed on the interface prototype
  object"; keeps the side-store; installs ~70 properties per Document wrapper); (b) a fourth file-local hook.
- **D-2 — members and their order.** `Document.prototype` receives, in today's order, the tables
  `DOCUMENT_METHODS_PRE_LIVE_COLLECTION`, `DOCUMENT_LIVE_COLLECTION_METHODS` (gated by `installs_dom(live_collection_spec_level())`),
  `DOCUMENT_METHODS_POST_LIVE_COLLECTION`, `document_traversal::FACTORIES`, `DOCUMENT_RO_ACCESSORS`,
  `DOCUMENT_RW_ACCESSORS`, `DOCUMENT_COOKIE_RW_ACCESSOR` (`compat-webapi` + `installs(document_cookie_spec_level())`),
  the ParentNode mixin and the event-handler IDL attributes (`Global`, `DocumentElement`, `DocumentOnly` scopes)
  (`document.rs:995-1053`). The A1 contract "install order = enumeration order" (Codex R9) moves to
  `Object.getOwnPropertyNames(Object.getPrototypeOf(document))` (cell W3). No Document member remains an own property;
  `[LegacyUnforgeable] location` would be the one own property and is not implemented (§7).
- **D-3 — identity unchanged.** One wrapper per Document entity per VM through `HostData`'s wrapper cache (CLAUDE.md
  side-store rule (a), interim; no migration). The prototype is fixed at first allocation, so a Document wrapper must
  never be allocated while unbound (`prototype_kind_for` returns `OtherNode` when unbound): named risk R-4, guard
  `debug_assert!` in `create_element_wrapper` for `NodeKind::Document` (Q-S3).
- **D-4 — one browsing-context predicate.** `document_has_browsing_context(ctx, doc) -> bool` (NEW, in
  `vm/host/document_accessors.rs` after D-14 (b)) = "`doc` is the bound document" (`HostData::document`). It is host
  marshalling of the VM's single browsing-context ↔ Document association, stated once. Its six copies (H-7) become
  calls; its storage home stays per-VM until B1 (`#11-browsing-context-state-ecs-components`).
- **D-5 — members that must consult it.** `hasFocus` gains the has-focus steps' step 1 (no navigable → `false`) before
  the tab-hidden check (HTML §6.6.4); Document `getSelection` gets its own native `native_document_get_selection`
  (NEW): brand-check `this`, no browsing context → `null` (Selection API §4.1 returns `Selection?`), else the Window's
  selection (today's behaviour). `getSelection` on Window is unchanged.
- **D-6 — the no-browsing-context values are the spec's.** `defaultView` `null`, `cookie` `""` / no-op set
  (cookie-averse), `referrer` `""`, `hasFocus()` `false`, `getSelection()` `null`, `hidden` `true` and `visibilityState`
  `"hidden"` (HTML §6.2: "visibility state … initially set to "hidden""; only *update the visibility state* changes it,
  and it runs for documents with a browsing context). Three of these change an observable value of today's DOMParser
  and cloned Documents (`hidden`, `visibilityState`, `getSelection`; cell W6): they are asserted only after §1.1 agrees,
  and a disagreement goes to the PM.
- **D-7 — `compatMode` reads the dom-api handler.** The VM native routes through `invoke_document_accessor(..,
  "document.compatMode.get", "")` and its constant is deleted (values identical; one answer instead of two, H-6).
- **D-8 — `readyState` is brand-checked; its value source is not changed.** It brand-checks `this` through
  `document_receiver`. Its value stays `"complete"` for every Document: the spec default for a Document without a
  browsing context (HTML §3.1.5) and today's answer for the bound one. The bound document's readiness has two answers
  today (H-6); routing it to `GetReadyState` changes the bound document's observable readiness during load — slot
  proposal R-4.
- **D-9 — `URL` / `documentURI` read the component.** The VM native becomes
  `invoke_document_accessor(ctx, this, "URL", "document.URL.get", "")` (brand check included), and `GetDocumentUrl`
  returns `EcsDom::document_url(this)` serialized (DOM §4.5: "return this's URL, serialized"). The VM no longer reads
  `navigation.current_url` for any Document member (P-5, P-10 ratchets).
- **D-10 — `document.rs` split = C0 D-14 (b).** No new seam (§0.3 P-1). Whichever of C0c and C1-pre is cut first lands
  D-14 (b) as the C0 draft states it; the other rebases (edge e-14b).
- **D-11 — C1-pre0: pure-move split of `vm/globals.rs` (1533).** C1-pre adds one call to `register_globals`. Seams (real
  cohesion, each a different spec family): (i) the ECMA-262 fundamentals — `register_prototypes`,
  `register_iterator_prototypes`, `register_object_global`, `register_array_global`, `register_math_global`,
  `register_json_global` and their only native `native_function_prototype_noop` (`globals.rs:61-72`, `:1172-1533`) →
  `crates/script/elidex-js/src/vm/globals_fundamentals.rs` (NEW: named after `globals_errors.rs` / `globals_primitives.rs`);
  (ii) the property-install primitives `register_global_function`, `install_methods`, `install_ro_accessors`,
  `install_rw_accessors`, `installs`, `installs_dom` (`:1014-1170`) → `crates/script/elidex-js/src/vm/globals_install.rs` (NEW).
  What stays is `register_globals`, the flat ordered registration sequence its authors annotate as unsplittable
  (`globals.rs:85-89`), ≈ 1000 lines; re-measured at the cut (§9 item 1). `vm/mod.rs` (2782) gains one field and is
  **not** split: it is one struct definition (`VmInner`, `:181-2776`), and its only seam, grouping fields into
  sub-structs, is a non-pure-move refactor deferred in memory `vm-inner-substruct-deferral`.
- **D-12 — Layering / ECS-native.** vm/host keeps prototype install, brand check and marshalling only (D-4 consolidates
  an existing association check; D-7 and D-9 remove host answers in favour of dom-api handlers). The per-entity
  side-store `HostData::document_methods_installed` (`HashSet<Entity>`, `crates/script/elidex-js/src/vm/host_data/mod.rs:106-120`)
  is deleted, not migrated: it records a per-VM fact about a wrapper, which the prototype makes unnecessary. Idiom
  table: OO "class Document owns its methods" → Web IDL interface prototype object (JS side); "document holds its URL
  as a field" → `DocumentUrl` component on the Document entity (ECS side, `Send + Sync`, not a per-VM handle: the
  side-store rule's component case); the discriminator for both is the existing `NodeKind::Document` component.
- **D-13 — `DocumentUrl` (NEW), one component, present on every Document.** `pub struct DocumentUrl(pub url::Url)` in
  `crates/core/elidex-ecs/src/components.rs`, beside `DocumentBaseUrl`, with `EcsDom::document_url(doc)` (NEW; by
  reference or by value per the crate's accessor idiom) and `EcsDom::set_document_url(doc, url)` (NEW, the
  only writer API). Present on every Document entity from spawn, written at both spawn sites (H-14):
  `create_document_node` spawns `DocumentUrl(about_blank_url())` (DOM §4.5: "Unless stated otherwise, a document's …
  URL is "about:blank"" — so C1's I, the fragment throwaway and a parser-built page start at `about:blank`), and the
  shallow cloner copies the source's for a Document source (DOM §4.4 *clone a node* step 5, Document: "Set copy's …
  URL … to those of node"; clone-policy table row "intrinsic copy", `tree_clone.rs:13-40`). Readers treat absence as a
  bug (`debug_assert!`), and in release return the DOM default `about:blank`, never `current_url`.
- **D-14 — DOMParser's URL.** `elidex_form::parse_into_inert_document(dom, markup, caller_url)` sets the new
  Document's `DocumentUrl` to `caller_url` (HTML §8.5.1 step 3: "URL is relevantDocument's URL"), and the VM native
  (`dom_parser.rs:282`) passes the **bound document's** `DocumentUrl` (relevantDocument = this's relevant global's
  associated Document), not `navigation.current_url`. The `initialize_base_url_for_document` fallback it already
  passes (`inert_document.rs:196`) is then read back from the new Document's `DocumentUrl`, so one value feeds both
  components (D-17; the base URL itself stays `#11-document-base-url`'s).
- **D-15 — the main document's URL: one commit function.** `elidex_script_session::commit_document_url(driver, dom,
  document, url: Option<Url>)` (NEW, engine-independent): writes `dom.set_document_url(document, url or about:blank)`
  (HTML §7.5.1 *create and initialize a Document object*: URL = creationURL; navigation commits and fragment navigations
  set the active document's URL), then calls `driver.set_current_url(url)` so the per-VM mirror stays equal. All 7
  shell production sites of H-16 (and the two test-support sites `app_test_support.rs:191`,
  `content_test_support.rs:249`) call it instead of `runtime.set_current_url`; `HostDriver::set_current_url`'s doc
  becomes "mirror only; commit through `commit_document_url`". `pipeline.url` (the shell's own copy) is untouched.
- **D-16 — the synchronous URL update.** `history.rs:248` (*URL and history update steps* step 9, HTML §7.4.4: "Set the
  URL given document to newURL") writes the bound document's component through the same pair of writes as D-15, in one
  VM helper `VmInner::update_document_url(doc, url)` (NEW; host marshalling: `EcsDom::set_document_url` plus the
  mirror), so a same-script `document.URL` after `pushState` reads the new URL. The history algorithm itself is
  pre-existing host code and is not moved here (§7).
- **D-17 — `DocumentUrl` is a second component, not `DocumentBaseUrl` reused (PM 338).** The existing abstraction was
  checked first. The spec keeps the two apart: a document's *URL* is its own fact (DOM §4.5: "URL (a URL) … Unless
  stated otherwise … URL is "about:blank""), while its *document base URL* (HTML §2.4.3) is derived — step 1 "If
  document has a descendant base element that has an href attribute, then return the frozen base URL of the first such
  base element", step 2 the base URL override, step 3 the *fallback base URL*, whose steps are 1 "If document's URL
  matches about:blank or about:srcdoc, and document's about base URL is non-null, then return document's about base
  URL" and 2 "Return document's URL". So the two differ whenever a `<base href>` exists (`document.URL` is the page,
  `document.baseURI` the `<base>`'s frozen URL) or an about base URL applies. elidex's `DocumentBaseUrl` is that derived
  value (H-18): `BaseUrlMaintainer` overwrites it with a `<base>`'s frozen URL, so it cannot hold the document's URL,
  and renaming it would leave the base URL without a home. **Relation**: `DocumentUrl` is an **input** of
  `DocumentBaseUrl` (through the fallback base URL); `DocumentBaseUrl` stays a cache with one writer family,
  `BaseUrlMaintainer` (`recompute_document_base` / `initialize_base_url_for_document`), and its one reader
  `document_base_url`. Writers of each: `DocumentUrl` — spawn default, clone, DOMParser, `commit_document_url` (where
  navigation writes the main document's, D-15), `update_document_url` (D-16); `DocumentBaseUrl` — spawn default and
  `BaseUrlMaintainer` only (C1-pre adds no writer to it). HTML §2.4.3 *set the URL* is "1 Set document's URL to url;
  2 Respond to base URL changes given document": step 1 is C1-pre's writers; step 2 (re-deriving the cache when the
  URL changes) and making the maintainer's fallback read `DocumentUrl` instead of `about_blank_url()` are
  `#11-document-base-url`'s, which owns every document-base reader and the fallback (R-7). The one C1-pre touch on
  that side is DOMParser (D-14): the caller URL is written to `DocumentUrl` first and the same value is then passed as
  the `initialize_base_url_for_document` fallback, read back from the component, so DOMParser's two facts come from one
  source. The base URL override (DOM) and the about base URL (HTML) are not modelled (§7).

## §1. Shape and verdict

**Axes (§2): 7** — B1 wrapper shape, B2 identity, B3 member surface and gates, B4 per-entity facts, B5 bind lifecycle,
B6 consumers (DOMParser, clone, C0c, C1), B7 the Document URL fact (store, writers, readers, mirror). ≥ 3 intersecting
axes → edge-dense: plan-review is mandatory (CLAUDE.md *Edge-dense work* (b)). It is a per-PR slice for the approved
EH-attrs umbrella once the PM adds it to umbrella §1.2 (R-5), so a terminal unit after plan-review (base case (c)).

**Split decision: three units.** C1-pre0 (`vm/globals.rs` pure move, D-11) → C1-pre (this memo), with C0 D-14 (b)
(`document.rs` pure move, D-10) also before C1-pre. D-1 to D-16 stay one PR: the prototype without D-4 to D-9 would
make I's surface reachable with the receiver-ignoring answers PM 328 rejects; D-9 without D-13 to D-16 has no store
to read; D-13 to D-16 without D-9 has no reader. Within the PR the URL work is its own commit after the prototype
commit (reviewable halves, one landing). Preflight reports SPLIT-RECOMMENDED (K = 4, M = 19): K counts the Selection
API row, a single member (`getSelection`), and M's rows are the spec homes of the one member suite and the one URL
fact, so a further split would separate a fact from its reader (the One-issue-one-way state) rather than shrink scope.

**K-0 lens (no step worse than HEAD).** Every changed outcome moves from HEAD's value to the spec's: own-property
enumeration of `document` (W2), brand checks (W9, W10), non-bound Documents' `hidden` / `visibilityState` /
`getSelection` (W6, gated on §1.1), and `URL` / `documentURI` of non-bound Documents (W16, W18, W19). The bound
document's `URL` keeps HEAD's value at every navigation step (W17), now from the component.

### §1.1 Browser measurement (before the implementation branch is cut)

Run in the built-in Chromium (and one other engine if available) on a page served over `http://`; record versions and
fill the two right-hand columns; any row whose result differs from the "C1-pre head" column goes to the PM. **No result
is filled here.** Spec column from the cited steps; HEAD column from code reading at `fab44a1d` (not run).
`dp = new DOMParser().parseFromString('<p></p>', 'text/html')`; `tpl = document.createElement('template')`.

| id | expression | spec (derivation) | elidex HEAD | elidex C1-pre head | Chromium | other |
|---|---|---|---|---|---|---|
| X1 | `Object.getOwnPropertyNames(document)` | `["location"]` (§3.7.6 / §3.7.7; `location` is `[LegacyUnforgeable]`) | every installed member | `[]` (`location` absent, §7) | | |
| X2 | `Object.getPrototypeOf(dp) === Object.getPrototypeOf(document)` | `true` (both Document) | `true` (both `Node.prototype`) | `true` | | |
| X3 | `Object.getPrototypeOf(document).hasOwnProperty('createElement')` | `true` (HTML §7.2.2: `HTMLDocument` is the Document interface object, so no intermediate prototype) | `false` | `true` | (prediction: `false`, an `HTMLDocument.prototype` in between) | |
| X4 | `[dp.hidden, dp.visibilityState]` | `[true, "hidden"]` (§6.2 initial value) | `[false, "visible"]` | `[true, "hidden"]` | (prediction: `[true, "hidden"]`) | |
| X5 | `dp.getSelection()` | `null` (Selection API §4.1; prose not extractable by `webref body` on 2026-11-04, see §3) | a `Selection` | `null` | (prediction: `null`) | |
| X6 | `[dp.defaultView, dp.cookie, dp.referrer, dp.hasFocus(), dp.readyState, dp.compatMode]` | `[null, "", "", false, "complete", "CSS1Compat"]` | same | same | | |
| X7 | `dp.activeElement === dp.body` | spec-silent for a Document without a navigable (§6.6.6 step 1 reads "this's node document's focused area") | `true` | `true` (unchanged) | | |
| X8 | `u = dp.URL; history.pushState(null, '', '?x'); [u === location.href.replace('?x',''), dp.URL === u]` | `[true, true]` (§8.5.1 step 3: dp's URL is fixed at creation) | `[true, false]` (receiver ignored) | `[true, true]` | | |
| X9 | `c = document.cloneNode(); history.pushState(null, '', '?y'); c.URL !== document.URL` | `true` (DOM *clone a node* step 5, Document) | `false` | `true` | | |
| X10 | `I = tpl.content.ownerDocument; [typeof I.createElement, I.defaultView, I.URL, I.hidden, I.getSelection(), I.activeElement]` | `["function", null, "about:blank", true, null, null]` (C1 memo M3; §6.6.6 steps 4–7) | n/a (I is `document`) | C1's head: the spec column | | |

X3 and X7 are informative (the spec decides X3; X7 is unchanged). X4, X5 gate W6 (D-6).

## §2. Coupled invariants

### §2.1 Invariant per axis

| axis | invariant |
|---|---|
| B1 shape | Every wrapper of a `NodeKind::Document` entity in a VM has `[[Prototype]]` = that VM's `Document.prototype`, chosen in `create_element_wrapper`; no other code decides a Document wrapper's shape |
| B2 identity | One wrapper per Document entity per VM (wrapper cache); the prototype choice never allocates or replaces a wrapper |
| B3 surface | `Document.prototype` carries exactly HEAD's member names in HEAD's order under HEAD's gates; no Document member is an own property of a wrapper; brand checks keep `document_receiver`'s contract (unbound / non-platform → fallback, wrong kind → `TypeError`) |
| B4 facts | Each member answers from `this`: tree facts through dom-api with `this`; browsing-context facts through the one predicate (D-4); the URL through `DocumentUrl` (D-9); every other fact per §5.1 P-11 |
| B5 lifecycle | Bind / unbind / rebind change no wrapper's prototype and run no per-bind install; a newly bound document's wrapper gets the same prototype; no per-entity install state exists |
| B6 consumers | DOMParser, `cloneNode`, C0c's `adoptNode`, C1's I carry no path-specific Document step; C1's T5, T1 (third and URL legs) and M3 hold on C1's head |
| B7 URL | `DocumentUrl` (the URL) and `DocumentBaseUrl` (the derived base URL) stay separate, and C1-pre writes only the former (D-17); every Document entity carries exactly one `DocumentUrl` from spawn; it changes only through `EcsDom::set_document_url`, called by the D-14 / D-15 / D-16 writers; every Document-member read of a document's URL reads it; the per-VM `current_url` equals the bound document's component after every commit and update |

### §2.2 All pairs (21), with intersection

| pair | intersect? | intersection |
|---|---|---|
| B1×B2 | yes | The prototype is chosen at first allocation and kept by every cache hit; a Document wrapper allocated while unbound would keep `Node.prototype` forever (R-4; D-3's guard; Q-S3) |
| B1×B3 | yes | Members live on the prototype, so every Document wrapper has them; order is the prototype's (W2, W3) |
| B1×B4 | yes | The same function objects serve every Document; answers differ only through `this` (W1, W6) |
| B1×B5 | yes | A rebind to a new document allocates its wrapper with the same prototype; `install_document_global` no longer installs (W11) |
| B1×B6 | yes | DOMParser / clone / `ownerDocument` / C0c / I all reach `create_element_wrapper`; population P-1 is the ratchet (W1, W5) |
| B1×B7 | yes | `URL` lives on the prototype and reads the receiver's component, so the generic-path wrapper of a fresh Document reads `about:blank` (W16) |
| B2×B3 | yes | Members returning objects keep today's identity behaviour (`forms` allocates per call, a pre-existing `[SameObject]` defect, §7); the prototype adds no identity |
| B2×B4 | yes | Two Documents have two wrappers whose answers differ (`dp.body !== document.body`; W1) |
| B2×B5 | yes | The wrapper cache is the only per-entity VM state left for Documents once `document_methods_installed` is gone (W11) |
| B2×B6 | yes | `p.ownerDocument === p.ownerDocument`; `dp.documentElement.parentNode === dp` (W4) |
| B2×B7 | no | The URL is a component of the entity; no wrapper identity is involved in writing or reading it |
| B3×B4 | yes | `URL` / `documentURI` / `readyState` / `getSelection` gain the brand check the rest have (W9, W10) |
| B3×B5 | yes | Gates are evaluated once at registration; the policy is fixed at VM construction (H-10), so the gated set equals today's per-wrapper set (W12) |
| B3×B6 | yes | C0c appends `adoptNode` to `DOCUMENT_METHODS_POST_LIVE_COLLECTION`; under D-1 it lands on the prototype (edge e-c0c, R-3) |
| B3×B7 | yes | The `cookie` accessor (compat-gated) reads `this`'s component for the cookie URL; with the gate off it is absent and reads nothing (W12, W20) |
| B4×B5 | yes | The predicate reads `HostData::document` at call time; after a rebind the old document reads "no browsing context" (unchanged from HEAD's copies) |
| B4×B6 | yes | DOMParser and cloned Documents: `hidden` / `visibilityState` / `getSelection` change (W6, gated); I gets the same answers (C1's T1, M3) |
| B4×B7 | yes | `URL` answers from `this`'s component for bound and non-bound Documents alike — no bound-ness branch in the reader (W16 vs W17) |
| B5×B6 | yes | A DOMParser document's wrapper survives unbind with its prototype; no re-install is needed on rebind (W11 variant) |
| B5×B7 | yes | The shell commits the URL (D-15) whether or not the VM is bound — the component is written on the `EcsDom`, not through `HostData`; the VM-side update (D-16) runs only inside a bound script (W17 legs) |
| B6×B7 | yes | DOMParser's URL is the bound document's component at parse (D-14, W18); a clone copies its source's (D-13, W19); C1's I gets `about:blank` from `create_document_node` with no C1 code (W16) |

Count: 21 pairs; 20 intersect, 1 does not (B2×B7). Re-count:
`awk '/^### §2.2/{f=1} /^## §3/{f=0} f && /^\| B[0-9]×B[0-9]/' <memo> | awk -F'|' '{print $3}' | sort | uniq -c`.

## §3. Spec coverage map

Lookups (2026-11-04): `.claude/tools/webref coverage-map webidl 3.7.3 webidl 3.8 webidl 3.7.6 webidl 3.7.7 dom 4.5 dom 4.4
html 7.2.2 html 7.3.2 html 7.4.4 html 7.5.1 html 3.1.4 html 3.1.5 html 6.2 html 6.6.4 html 6.6.6 html 8.5.1 html 4.12.3
selection-api 4.1` (skeleton); `heading` for each pair; `idl dom Document`, `idl html Document` (`[SameObject]`
collections, `location` `[LegacyUnforgeable]`), `idl selection-api Document` (`Selection? getSelection()`); step text
from `body webidl dfn-interface-prototype-object`, `body webidl js-attributes`, `body webidl js-operations`, `body
webidl internally-create-a-new-object-implementing-the-interface`, `body dom concept-document-url` (default URL
`about:blank`), `body dom dom-document-url`, `body dom concept-node-clone`, `body html dom-document-defaultview`, `body
html dom-document-referrer`, `body html dom-document-cookie`, `body html current-document-readiness`, `body html
visibility-state`, `body html has-focus-steps`, `body html dom-documentorshadowroot-activeelement`, `body html
dom-domparser-parsefromstring`, `body html htmldocument`, `body html url-and-history-update-steps` (step 9), `body html
initialise-the-document-object` (URL = creationURL). `body selection-api dom-document-getselection` failed ("anchor …
not encountered"; possible multipage drift); the §4.1 row rests on the IDL and the dfn only.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| Web IDL §3.7.3 Interface prototype object | creation steps 1–3 | 3: interface inherits → proto = inherited interface's prototype object (Node) | `register_document_prototype` (NEW) in `crates/script/elidex-js/src/vm/host/document.rs` (D-1) | ✓ | no |
| Web IDL §3.8 Platform objects implementing interfaces | *internally create a new object implementing the interface*: prototype step | prototype = interface prototype object for the primary interface in realm | `create_element_wrapper` `crates/script/elidex-js/src/vm/host/elements.rs:74` via `PrototypeKind::Document` (NEW) `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs:169` | ✓ | no |
| Web IDL §3.7.6 Attributes | regular attributes on the interface prototype object | unforgeable / `[Global]` → on the instance (Document: `location` only) | `DOCUMENT_RO_ACCESSORS`, `DOCUMENT_RW_ACCESSORS`, `DOCUMENT_COOKIE_RW_ACCESSOR`, event-handler attributes → prototype (D-2) | ✗ (`location` not implemented, §7) | no |
| Web IDL §3.7.7 Operations | regular operations on the interface prototype object | unforgeable / `[Global]` → instance (none for Document) | method tables, `document_traversal::FACTORIES`, ParentNode mixin → prototype (D-2) | ✓ | no |
| WHATWG DOM §4.5 Interface Document | a document's URL, default "about:blank"; `URL` / `documentURI` / `compatMode` getter steps | URL: return this's URL serialized; compatMode: quirks → "BackCompat", else "CSS1Compat" | `DocumentUrl` (NEW) `crates/core/elidex-ecs/src/components.rs`; `create_document_node` `crates/core/elidex-ecs/src/dom/mod.rs:410` (D-13); `GetDocumentUrl` `crates/dom/elidex-dom-api/src/char_data/document_props.rs:14` (D-9); `GetCompatMode` (D-7) | ✗ (mode not modelled, §5.1 P-11) | no |
| WHATWG DOM §4.4 Interface Node | *clone a node* step 5 (Document: copy's encoding, content type, URL, origin, type, mode … to those of node); `ownerDocument` | Document clone | `clone_node_shallow_unchecked` `crates/core/elidex-ecs/src/dom/tree_clone.rs:270` copies `DocumentUrl` (D-13); `native_node_clone_node` `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:201` (hook removed, D-2) | ✗ (encoding, content type, origin, type, mode not modelled) | yes |
| WHATWG HTML §7.5.1 Shared document creation infrastructure | *create and initialize a Document object*: URL = creationURL | navigation-created Documents | `commit_document_url` (NEW) in `crates/script/elidex-script-session/src/engine.rs`; shell sites `crates/shell/elidex-shell/src/pipeline.rs:258` and P-9 (D-15) | ✓ | yes |
| WHATWG HTML §2.4.3 Document base URLs | *document base URL* 1–3; *fallback base URL* 1–2; *set the URL* 1–2 | URL ≠ base URL when a `<base href>` exists (step 1) or an about base URL applies (fallback step 1); the fallback returns the document's URL | `DocumentUrl` (NEW) is the URL; `DocumentBaseUrl` stays the derived base (`crates/dom/elidex-dom-api/src/element/document_base.rs:126`) (D-17); set-the-URL step 2 and the fallback's read of `DocumentUrl` are `#11-document-base-url`'s | ✗ (fallback reads `about:blank`; about base URL, base URL override not modelled) | yes |
| WHATWG HTML §7.4.4 Non-fragment synchronous "navigations" | *URL and history update steps* step 9 (set the URL given document to newURL) | `pushState` / `replaceState` | `crates/script/elidex-js/src/vm/host/history.rs:248` via `update_document_url` (NEW) (D-16) | ✓ | yes |
| WHATWG HTML §7.2.2 The Window object | `defaultView` getter step 1; `HTMLDocument` = the Document interface object | browsing context null → null | `native_document_get_default_view` `document.rs:565` via the predicate (D-4) | ✓ | no |
| WHATWG HTML §7.3.2 Browsing contexts | a Document's browsing context | bound document ↔ the VM's browsing context; any other → null | `document_has_browsing_context` (NEW) (D-4) | ✓ | no |
| WHATWG HTML §3.1.4 Resource metadata management | cookie-averse Document; `cookie` get / set ("the document's URL"); `referrer` | no browsing context → cookie-averse; cookie URL = `this`'s `DocumentUrl` | `document.rs:610`, `:658`, `:704` via the predicate (D-4, D-6) and the component (D-9) | ✓ | yes |
| WHATWG HTML §3.1.5 Reporting document loading status | current document readiness, initially "complete"; `readyState` getter | Documents without a browsing context keep the default | `native_document_get_ready_state` `document.rs:492` (brand check, D-8) | ✗ (bound document's readiness, R-4) | no |
| WHATWG HTML §6.2 Page visibility | visibility state initially "hidden"; `hidden` / `visibilityState` getters | no browsing context → never updated → "hidden" | `document.rs:854`, `:888` via the predicate (D-6) | ✓ | no |
| WHATWG HTML §6.6.4 Processing model | *has focus steps* 1–2 | 1 node navigable null → false; 2 no system focus → false | `native_document_has_focus` `document.rs:827` (D-5) | ✓ | no |
| WHATWG HTML §6.6.6 Focus management APIs | `activeElement` steps 1–7; `hasFocus()` | candidate a Document → body, else document element, else null | `native_document_get_active_element` `document.rs:775` (unchanged; X7) | ✗ (step 1 for no navigable is spec-silent) | no |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 2–3 | new Document's URL = relevantDocument's URL | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110`; VM `crates/script/elidex-js/src/vm/host/dom_parser.rs:282` (D-14) | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *appropriate template contents owner document* 1.1.1 (browsing context null) | I has no browsing context; its URL is the default | none in C1-pre (C1 creates I with `create_document_node`, D-13 gives it `about:blank`) | ✓ | no |
| Selection API §4.1 Extensions to Document interface | `Selection? getSelection()` | no browsing context → null | `native_document_get_selection` (NEW) in `vm/host/document_accessors.rs` (NEW: D-14 (b)) (D-5) | ✓ | no |

## §4. Mechanism

### §4.1 The prototype (D-1, D-2)

1. `vm/mod.rs`: field `document_prototype: Option<ObjectId>` (NEW) beside `document_fragment_prototype` (`:506`);
   `vm/init.rs`: its `None` initializer beside `:302`.
2. `vm/host/document.rs` (after D-14 (b), where the installer and tables stay): `install_document_methods_for_entity`
   becomes `VmInner::register_document_prototype` (NEW): allocate an ordinary object with `prototype: node_prototype`,
   run the install sequence of `document.rs:995-1053` against it unchanged, store it. `Vm::install_document_methods_if_needed`
   is deleted.
3. `vm/globals.rs` (after C1-pre0): one call in `register_globals`, after every registration the install sequence
   needs (`register_node_prototype` `:215`; whatever `install_event_handler_attrs` and `install_parent_node_mixin`
   require — Q-S1) and before the first possible Document wrapper (the first bind).
4. `entity_kind.rs`: `Some(NodeKind::Document) => PrototypeKind::Document` in `prototype_kind_for`'s `match`
   (`:169`); `PrototypeKind::Document` (NEW) documented like its siblings.
5. `elements.rs`: arm `PrototypeKind::Document => self.document_prototype.or(self.node_prototype).expect(..)` like the
   `DocumentFragment` arm (`:145-148`); the comment `:95-96` loses "Document".
6. Deleted: `install_document_methods_if_cloned_doc` and its call (`node_methods_extras.rs:213`, `:238-264`), the
   DOMParser install (`dom_parser.rs:303-305`), the installer call in `install_document_global` (`globals.rs` host
   `:45-49`), `HostData::document_methods_installed` and its initializer (`host_data/mod.rs:106-120`, `:617`). Doc
   comments naming them: `node_methods_extras.rs:19-21`, `:197-200`, `dom_parser.rs:32`, `host_data/mod.rs:83`,
   `crates/script/elidex-js/src/vm/host/media_query.rs:124` (cites the set as a precedent; re-pointed), the slot name in
   `node_methods_extras.rs:245-247`.

### §4.2 Per-entity facts (D-4 to D-9)

| member | HEAD source | C1-pre |
|---|---|---|
| `defaultView`, `cookie` (get, set), `referrer`, `hidden`, `visibilityState` | own copy of `bound_doc != Some(doc)` (H-7) | `document_has_browsing_context` (D-4); `hidden` / `visibilityState` no-BC values → `true` / `"hidden"` (D-6); `cookie`'s URL = `this`'s `DocumentUrl` (D-9) |
| `hasFocus` | tab-hidden (VM-global) then `this`'s focus | predicate first (step 1), then tab-hidden, then `this`'s focus (D-5) |
| `getSelection` | Window's native, `_this` | `native_document_get_selection` (NEW): brand check, predicate → `null`, else the Window's selection (D-5) |
| `compatMode` | VM constant after brand check | `GetCompatMode` through `invoke_document_accessor` (D-7) |
| `readyState` | `_this`, constant | brand check, constant (D-8; R-4) |
| `URL`, `documentURI` | `_this`, `current_url` | `GetDocumentUrl` reading `this`'s `DocumentUrl` (D-9) |
| the other 28 members (§5.1 P-4, class T) | dom-api / engine-indep with `this` | unchanged |

Natives of class B live in `vm/host/document_accessors.rs` (NEW: D-14 (b)) after D-14 (b), with the predicate.

### §4.3 The Document URL (D-13 to D-16)

| role | site | what |
|---|---|---|
| store | `DocumentUrl` (NEW) `crates/core/elidex-ecs/src/components.rs`; re-export in `crates/core/elidex-ecs/src/lib.rs` | one per Document entity |
| writer: spawn (default) | `create_document_node` `crates/core/elidex-ecs/src/dom/mod.rs:411` | `about:blank` (DOM §4.5) |
| writer: clone | `clone_node_shallow_unchecked` `tree_clone.rs:270-379` (Document source) | copy (DOM §4.4 clone step 5); clone-policy table row |
| writer: DOMParser | `parse_into_inert_document` `inert_document.rs:110-201` | the caller's (relevantDocument's) URL (HTML §8.5.1 step 3) |
| writer: navigation commit | `commit_document_url` (NEW) in `crates/script/elidex-script-session/src/engine.rs`, called from the 7 shell sites of P-9 | creationURL / the committed URL (HTML §7.5.1; fragment and traversal commits) |
| writer: URL update | `VmInner::update_document_url` (NEW) from `history.rs:248` | newURL (HTML §7.4.4 step 9) |
| API | `EcsDom::set_document_url` / `EcsDom::document_url` (NEW) in `crates/core/elidex-ecs/src/dom/mod.rs` | the only write / read API |
| reader: `URL`, `documentURI` | `GetDocumentUrl` (dom-api) | `this`'s component |
| reader: `cookie` get / set | `document.rs:640`, `:694` (in `document_accessors.rs` after D-14 (b)) | `this`'s component (marshalling read) |
| reader: DOMParser's relevantDocument | `dom_parser.rs:282` | the bound document's component |
| mirror | `NavigationState::current_url` | written in the same function as the component (D-15, D-16); its non-Document readers are `#11-document-base-url`'s (R-7) |

Doc comments made false by the store and rewritten in the same commit: `crates/core/elidex-ecs/src/lib.rs:15-17`, the
`#11-document-url-real-navigation` mentions (`crates/core/elidex-ecs/src/dom/mod.rs:391`,
`crates/dom/elidex-dom-api/src/element/href_accessor.rs:33`, `inert_document.rs:182-197`,
`crates/script/elidex-js/src/vm/tests/tests_base_href.rs:45`, `crates/script/elidex-js/src/vm/tests/tests_dom_parser.rs:788`;
that slot is closed, subsumed by `#11-document-base-url`), `document.rs:42-43`, `dom_parser.rs:280-282`,
`HostDriver::set_current_url`'s doc (`crates/script/elidex-script-session/src/engine.rs:353-360`).

### §4.4 What C1 gets

On C1's base (C1-pre landed): `tpl.content.ownerDocument` (I, allocated by `ownerDocument`'s generic path, spawned by
`create_document_node`) has `Document.prototype` and `DocumentUrl(about:blank)`, so C1's T5, T1's third leg and its new
URL leg (`I.URL === 'about:blank'`, PM 335 (3)), and M3's legs read the spec values with no C1 code. C1's §4.1 / §4.8
statement "the VM touches none of it" becomes true; R-C5's precedent text becomes "every Document wrapper has
`Document.prototype` (C1-pre D-1)" (R-2).

## §5. Populations, touched files, named risks

### §5.1 Populations (re-runnable)

`NT='/tests/\|tests\.rs\|/tests_\|test_helpers'`.
- **P-1 wrapper allocations** (property: a VM object whose kind is `HostObject` is created):
  `git grep -nE '(kind: |\.kind = )ObjectKind::HostObject' -- 'crates/script/elidex-js/src/*.rs' | /usr/bin/grep -v "$NT"`
  → 7 at `fab44a1d`: `vm/globals.rs:108` (globalThis), `vm/host/canvas/mod.rs:334`, `:829` (2D contexts via
  `intern_wrapper`), `vm/host/elements.rs:159` (DOM nodes), `vm/host/offscreen_canvas/mod.rs:365`, `:405`,
  `vm/host/worker.rs:395`. Only `elements.rs:159` wraps DOM node entities, so it is the only Document wrapper
  allocator. Ratchet (§9 item 3): the count of DOM-node allocators stays 1.
- **P-2 path-specific installs**: `git grep -n 'install_document_methods\|document_methods_installed' -- crates` →
  0 hits at C1-pre's head.
- **P-3 Document wrapper first-wrap paths** (callers of the allocator that can receive a Document entity):
  `git grep -n 'create_element_wrapper(\|wrap_entity_or_null(\|wrap_entities_as_array(' -- 'crates/script/elidex-js/src/*.rs' | /usr/bin/grep -v "$NT"`
  plus `invoke_dom_api`'s phase 3. Not enumerated one by one: D-1 makes the allocator decide, so the population's size
  is not load-bearing; its use is that none of them may add a Document step (P-2's ratchet).
- **P-4 Document members by fact source** (property: the member's native and what it reads). Source: the tables
  `document.rs:1071-1197` plus `document_traversal::FACTORIES` (`crates/script/elidex-js/src/vm/host/document_traversal.rs:235-239`);
  for each, `/usr/bin/grep -n 'fn <native>' -A12`. 39 table entries at `fab44a1d`:
  class **T** (tree facts of `this`, 28): `getElementById`, `querySelector`, `querySelectorAll`,
  `getElementsByTagName`, `getElementsByClassName`, `getElementsByName`, `createElement`, `createTextNode`,
  `createComment`, `createDocumentFragment`, `createRange`, `createTreeWalker`, `createNodeIterator`,
  `documentElement`, `head`, `body`, `firstElementChild`, `lastElementChild`, `children`, `childElementCount`,
  `baseURI`, `doctype`, `forms`, `images`, `links`, `activeElement`, `styleSheets`, `title`;
  class **B** (browsing-context facts, 6): `defaultView`, `cookie`, `referrer`, `hidden`, `visibilityState`,
  `hasFocus`; class **G** (receiver ignored or constant, 5): `URL`, `documentURI`, `readyState`, `getSelection`,
  `compatMode`. The ParentNode mixin and the event-handler attributes are class T (they act on `this`).
- **P-5 receiver-ignoring natives**: `/usr/bin/grep -n '_this: JsValue' crates/script/elidex-js/src/vm/host/document.rs`
  → `:468` (URL / documentURI), `:494` (readyState); plus the mapped `window.rs:805` (getSelection). 0 at C1-pre's
  head among Document members (`document*.rs`).
- **P-6 tests reading Document members as own properties**:
  `/usr/bin/grep -rn "getOwnPropertyDescriptor(document\|getOwnPropertyNames(document\|document\.hasOwnProperty\|Object\.keys(document" crates/script/elidex-js/src/vm/tests`
  → 10 in 4 files: `tests_cookie_referrer.rs:552`, `:587`, `:633`, `:669`, `:706`, `:743`;
  `tests_document_members.rs:120`; `tests_cssom_sheet.rs:711`; `tests_html_element_proto.rs:364`, `:365`. All
  rewritten to read the descriptor from `Object.getPrototypeOf(document)`. Complement (`for … in document`,
  `Reflect.ownKeys(document)`): `/usr/bin/grep -rn 'in document)\|ownKeys(document' crates/script/elidex-js/src/vm/tests` → 0.
- **P-7 tests pinning the install mechanism**: `tests_document_methods.rs` fn `document_methods_install_per_document_entity`
  (`:600`), `tests_clone_node.rs:242-280`. Rewritten to the prototype form (W11, W5).
- **P-8 Document spawn sites** (property: an entity spawned with `NodeKind::Document`):
  `git grep -n 'world\.spawn(\|world_mut()\.spawn(' -- 'crates/*.rs' | /usr/bin/grep -v "$NT"` → 14 spawn sites; the
  bundles carrying `NodeKind::Document` are `crates/core/elidex-ecs/src/dom/mod.rs:411` (`create_document_node`) and
  `tree_clone.rs:379` (the cloner copies the source's `NodeKind`); the rest spawn elements, character data,
  fragments, shadow roots, Window, Worker, canvas and scratch entities (read per line). Both write `DocumentUrl` (D-13);
  a unit cell (U1, U2) per site.
- **P-9 main-document URL commits** (property: the shell tells the engine the active document's URL):
  `git grep -n '\.set_current_url(' -- 'crates/*.rs' | /usr/bin/grep -v "$NT"` → `crates/script/elidex-js/src/engine.rs:557`
  (the engine impl) and the 7 shell sites of H-16, plus the two test-support sites
  (`crates/shell/elidex-shell/src/app_test_support.rs:191`, `crates/shell/elidex-shell/src/content_test_support.rs:249`,
  which the filter drops). At C1-pre's head the shell sites call `commit_document_url`; the ratchet is "no shell call of
  `.set_current_url(` outside `commit_document_url`".
- **P-10 Document-URL reads** (property: a Document member or DOMParser reads a document's URL):
  `/usr/bin/grep -n 'navigation\.current_url' crates/script/elidex-js/src/vm/host/document*.rs crates/script/elidex-js/src/vm/host/dom_parser.rs`
  → `document.rs:474` (URL), `:640` (cookie get), `:694` (cookie set), `dom_parser.rs:282` (4); 0 at C1-pre's head. The
  complement — `current_url` readers that are not Document members — is R-7's population:
  `git grep -n 'navigation\.current_url\|\.current_url\b' -- 'crates/*.rs' | /usr/bin/grep -v "$NT"` (≈ 40 hits in
  `location.rs`, `history.rs`, `fetch/`, `storage.rs`, `service_worker/`, `worker.rs`, `event_source.rs`,
  `request_response/`, `navigation.rs`, `sw_thread.rs`, `engine.rs` and the shell).
- **P-11 every per-entity Document fact the Document members read** (PM 335 (2); property: a Document state item of
  DOM §4.5 / HTML §3.1 / §6 / §7 that a P-4 member's native or dom-api handler reads). Population command: the VM-side
  reads `/usr/bin/grep -n 'ctx\.vm\.navigation\|is_tab_hidden\|cookie_jar\|selection\|custom_elements::\|host_data' crates/script/elidex-js/src/vm/host/document.rs crates/script/elidex-js/src/vm/host/window.rs`
  plus, for the dom-api side, each handler named by `/usr/bin/grep -n 'invoke_dom_api\|invoke_document_accessor' crates/script/elidex-js/src/vm/host/document.rs`
  resolved through `crates/dom/elidex-dom-api/src/registry.rs` and read for the components it touches.

  | fact (spec) | members | store at HEAD | C1-pre writer → reader, or slot |
  |---|---|---|---|
  | URL (DOM §4.5) | `URL`, `documentURI`, `cookie` | `current_url` (per-VM), none for other Documents | `DocumentUrl`, §4.3 (D-13 to D-16) |
  | node tree: children, descendants, elements by id / class / name / tag, `title`, `doctype`, ParentNode (DOM §4.2) | 23 of class T | `TreeRelation`, `TagType`, `Attributes`, `TextContent`, `DocTypeData` (ECS) | existing writers → dom-api handlers with `this` (unchanged) |
  | node document of created nodes (DOM §4.5 `createElement` …, Range / traversal owner) | `create*` (7) | `AssociatedDocument` (ECS) | existing (`Some(this)`) → unchanged |
  | document base URL (HTML §2.4.3) | `baseURI` | `DocumentBaseUrl` (ECS) | existing `BaseUrlMaintainer` → `GetDocumentBaseURI` (unchanged). Its fallback base URL reads `about:blank`, not the document's URL: existing slot `#11-document-base-url` (R-7 appends that `DocumentUrl` now exists) |
  | focused area (HTML §6.6) | `activeElement`, `hasFocus` | `ElementState::FOCUS` (ECS), read by `elidex_dom_api::focus::current_focus(dom, this)` | existing → unchanged; `hasFocus` also reads the browsing context (row below) |
  | style sheets (CSSOM) | `styleSheets` | ECS (the document's sheets, collected by `collect_document_stylesheets`) | existing → unchanged |
  | browsing context (HTML §7.3.2) | `defaultView`, `cookie` (averse), `referrer`, `hidden`, `visibilityState`, `hasFocus`, `getSelection` | `HostData::document` (per-VM) | the predicate (D-4) reads it; storage home: existing slot `#11-browsing-context-state-ecs-components` (B1) |
  | visibility state (HTML §6.2) | `hidden`, `visibilityState` | bound: `HostData::is_tab_hidden` (per-VM, the traversable's); others: none | D-6: non-bound → initial `"hidden"` (never updated without a browsing context); bound unchanged; home as the row above |
  | the document's selection (Selection API) | `getSelection` | `HostData::selection_instance` (per-VM singleton) | D-5: non-bound → `null`; bound unchanged; home as the browsing-context row |
  | referrer (HTML §3.1.4) | `referrer` | `NavigationState::referrer` (per-VM), read only for the bound document | **slot proposal R-8** `#11-document-referrer-component` |
  | current document readiness (HTML §3.1.5) | `readyState` | VM constant; `SessionCore::document_ready_state` (session, shell-written) for the bound document | **slot proposal R-4** `#11-document-readiness-one-source` |
  | mode (DOM §4.5) | `compatMode` | none (constant "CSS1Compat") | **slot proposal R-9** `#11-document-mode` |
  | custom element registry (DOM §4.5) | `createElement` (and C0c's `adoptNode`) | the VM's registry, used whatever `this` is (`document.rs:310-355`; C0c memo T15r) | existing slot `#11-document-null-registry-adopt-reset` (scope (1): null for DOMParser documents and the inert template document) — no C1-pre change |
  | cookies (the jar) | `cookie` | `HostData::cookie_jar` | not a per-entity fact (CLAUDE.md side-store exception (b)) |

- **P-12 `DocumentBaseUrl` writers and readers** (PM 338; property: a write or read of the component):
  `git grep -n 'DocumentBaseUrl' -- 'crates/*.rs' | /usr/bin/grep -v "$NT"` plus the callers of its two functions,
  `git grep -n 'recompute_document_base(\|initialize_base_url_for_document(\|document_base_url(' -- 'crates/*.rs' | /usr/bin/grep -v "$NT"`.
  At `fab44a1d`: **writers** — `crates/core/elidex-ecs/src/dom/mod.rs:414` (spawn, `about:blank`);
  `crates/dom/elidex-dom-api/src/element/document_base.rs:154` (the only maintainer insert, in `recompute_document_base`),
  reached from `initialize_base_url_for_document` (`:202-205`; callers `BaseUrlMaintainer::initialize_from_tree` `:285`,
  i.e. the bind path's D-31 pass `crates/script/elidex-js/src/vm/vm_api.rs:305-313`, with `about_blank_url()`, and
  DOMParser `inert_document.rs:196` with the caller URL) and from the maintainer's Insert / Remove / AttributeChange
  arms (`:300`, `:314`, `:366`, `:396`, all `about_blank_url()`). **Readers** — `document_base_url` (`:50-62`, the only
  read of the component outside the maintainer's own `unchanged` check `:147-150`), called by `GetDocumentBaseURI`
  (`crates/dom/elidex-dom-api/src/char_data/document_props.rs:51`), `GetNodeBaseURI` (`:76`) and
  `effective_base_url` (`crates/dom/elidex-dom-api/src/element/href_accessor.rs:51`). Not cloned (`tree_clone.rs:31`,
  "out-of-domain"). C1-pre adds no writer or reader to this population (D-17); it changes the value DOMParser's call
  passes only in where it comes from (D-14).

  Facts no Document member reads at `fab44a1d` (encoding, content type, origin, type, allow declarative shadow roots,
  about base URL) are outside the population; a member that starts reading one must add a row.

### §5.2 Existing tests and docstrings whose expectation changes

P-6's 10 sites (descriptor lookups move to the prototype; values unchanged); P-7's two tests (their observable is kept,
their mechanism text changes); `tests_document_methods.rs:565-596` (`document.URL` after a commit: the fixture writes
the component the way the shell does, `commit_document_url` or `EcsDom::set_document_url`, instead of
`navigation.set_current_url`); the dom-api unit `crates/dom/elidex-dom-api/src/char_data/tests/document_props.rs:8`
(`GetDocumentUrl` on a fresh document → `about:blank` from the component; plus a set-then-read case);
`crates/shell/elidex-shell/src/content_fragment_nav_tests.rs:92-101` gains the component assertion (W17). Every VM test
that sets `current_url` and then reads `document.URL` / `documentURI` / `cookie` is re-derived at the cut:
`/usr/bin/grep -rln 'set_current_url\|navigation\.current_url' crates/script/elidex-js/src/vm/tests crates/script/elidex-js/src/tests_*.rs | xargs /usr/bin/grep -ln 'document\.URL\|documentURI\|document\.cookie'`.
Docstrings: §4.1 item 6 and §4.3's list.

### §5.3 Touched files

Line counts `wc -l` at `fab44a1d`; re-measured at the cut (§9 item 1).

| file | lines | change | 1000-line decision |
|---|---|---|---|
| `crates/script/elidex-js/src/vm/host/document.rs` | 1197 → ≈ 537 after D-14 (b) | installer → `register_document_prototype`; `URL` / `readyState` natives if they stay here | split by C0 D-14 (b) first (D-10) |
| `vm/host/document_accessors.rs` (NEW: D-14 (b)) | ≈ 516 | the predicate, class-B natives, `native_document_get_selection` (NEW), `compatMode`, cookie URL reads | < 1000 |
| `crates/script/elidex-js/src/vm/globals.rs` | 1533 | one registration call | split first by C1-pre0 (D-11) |
| `crates/script/elidex-js/src/vm/mod.rs` | 2782 | one field | not split (D-11) |
| `crates/script/elidex-js/src/vm/init.rs` | 858 | one initializer | < 1000 |
| `crates/script/elidex-js/src/vm/host/elements.rs` | 425 | one arm, comment | < 1000 |
| `crates/script/elidex-js/src/vm/host_data/engine_feature/entity_kind.rs` | 201 | `PrototypeKind::Document` | < 1000 |
| `crates/script/elidex-js/src/vm/host_data/mod.rs` | 729 | delete the set | < 1000 |
| `crates/script/elidex-js/src/vm/host/globals.rs` | 57 | drop the install call | < 1000 |
| `crates/script/elidex-js/src/vm/host/dom_parser.rs` | 410 | drop the install; relevantDocument's URL (D-14) | < 1000 |
| `crates/script/elidex-js/src/vm/host/node_methods_extras.rs` | 293 | drop the hook | < 1000 |
| `crates/script/elidex-js/src/vm/host/history.rs` | (measure) | `update_document_url` at `:248` (D-16) | re-measured; < 1000 expected |
| `crates/script/elidex-js/src/vm/host/navigation.rs` | (measure) | `update_document_url` beside `set_current_url` | re-measured |
| `crates/script/elidex-js/src/vm/host/media_query.rs` | — | one comment | (cited line only) |
| `crates/core/elidex-ecs/src/components.rs` | 822 | `DocumentUrl` (≈ +12) | < 1000 |
| `crates/core/elidex-ecs/src/dom/mod.rs` | 926 | spawn default; `document_url` / `set_document_url` (≈ +25) | < 1000 (≈ 951); if the base is ≥ 975, the Document-entity helpers (`:380-470`) move out as a standalone pure-move prereq first |
| `crates/core/elidex-ecs/src/lib.rs` | — | re-export; doc `:15-17` | (small) |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | copy for a Document source; policy-table row | < 1000 |
| `crates/dom/elidex-form/src/inert_document.rs` | 287 | write the caller URL (D-14); comment `:182-197` | < 1000 |
| `crates/dom/elidex-dom-api/src/char_data/document_props.rs` + its tests | (small) | `GetDocumentUrl` reads the component | < 1000 |
| `crates/script/elidex-script-session/src/engine.rs` | (measure) | `commit_document_url` (NEW); `set_current_url` doc | re-measured |
| `crates/shell/elidex-shell/src/pipeline.rs` | 975 | `:258` → `commit_document_url` (net 0 lines) | < 1000 (any growth here moves the call's helper out first) |
| `crates/shell/elidex-shell/src/app/navigation.rs` 690, `crates/shell/elidex-shell/src/content/navigation.rs` 752, `app_test_support.rs` 317, `content_test_support.rs` 408 | as listed | call sites (net ≈ 0) | < 1000 |
| `crates/script/elidex-js/src/vm/tests/tests_document_wrapper.rs` (NEW) | — | W cells | new file |
| `crates/script/elidex-js/src/vm/tests/tests_document_methods.rs` | 997 | two tests rewritten in place, net ≤ 0 lines | < 1000; if a rewrite needs more lines it moves to the new file instead |
| `tests_cookie_referrer.rs` 900, `tests_document_members.rs` 200, `tests_cssom_sheet.rs` 983, `tests_html_element_proto.rs` 647, `tests_clone_node.rs` 296 (in `crates/script/elidex-js/src/vm/tests/`) | as listed | P-6 / P-7 rewrites, net 0 lines | < 1000 |
| `crates/shell/elidex-shell/src/content_fragment_nav_tests.rs` | 443 | W17 legs | < 1000 |
| `crates/core/elidex-ecs/src/dom/tests/document_url.rs` (NEW) | — | U cells | new file |

### §5.4 Named risks (and the observable change each is)

- **R-1 own-property enumeration of `document`** → empty (X1, W2).
- **R-2 non-bound Documents' visibility and selection** → `hidden` `true`, `visibilityState` `"hidden"`,
  `getSelection()` `null` for DOMParser and cloned Documents (W6), gated on §1.1.
- **R-3 brand checks** → `URL` / `documentURI` / `readyState` getters called on a non-Document platform object throw
  `TypeError` (W9, W10).
- **R-4 a Document wrapper allocated while unbound** would keep `Node.prototype` (B1×B2). Predicted unreachable (Q-S3);
  `debug_assert!` (D-3).
- **R-5 registration order** — a misplaced `register_document_prototype` panics at VM construction (loud); Q-S1.
- **R-6 a missed URL writer** — a commit path that bypasses `commit_document_url` leaves `document.URL` at the
  previous URL while `location.href` moves (mirror divergence). Guarded per path by W17's legs and P-9's ratchet.
- **R-7 non-bound URLs change** → DOMParser documents keep their creation URL across `pushState` (W18), clones their
  source's (W19), I `about:blank` (W16): HEAD reports the page's current URL for all three.

## §6. Test plan

Columns: *HEAD* = `fab44a1d` (+ C1-pre0 and D-14 (b), both pure moves); *C1-pre* = spec. **Red-first**: every red cell
runs on the base before the change with its control green; the failures go in the PR description. Fixture shape (W1,
W15, W16): a second Document entity in the bound `EcsDom` made by `dom.create_document_node()` (the spawn C1's I will
use), not bound, with an element appended and that element exposed as a global through
`vm.inner.create_element_wrapper` + `vm.set_global` (`tests_custom_elements.rs:1184` idiom) — the shape C1's I has (a
Document in the page `EcsDom` reached through `ownerDocument`).

### §6.1 VM integration (`tests_document_wrapper.rs`, NEW)

| cell | kind | script | HEAD | C1-pre = spec |
|---|---|---|---|---|
| W1 | red (fixture) | `d = p.ownerDocument;` then separately `typeof d.createElement`, `d.defaultView === null`, `Object.getPrototypeOf(d) === Object.getPrototypeOf(document)`, `d.createElement('i').ownerDocument === d`, `d.body` | `"undefined"`, `false`, `false`, throws `TypeError`, `undefined` | `"function"`, `true`, `true`, `true`, `null` |
| W2 | red | `[Object.getOwnPropertyNames(document).length, 'createElement' in Object.getPrototypeOf(document)]` | `[N > 0, false]` | `[0, true]` |
| W3 | red (order transfer) | `Object.getOwnPropertyNames(Object.getPrototypeOf(document))` equals the list `Object.getOwnPropertyNames(document)` printed on the base (pinned as a literal) | `Node.prototype`'s names | the pinned list |
| W4 | guard | `p.ownerDocument === p.ownerDocument`; `dp.documentElement.parentNode === dp`; `document.documentElement.parentNode === document` | all `true` | all `true` |
| W5 | guard | `[Object.getPrototypeOf(dp), Object.getPrototypeOf(document.cloneNode(true))].every(x => x === Object.getPrototypeOf(document))`; `typeof dp.createElement`, `typeof document.cloneNode(true).createElement` | `true`; `"function"` ×2 | same |
| W6 | red (gated, D-6) | `[dp.hidden, dp.visibilityState, dp.getSelection()]`; the same for `document.cloneNode()` | `[false, "visible", Selection]` | `[true, "hidden", null]` |
| W7 | guard | `[dp.defaultView, dp.cookie, (dp.cookie = 'a=b', dp.cookie), dp.referrer, dp.hasFocus(), dp.readyState, dp.compatMode]` | `[null, "", "", "", false, "complete", "CSS1Compat"]` | same |
| W8 | guard (bound document unchanged) | `[document.defaultView === globalThis, document.hidden, document.visibilityState, document.getSelection() === getSelection(), document.compatMode, document.readyState]` | `[true, false, "visible", true, "CSS1Compat", "complete"]` | same |
| W9 | red | `Object.getOwnPropertyDescriptor(P, 'body').get.call(document.body)` with `P = Object.getPrototypeOf(document)` (HEAD: `P = document`) — message | HEAD's message (run on the base) | `TypeError` "Illegal invocation" |
| W10 | red | `Object.getOwnPropertyDescriptor(P, n).get.call(document.body)` for `n` in `URL`, `documentURI`, `readyState`; HEAD with `P = document` | returns a string | `TypeError` "Illegal invocation" ×3 |
| W11 | red (rewrite of `document_methods_install_per_document_entity`) | bind doc1, record `Object.getPrototypeOf(document)` in a global; unbind; bind doc2; `[typeof document.getElementById, Object.getPrototypeOf(document) === saved, document.hasOwnProperty('getElementById')]` | `["function", true, true]` | `["function", true, false]` |
| W12 | guard (gates) | `'getElementsByTagName' in document`; with `compat-webapi` + BrowserCompat `'cookie' in document`, in an App-mode VM `'cookie' in document` | `true`; `true`; `false` | same |
| W15 | red (fixture) | as W1, with `ElementState::FOCUS` set on the fixture element (Rust) and the tab visible: `d.hasFocus()` | `true` | `false` |
| W16 | red (fixture; PM 335 (3); I's shape) | `d = p.ownerDocument; [d.URL, d.documentURI]` on the generic path, the bound document at `https://example.com/a` (fixture commits it through `commit_document_url`) | `[undefined, undefined]` (no install; with D-1 alone: the page URL) | `["about:blank", "about:blank"]` |
| W18 | red | `u = dp.URL; history.pushState(null, '', '?x'); [u === 'https://example.com/a', dp.URL === u, document.URL.endsWith('?x')]` | `[true, false, true]` | `[true, true, true]` |
| W19 | red | `c = document.cloneNode(); history.pushState(null, '', '?y'); [c.URL === 'https://example.com/a', document.URL.endsWith('?y')]` | `[false, true]` | `[true, true]` |
| W21 | guard (D-17: URL ≠ base URL) | bound document at `https://example.com/a`, then `b = document.createElement('base'); b.href = 'https://cdn.example/x/'; document.head.append(b); [document.URL, document.baseURI]`; and `dp2 = new DOMParser().parseFromString('<base href="https://cdn.example/y/">', 'text/html'); [dp2.URL, dp2.baseURI]` | `[page URL, "https://cdn.example/x/"]`; `[page URL, "https://cdn.example/y/"]` | same (now `URL` from `DocumentUrl`, `baseURI` still from `DocumentBaseUrl`) |
| W20 | guard (compat-webapi) | cookie scoped by the document URL: commit `https://example.com/p/a`, `document.cookie = 'k=v; path=/p'`; `document.cookie.includes('k=v')`; on `dp`: `dp.cookie === ''` | `true`; `true` | same (now through the component) |

### §6.2 Shell integration (the main document's URL; PM 335 (3))

W17, in the shell's navigation suites (`content_fragment_nav_tests.rs` and its `app_*` / `content_history_*` siblings;
file per leg chosen at the cut, Q-S6). Each leg asserts the trio `pipeline.dom.document_url(pipeline.document)`,
`runtime.current_url()` and an eval of `document.URL`, all equal to the expected URL:

| leg | path (P-9 site) | expected | HEAD |
|---|---|---|---|
| W17a | initial load (`pipeline.rs:258`) | the loaded URL | `document.URL` = loaded URL (component absent) |
| W17b | fragment navigation (`content/navigation.rs:423`, `app/navigation.rs:139`) | `…#sec` | as HEAD for `document.URL`; component absent |
| W17c | cross-document navigation / reload / traversal (`content/navigation.rs:345`, `app/navigation.rs:346`) | the new URL, on the new pipeline's Document | as above |
| W17d | `history.pushState(null, '', '?p')` then the shell drain (`history.rs:248`, `content/navigation.rs:726`, `app/navigation.rs:662`) | `…?p` synchronously in the same script, and after the drain | as above |

At HEAD the `document.URL` legs pass (from `current_url`) and the component legs fail (absent); at C1-pre's head all
three agree. The component leg is the red one; the `document.URL` leg is the K-0 guard.

### §6.3 Unit (`crates/core/elidex-ecs/src/dom/tests/document_url.rs`, NEW, and the owning crates)

| cell | shape | expected |
|---|---|---|
| U1 | `create_document_node()` → `document_url` | `about:blank` |
| U2 | `clone_node_shallow` / `clone_subtree` of a Document with URL X | the copy's URL is X; the source unchanged |
| U3 | `set_document_url(doc, Y)`; read | Y; a non-Document → no write (returns false / `debug_assert!`) |
| U4 | elidex-form: `parse_into_inert_document(dom, markup, caller)` | the new Document's URL is `caller` |
| U5 | dom-api: `GetDocumentUrl` on a fresh document and after a set | `about:blank`, then the set value |
| U6 | elidex-script-session: `commit_document_url(driver, dom, doc, None)` with a recording driver | component `about:blank`; driver saw `None` |

### §6.4 Mutation guards

| mutation | killed by |
|---|---|
| no `PrototypeKind::Document` arm (Document → `Node.prototype`) | W1, W2, W5 |
| members installed on the prototype **and** on wrappers | W2, W11 |
| install sequence reordered | W3 |
| a class-B native keeps a private copy of the check / the predicate inverted | W6, W7, W8 |
| `hasFocus` without step 1 | W15 |
| Document `getSelection` delegates unconditionally | W6 |
| gate evaluated per wrapper / dropped | W12 |
| DOMParser or clone install left behind | P-2 ratchet, W11 |
| `URL` / `readyState` without brand check | W10 |
| `GetDocumentUrl` falls back to `current_url` (or the VM native kept) | W16, W18, W19 |
| `URL` reads `DocumentBaseUrl` (the reuse D-17 rejects) | W21 (both legs), W16 against a `<base>`-carrying variant |
| `create_document_node` without the default | U1, W16 |
| the cloner drops `DocumentUrl` | U2, W19 |
| DOMParser reads `current_url` instead of the bound document's component (both equal at parse) | W18 is blind to it: U4 plus the P-10 ratchet (0 `navigation.current_url` in `dom_parser.rs`) |
| one shell commit site left on `set_current_url` | that site's W17 leg, P-9 ratchet |
| `history.rs:248` updates only the mirror | W17d (synchronous leg), W18 |

### §6.5 Kept green

`tests_document_methods.rs`, `tests_document_members.rs`, `tests_dom_parser.rs`, `tests_clone_node.rs`,
`tests_cookie_referrer.rs`, `tests_cssom_sheet.rs`, `tests_html_element_proto.rs`, `tests_parent_node_mixin.rs`,
`tests_base_href.rs`, the history / location suites, the event-handler attribute suites, the shell navigation suites,
`crates/dom/elidex-dom-api/src/char_data/tests/document_props.rs`. Commands: `cargo test -p elidex-ecs --all-features`,
`-p elidex-dom-api`, `-p elidex-form`, `-p elidex-script-session`, `-p elidex-js`, `-p elidex-shell`, then `mise run ci`.

## §7. Out of scope and slots

| item | where | defect (stated, not the remedy) |
|---|---|---|
| `current_url` as a per-VM mirror of the bound document's URL, read by non-Document readers | `#11-document-base-url` (R-7) | `location`, `history`'s no-URL case, fetch / API base, origin, storage, workers read the mirror, not the Document's component |
| DOM node interface objects (`Node`, `Element`, `Document`, …; `HTMLDocument` alias) | no slot found (`/usr/bin/grep -n 'interface object' memory/project_open-defer-slots.md` → no slot row); R-6 | `x instanceof Document` throws `ReferenceError`; `new Document()` is unavailable |
| `[LegacyUnforgeable] location` on Document | not tracked here | `document.location` is absent |
| `[SameObject]` `forms` / `images` / `links` | not tracked here (live-collection family, B0 / B1 of the A1 gate) | each read allocates a new collection |
| referrer, readiness, mode | R-8, R-4, R-9 (slot proposals) | §5.1 P-11 rows |
| the base URL's dependency on the URL (fallback base URL step 2; *respond to base URL changes*), the about base URL, the base URL override | `#11-document-base-url` (R-7) | with no `<base href>`, `baseURI` and relative-URL resolution use `about:blank`, not the document's URL, and are not re-derived when the URL changes |
| browsing-context facts as ECS components | B1 / `#11-browsing-context-state-ecs-components` | the predicate's input is per-VM `HostData::document` |
| the per-Document custom element registry | `#11-document-null-registry-adopt-reset` | `I.createElement` / `dp.createElement` use the VM's registry |
| the *URL and history update steps* algorithm living in vm/host | not tracked here (pre-existing; `history.rs`) | the algorithm computing newURL is host code; C1-pre only replaces its write |
| `activeElement` for a Document without a navigable | §1.1 X7 (measurement only) | spec-silent step 1 |
| `document_receiver`'s silent fallback for non-platform receivers | pre-existing elidex policy | Web IDL would throw `TypeError` |
| multi-realm `Document.prototype` | B1 (H-1 below) | one prototype per VM |

Own deferrals: 3 slot proposals (R-4, R-8, R-9), each a pre-existing defect C1-pre does not widen; the other rows are
pre-existing or another unit's scope.

## §8. Edges, hand-offs and records

**Edges.**
- e-pre0: **C1-pre0 → C1-pre** (code predecessor; D-11).
- e-14b: **C0 D-14 (b) → C1-pre** (code predecessor; D-10). D-14 (b) also precedes C0c; whichever of C0c / C1-pre is
  cut first lands it.
- e-c0c: **C0c ↔ C1-pre**, either order (no semantic dependency). If C1-pre lands first, C0c's `adoptNode` entry lands
  on `Document.prototype` (R-3).
- c-pre: **C1-pre → C1** (semantic: C1's T5, T1 (with its URL leg), M3 need D-1, D-4 to D-9, D-13).
- e-dbu: **C1-pre → `#11-document-base-url`** (data: the program's document-URL facet now has its store; R-7).
- VM P4 (driver's check, re-done here): `vm-is-constructor-move` landed as #588 (`4fd85f4e`); its only file in common
  with C1-pre is `vm/mod.rs` (one `mod` line near the top vs one field near `:506`: rebase only).
  `vm-construct-isconstructor-check` and `vm-p4-slice-1a-plan` are docs-only (`git diff --stat
  $(git merge-base origin/main <branch>) <branch>`); slice 1aa cites `dom_bridge.rs:567` as an idiom only. C1-pre0
  touches `vm/globals.rs` and `vm/mod.rs` (`mod` lines): re-check both against open VM P4 branches at the cut.
- C0b (`node_document`, `NewNodeOwner`) edits `crates/core/elidex-ecs/src/dom/mod.rs` and `tree_clone.rs` too: C1-pre's
  additions there are re-measured on whichever lands second (§9 item 1).

**Hand-offs.**
- H-1 to B1: one `Document.prototype` per realm; `DocumentUrl` is per entity and needs nothing.
- H-2 to C1: §4.4; R-2.
- H-3 to C0c: R-3.

**Records (for the PM; exact texts).**
- **R-1 ledger `#11-bridge-document-post-install`**: "Owner: Layout lane (PM 328). Discharged by C1-pre (plan
  `docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`): every Document wrapper gets `Document.prototype` at
  `create_element_wrapper` (prototype selection by `NodeKind`, the bridge step the other node kinds already have); the
  three path-specific installs and `HostData::document_methods_installed` are deleted. Closes at C1-pre's landing."
- **R-2 C1 memo**: T1's second leg `I instanceof Document` → `Object.getPrototypeOf(I) === Object.getPrototypeOf(document)`
  (no `Document` interface object, H-9); T1 gains `I.URL === 'about:blank'` (PM 335 (3)); §4.1 / §4.8 / §5.3: the VM
  work is C1-pre's (edge c-pre); R-C5's precedent text per §4.4; Q-S10 answered by §5.1 P-4 / P-11; H-C10 lists both
  Document spawn sites (H-14).
- **R-3 C0c memo**: if C1-pre lands first, D-13's "installed as an own property" and H-c6 read "installed on
  `Document.prototype` (C1-pre D-1)"; the table entry is unchanged.
- **R-4 slot proposal `#11-document-readiness-one-source`**: **Defect**: the bound document's `readyState` has two
  answers — the VM constant `"complete"` (`document.rs:492-501`) and the dom-api `GetReadyState`, which reads the
  shell-driven `SessionCore::document_ready_state` (`pipeline.rs:488`) and which the VM never calls; a script running
  before load completes reads `"complete"`. **Why deferred**: changing it changes the bound document's observable
  readiness during load, outside PM 328 / 335's per-entity scope; the per-Document home of readiness belongs with the
  browsing-context cluster. **Trigger**: a PR that runs scripts before the shell marks the document complete, or B1.
  **Re-eval**: 2027-01-31. **Owner**: Layout (default; the PM may assign).
- **R-5 umbrella §1.2**: add C1-pre's row before C1's: "| **C1-pre** (owner: Layout lane; PM 328 / 335) | slot
  `#11-bridge-document-post-install`: every Document wrapper is a Document; the document URL as a per-Document
  component | C0 D-14 (b) and C1-pre0 landed |".
- **R-6 PM**: the DOM node interface-object gap (§7 row 2), class and evidence, for the PM to register or decline.
- **R-7 ledger `#11-document-base-url` (append)**: "C1-pre (PM 335) lands `DocumentUrl`, the document URL as one
  component on every Document (writers: spawn default, clone, DOMParser, `commit_document_url`, the URL and history
  update steps; readers: `URL`, `documentURI`, `cookie`, DOMParser). `navigation.current_url` stays as a per-VM mirror
  written by the same functions; this program's document-URL facet is now to re-point the mirror's readers (population:
  `git grep -n 'navigation\.current_url\|\.current_url\b' -- 'crates/*.rs'`, less tests and C1-pre's own) — the
  fallback base URL included — and delete the mirror for Window VMs. The closed `#11-document-url-real-navigation`
  mentions in `crates/` are re-pointed by C1-pre. `DocumentBaseUrl` stays the derived document base URL (C1-pre D-17,
  HTML §2.4.3); this program's remaining base work includes making `BaseUrlMaintainer`'s fallback read the document's
  `DocumentUrl` (the *fallback base URL* step 2) instead of `about_blank_url()` (`document_base.rs:285`, `:300`, `:366`,
  `:396`) and re-deriving the cache when `DocumentUrl` changes (*set the URL* step 2, *respond to base URL changes*)."
- **R-8 slot proposal `#11-document-referrer-component`**: **Defect**: a Document's referrer (HTML §3.1.4, set by
  *create and initialize a Document object*) is held per-VM (`NavigationState::referrer`, set by the shell through
  `Vm::set_navigation_referrer`, `crates/script/elidex-js/src/vm/vm_api.rs:186`), not on the Document entity — the
  side-store→component rule's case. Observable today: none (only the bound document is read; others read `""`, their
  spec value). **Why deferred**: PM 335 bounds C1-pre to the URL fact; the referrer's writer is the navigation
  pipeline, the same cluster as origin. **Trigger**: a second Document with a non-empty referrer (iframes / B1) or
  `#11-browsing-context-state-ecs-components` starting. **Re-eval**: 2027-01-31. **Owner**: Layout (default).
- **R-9 slot proposal `#11-document-mode`**: **Defect**: a Document's mode (DOM §4.5: no-quirks / quirks /
  limited-quirks) is not stored; `compatMode` answers `"CSS1Compat"` for every Document, including one whose doctype
  puts it in quirks mode, and layout reads no mode. **Why deferred**: neither parser tier computes the mode at
  `fab44a1d`; giving it a store without a writer would be a stub. **Trigger**: a PR that implements quirks-mode
  detection in either parser tier, or quirks-mode layout. **Re-eval**: 2027-01-31. **Owner**: L1/L2 (PM assigns).

## §9. Definition of done

1. Coordinates re-measured at the cut (C1-pre0 and D-14 (b) landed; C0b if landed); §5.1 populations re-run and
   recorded; open VM P4 branches re-checked for `vm/globals.rs`, `vm/mod.rs`, `vm/init.rs`, `vm/host/elements.rs`,
   `vm/host_data/**`.
2. C1-pre0 landed as a pure move (no line changed outside moved blocks, `mod` and `use` lines); `vm/globals.rs` < 1000
   or the residue is `register_globals` alone (D-11).
3. D-1 to D-17 implemented; P-1's DOM-node allocator count 1; P-2 0 hits; P-5 0 among Document members; P-9's ratchet
   holds; P-10 0 hits.
4. §1.1 filled before the implementation branch is cut; W6 asserted only if X4 / X5 agree, else the PM's decision.
5. §6.1 to §6.3 cells present; every red cell run red on the base with its control green, recorded in the PR
   description.
6. §5.2 rewrites done; `git grep -n 'install_document_methods\|document_methods_installed\|bridge-document-post-install\|document-url-real-navigation' -- crates` → 0.
7. `cargo test` for the six crates of §6.5 and `mise run ci` green.
8. `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-11-eh-attrs-c1-pre-document-wrapper.md`
   exits 0 (run, not trusted).
9. The PM has applied R-1, R-3, R-5, R-7 (or recorded why not), registered or declined R-4, R-8, R-9 and R-6, and
   decided O-1 to O-3.

## §10. Revision history

*Rev 1* (2026-11-04): first draft (PM 328 brief), with PM 335 applied before the first commit: D-9 rewritten (the
component instead of "value per O-1"), D-13 to D-16 added, §1 axis B7, §2.2 to 21 pairs, §4.3, §5.1 P-8 to P-11, §6.1
W16 / W18 to W20, §6.2, §6.3, R-4 / R-7 to R-9; the draft's O-1 (URL home) is decided by PM 335. *Rev 1.1*
(2026-11-04, PM 338): D-17 (`DocumentUrl` vs the existing `DocumentBaseUrl`: the base URL is derived from the URL,
HTML §2.4.3, so the existing component is not reused), H-18, §5.1 P-12 (its writers and readers), the §3 row HTML
§2.4.3, D-14's fallback read-back, W21, the §6.4 row, the §7 row, R-7's addendum.

## §11. Open questions

**For the PM / user.**
- **O-1 Ratification of PM 335's URL home.** The IBP umbrella records that "either home answer for the document URL is
  the user's to ratify" (`docs/plans/2026-09-inline-box-predicate.md:1659`), and the user's 2026-09-28 ruling kept
  `current_url` per-VM until B1. C1-pre keeps `current_url` (as a mirror) and gives the Document URL a component home
  — does the PM consider that within PM 335's authority, or does it go to the user? This memo proceeds on PM 335.
- **O-2 C1-pre0 (D-11).** `vm/globals.rs` is the VM core lane's file (driver absent): confirm the split and its owner
  (proposed: Layout lane, as C1-pre's prerequisite).
- **O-3 Order with C0c (e-c0c).** Either; the first cut takes D-14 (b). The PM picks if both are ready together.

**For plan-review.**
- **Q-1** D-16 writes the component from vm/host (`history.rs`) through `EcsDom::set_document_url` — a direct `EcsDom`
  write of a value the host's existing history algorithm computed. Is that within the Layering mandate's marshalling
  allowance, or must the write go through a session / dom-api entry? (This memo's reading: a component store of an
  already-computed value, no new algorithm; the algorithm's location is §7's row.)

**Scout questions** (facts needed at the cut; not read here).
- **Q-S1** In `register_globals`, the earliest point after which `install_event_handler_attrs`,
  `install_parent_node_mixin` and the `FACTORIES` natives can be installed on an object (prediction: after
  `register_node_prototype` `:215`).
- **Q-S2** Any non-test Rust code that reads a Document wrapper's **own** properties? Prediction: none
  (`natives_promise.rs:526` only allocates a wrapper as an event target).
- **Q-S3** Can a Document entity be wrapped while `HostData` is unbound (R-4)? Prediction: no.
- **Q-S4** Is the fragment-parse throwaway Document (`teardown.rs:301`) ever reachable from script? Prediction: no.
- **Q-S5** Is `HostDriver` used as `dyn HostDriver` or generically at the shell sites (shapes `commit_document_url`'s
  signature)? And does any non-shell embedder (wasm runtime, `elidex-navigation`) commit a document URL? Prediction:
  only the shell (`git grep -n '\.set_current_url(' -- crates`).
- **Q-S6** Which shell test suites drive each P-9 site (W17 legs), and does a harness expose `pipeline.dom` and
  `pipeline.document` to assertions (`content_fragment_nav_tests.rs:92-101` reads `state.pipeline`)?
