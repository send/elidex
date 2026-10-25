# EH-attrs-at-creation — event-handler content attributes processed once, at the element's creation (per-PR plan-memo)

Standalone prerequisite work, label **`EH-attrs-at-creation`** (rev 1's label `EH-insert-once` and file
`2026-10-event-handler-insert-once.md` are retired: rev 1's contract was "delivered at first insertion", and this rev's
contract is "processed at creation", §0.5). Owner: Layout lane. It sits **outside** the IBP-sandbox program and is not
part of that umbrella's DAG (PM decision 17 (i)). IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`,
rev 3, branch `layout-ibp-sandbox-handler-gates`) depends on part of it (§1.4): N1's C8b(iv) needs a compiled handler to
survive a move into a DOMParser document, and HEAD turns it raw again on every move (§0.2 H1). The branch keeps its
name `layout-eh-insert-once` (not pushed).

**Rev 2** (2026-10-25). Folds PM decision 18 and the PM's three conditions on G1. (1) A landing-order comparison (§1)
decides that the write-time alternative (rev 1's R8, "the attribute change steps run when the attribute is written,
creation included") can land with no interim mechanism; rev 1's `HandlerAttrDelivery` Pending→Delivered state is
withdrawn (§4.3 R-D3), and the label and file name follow the new contract. (2) G1 (initial-HTML inline handlers are
never wired) is in scope, with end-to-end pins through `build_pipeline_from_loaded` → bind, red-first (§6.2). G2 (shadow
trees) and G5 (reads never deliver) are in scope by the inclusion criterion (§7). (3) The invariant axes are recounted
(§2.0); the count is 9, so whether to split is open (§10 O1), with a recommendation. (4) cloneNode references are
corrected to the real path; importNode/adoptNode cite `#11-cross-document-adopt-on-insert`; the silent-writer census is
reproducible from this memo (§5.0). Rev 1 (`28efbb67`): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fd6955e8`** coordinate (origin/main at the branch cut). Every code reference comes from
`/usr/bin/grep -n` (never the default ugrep) or `sed -n` over that tree, run on 2026-10-25 by this rev's author. Every
§ number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`) run on
2026-10-25. N1's memo is cited at its rev 3 head `0f99538d` (read-only here).

### §0.2 Premises re-measured

| id | claim | command | result |
|---|---|---|---|
| H1 | The Insert arm re-delivers baked `on*` attributes on **every** `MutationEvent::Insert` | `sed -n 373,386p` and `sed -n 427,463p crates/script/elidex-script-session/src/event_handler_consumer.rs` | **true.** `handle` (`:373`) routes `Insert` to `handle_insert` (`:427`), which walks `node` plus `traverse_descendants` (`:432`), reads every `Attributes` entry named in `EVENT_HANDLER_ATTRS` (`:61`) and calls `set_inline_handler` (`:468`) → `set_uncompiled`. No state check |
| H2 | `MutationEvent::Insert` fires for every insertion, a move included, except under a shadow root | `sed -n 282,298p crates/core/elidex-ecs/src/dom/tree/mutation.rs` | **true.** `fire_after_insert` (`:282`) returns early when `node` or `parent` is a shadow root (`:289`), otherwise dispatches `Insert` |
| H3 | `set_uncompiled` makes a compiled handler raw again | `sed -n 521,581p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `ensure_event_handler_current` (`:521`) takes the source, compiles, and overwrites the callable: a new function identity, and an IDL-set callable is replaced |
| H4 | The IDL setter supersedes inline source | `sed -n 664,692p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `activate_event_handler` (`:664`) calls `clear_uncompiled`. A later Insert undoes it (H1) |
| H5 | The DOMParser build suppresses every consumer | `sed -n 110,125p` and `sed -n 192,200p crates/dom/elidex-form/src/inert_document.rs` | **true.** `take_mutation_dispatcher` `:112`, restored `:198-199`. Untouched DOMParser nodes carry no `EventListeners` entry (N1 memo H-P13) |
| H6 | Fragment parses are built dispatch-suppressed and fire Insert at caller placement | `sed -n 318,333p crates/core/elidex-ecs/src/dom/tree/teardown.rs` | **true.** `begin_detached_fragment` (`:318`) takes the dispatcher (`:325`) and builds in a throwaway document (`:326`) |
| H7 | Every script attribute write reaches the AttributeChange arm while bound | §5.0 census | **true** for bound writes; the only non-dispatching writer that can name an `on*` attribute is a creation bake (§5.0 result (a)) |
| H8 | Clone copies an explicit component set only | `sed -n 13,36p crates/core/elidex-ecs/src/dom/tree_clone.rs` | **true.** The clone-policy table (`:19-34`) enumerates; `EventListeners` is in the "deliberate non-copy" row (`:30`) |
| H9 | Production parses the document **before** the VM binds, and bind seeds only the base URL (G1) | `sed -n 839,880p crates/shell/elidex-shell/src/pipeline.rs`; `sed -n 290,320p crates/script/elidex-js/src/vm/vm_api.rs`; `sed -n 113,137p crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | **true.** `build_pipeline_from_loaded` (`:839`) destructures an already-parsed `dom` (`:850-858`) and calls `run_scripts_and_finalize` (`:189`), whose bracket (`:355`) binds. `Vm::bind` builds `ConsumerDispatcher::new` (`vm_api.rs:292`) and calls `initialize_consumers` (`:313`) before installing it (`:314`). `initialize_consumers` (`consumer_dispatcher.rs:135-137`) calls only `base_url.initialize_from_tree`; its doc (`:113-134`) says other consumers need no init. Nothing wires a parsed `<button onclick>` until that node is inserted after bind |
| H10 | Light-tree walks skip shadow roots; a shadow-inclusive walker exists | `sed -n 294,302p` and `sed -n 15,60p crates/core/elidex-ecs/src/dom/tree/walkers.rs` | **true.** `traverse_descendants` (`:294`) skips `ShadowRoot` entities. `for_each_shadow_inclusive_descendant` (`:33`) visits light then shadow children (`:47-57`) and does **not** visit template contents (it follows `get_shadow_root` and `children_iter` only) |
| H11 | `remove_listener_and_prune_back_ref` keeps the `EventListeners` entry | `sed -n 49,61p crates/script/elidex-js/src/vm/inner.rs` | **true** |
| H12 | Bind/unbind keeps the listener store and the Window entity | `sed -n 213,230p crates/script/elidex-js/src/vm/vm_api.rs` | **true.** The first bind creates the Window (`create_window_root`, `:227`); later binds reuse it. `HostData` (owning `listener_store`) persists |
| H13 | **Every** bind rebuilds the dispatcher and re-runs `initialize_consumers`; every unbind removes it | `sed -n 28,36p crates/script/elidex-js/src/vm/lifecycle.rs`; `/usr/bin/grep -rn --include='*.rs' 'with_bound(' crates/shell \| wc -l` | **true.** Unbind calls `clear_mutation_dispatcher` (`lifecycle.rs:34`); bind installs a fresh one (`vm_api.rs:292-314`). The shell has 15 `with_bound(` call sites (initial scripts `pipeline.rs:355`, per-event brackets `lib.rs:299-529`, `pipeline.rs:133`, `:165`). So a bind-time delivery pass runs once per bracket and **must be idempotent per element** |
| H14 | At `initialize_consumers` the Window entity exists | `sed -n 213,230p` and `:313` of `vm_api.rs` | **true.** `we` is resolved or created (`:222-229`) before the dispatcher is built and initialised. `resolve_handler_target` (`event_handler_consumer.rs:328-334`) finds it through `dom.window_entity()` |
| H15 | Template contents are owned by the template's own document, and the clause-(b) composition does not suppress them | `sed -n 539,580p` and `sed -n 820,856p crates/core/elidex-ecs/src/dom/mod.rs`; `sed -n 95,113p crates/script/elidex-script-session/src/scripting.rs` | **true.** `attach_template_contents` (`mod.rs:560`) creates the fragment with `owner_doc` = the template's document (C2 approximation, `:551-557`; slot `#11-template-contents-owner-document`). For a node under that fragment, `scripting_disabled_for_platform_object` takes the composed root (`scripting.rs:104`), which is the fragment, not a Document, so it falls back to `owner_document` (`:108`), which follows the fragment's `AssociatedDocument` (`mod.rs:851-853`) to the page document: not suppressed. The doc comment names this false negative (`scripting.rs:59-60`) |
| H16 | srcdoc iframes do **not** go through `build_pipeline_from_loaded` | `sed -n 60,81p` and `/usr/bin/grep -n 'build_pipeline' crates/shell/elidex-shell/src/content/iframe/load.rs` | **true.** srcdoc / about:blank → `build_iframe_pipeline` → `build_pipeline_interactive_shared` (`load.rs:471`). `build_pipeline_from_loaded` is used by the same-origin URL path (`:221`) and the out-of-process path (`:327`). All three reach `run_scripts_and_finalize` |
| H17 | cloneNode's path | `sed -n 201,215p crates/script/elidex-js/src/vm/host/node_methods_extras.rs`; `/usr/bin/grep -n 'clone_node_with_shadow_honor\|clone_node_shallow' crates/dom/elidex-dom-api/src/node_methods/clone.rs`; `/usr/bin/grep -n 'fn clone_node_shallow' crates/core/elidex-ecs/src/dom/tree_clone.rs` | VM `native_node_clone_node` (`node_methods_extras.rs:201`) → `invoke_dom_api("cloneNode")` → `elidex-dom-api` `node_methods/clone.rs:154` → `clone_node_with_shadow_honor` (`:227`) → `EcsDom::clone_node_shallow` (`:323`; `tree_clone.rs:251`) → `clone_node_shallow_unchecked` (`tree_clone.rs:270`), whose silent `Attributes` copy is `builder.add(attrs)` at `tree_clone.rs:368`. The template post-pass attaches a fresh fragment (`clone.rs:422`) |

### §0.3 Premise errors (reported)

- **P1: `importNode` / `adoptNode` do not exist** in elidex (`/usr/bin/grep -rn -i --include='*.rs' -E
  'importnode|adoptnode' crates` → nothing). Insertion adoption itself is slot `#11-cross-document-adopt-on-insert`. This
  memo analyses only elidex's internal adopt, `EcsDom::adopt_subtree` (`teardown.rs:285`), used by fragment teardown.
- **P2: the Window-reflecting body set is not delegated** (G4). `onblur` `:68`, `onerror` `:90`, `onfocus` `:91`,
  `onload` `:99`, `onresize` `:125`, `onscroll` `:126` are `HandlerScope::Global`, so `resolve_handler_target` (`:328`)
  keeps them on the body. Merged by the PM into slot `#11-event-handler-target-determination` (facet 4). Not here.
- **P3: a docstring overstates a scripting rule** (`inert_document.rs:78-84`): the attribute change steps have no
  scripting condition; *get the current value of the event handler* step 3.2 nulls a raw value. The correction belongs to
  the slice that touches that file (slice b, §1.4); slice a does not touch it (MINOR until then).
- **P4: `EcsDom::clone_attributes` is not the cloneNode writer** (`tree_clone.rs:57`; one caller, the test
  `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`). The cloneNode path is H17.
- **P5 (brief, G1 trace): the iframe coordinates name URL-loaded iframes only.** `load.rs:221` / `:327` are the
  same-origin-URL and out-of-process paths; srcdoc iframes are built by `build_pipeline_interactive_shared` (H16). The
  root cause is the same for all three (H9), and §6.2 pins one `build_pipeline_from_loaded` iframe path and the srcdoc
  path.
- **P6 (brief, G1 trace): `vm_api.rs:312-314`.** `ConsumerDispatcher::new` is at `:292`; `initialize_consumers` at
  `:313`, the install at `:314`.
- **P7 (brief, cloneNode): `tree_clone.rs:368` is a line inside `clone_node_shallow_unchecked`** (defined at `:270`),
  not its definition (H17).
- **P8 (brief, R8 reasons): there is a third R8 blocker.** Besides (a) DOMParser raw entries and (b) the DOMParser
  `<body>` → page Window, R8 makes template contents raw at parse, and HEAD cannot read them as null (H15). §1.2 (c).
- **P9 (brief, reason (a)): H-P13 need not be rewritten.** H-P13 is a statement about N1's base. It stays true if this
  work's DOMParser processing lands after N1 (§1.2 (a)).

### §0.4 Decisions

- **D-1: once-key.** `HandlerAttrsProcessed` (NEW), a unit marker component on the **origin** element (never on the
  handler's target, the Window for `<body>` delegation). Meaning: *this element carries no handler content attribute
  whose creation-time value is still unprocessed.* Sparse: inserted only on an element that had a handler content
  attribute when processed, or that received a handler write (D-4).
- **D-2: processing.** One engine-independent function, `process_handler_content_attrs` (NEW), runs the attribute
  change steps (WHATWG HTML §8.1.8.1) for every handler content attribute E carries, at most once per element (D-1),
  subject to the session's `SpecLevelPolicy`, with the target from `resolve_handler_target`.
  `process_handler_content_attrs_in_subtree` (NEW) applies it over a shadow-including subtree (H10).
- **D-3: timing.** E's creation-time handler content attributes are processed at the **earliest consumer sighting** of
  E: E's creation, when a consumer exists then (slice b); the first bind of E's DOM, for elements created before any
  consumer existed (G1; slice a); E's first handler write (D-4); E's first insertion (fallback; slice a's primary point
  for post-bind creations until slice b). Insertion never re-processes (D-1).
- **D-4: writers.** A handler write (the AttributeChange arm, and both VM element setters) first ensures E is
  processed, then writes. Slice a restricts this to "E has no *other* deliverable handler content attribute" (then the
  ensure reduces to inserting the marker); slice b removes the restriction (§1.4).
- **D-5: bind.** `EventHandlerAttributeConsumer::initialize_from_tree` (NEW) processes the bound document's
  shadow-including tree; `ConsumerDispatcher::initialize_consumers` calls it. It is idempotent per element (D-1), which
  H13 makes mandatory.
- **D-6: VM surface.** The two element setters (`native_event_handler_set` `:344`, `native_body_weh_set` `:459`) call
  `note_handler_write` (NEW) before `activate_event_handler`. Marshalling only (Layering mandate).

### §0.5 Contract (single citable statement)

For an element E, "creation-time attributes" are the `Attributes` written without `MutationEvent::AttributeChange` at
E's creation (the four parser bakes and the clone copy, §5.0 (a)). "Handler state" is the `EventListeners` entry for an
event type on E's resolved target, plus its stored callable. "Processed" = the attribute change steps have run for E's
creation-time handler content attributes (D-1).

- **K-1, once.** E's creation-time handler content attributes are processed at most once. After that, no insertion,
  move, or rebind writes handler state derived from them. A compiled callable keeps its identity across moves and
  rebinds, in the live document and into a DOMParser document alike.
- **K-2, an AttributeChange write wins.** Processing never overwrites a handler written by `setAttribute` /
  `removeAttribute` of `on<T>` on E. Its value survives, including a callable compiled from it.
- **K-3, an IDL write wins.** Processing never overwrites a callable or null assigned through `E.on<T>` (or a `<body>`'s
  delegated WindowEventHandlers accessor), whatever wrote E's `Attributes` silently.
- **K-4, timing.** For every creation path that runs while a consumer exists, processing happens at creation, before
  any script can observe E. For elements created with no consumer (the pre-bind parse), it happens at the first bind,
  before that bind's first script. Processing writes the internal raw uncompiled handler on the resolved target.
- **K-5, inert documents read null.** A raw value that processing writes on a node whose node document has a null
  browsing context (a DOMParser document, template contents) reads as null and never runs (via N1's compile gate and
  `#11-template-contents-owner-document`).
- **K-6, bind completeness (G1).** At every bind, every element of the bound document's shadow-including tree is
  processed before any script of that bind runs.

Slices (§1.4): slice a delivers K-1, K-6, and K-2 / K-3 for every element with at most one deliverable handler
attribute; slice b delivers K-2 / K-3 for all elements, K-4's creation arm, and K-5. Red cells: K-1 C1–C3, C8, U1, U2,
E3; K-2 C4, C11, U5; K-3 C5, U5; K-4 E1, E2, U7; K-6 E1–E5, U7, U8; K-5 slice b (§6.3).

## §1. Landing-order comparison (PM decision 18 (1), with G1)

### §1.1 The question

Rev 1 built an interim (`Pending { superseded }` → `Delivered` at first insertion) and named R8 (the change steps at
write time, creation included) as the end state. CLAUDE.md "One issue, one way" forbids landing an interim that a later
PR deletes. So: is there a landing order under which R8 lands with no interim?

### §1.2 Each blocker, its removal, and the evidence

R8's blockers are exposures that R8 creates on populations whose spec outcome HEAD cannot yet produce. Each is checked
in code-checkable terms.

| # | population | exposure if R8 lands on `fd6955e8` | evidence | removed by |
|---|---|---|---|---|
| (a) | DOMParser nodes with handler attributes | parse-time raw entries; HEAD's compile gate reads the HostData flag only, so `d.onclick` compiles (spec: null, *get the current value* step 3.2) | N1 memo §4.2 / D-N2 (the node clause is added to the compile gate by N1); `scripting.rs:83-113` (the clause exists, but at HEAD the compile gate does not consult it; N1 memo D-N2 adds it) | **N1 landing first.** H-P13 does not need rewriting: it describes N1's base, which then does not contain R8's DOMParser processing. N1's own DOMParser cells write handlers after the parse (N1 memo H-P13 result column), so none depends on R8 |
| (b) | a DOMParser (or fragment-throwaway) `<body>` with a WindowEventHandlers attribute | parse-time delivery to the **page** Window: `resolve_handler_target` returns `dom.window_entity()` for any `<body>` (`event_handler_consumer.rs:328-334`) | `webref body html event-handler-attributes`: *determine the target* step 3 "If eventTarget's node document is not an active document, then return null." HEAD has no step 3 (slot `#11-event-handler-target-determination` facet 1) | **the slot's step-3 fix landing first** ("T3"). T3 needs no prerequisite: "not an active document" can reuse the effective-node-document rule already in `scripting.rs:100-111` against `EcsDom::document_root` (`dom/mod.rs:425`). With T3, R8 is spec-exact here: a DOMParser body's WEH attribute writes nothing, and moving that body into the page does not deliver it (insertion runs no change steps) |
| (c) | template contents and their clones (**not in the brief**, P8) | parse-time raw entries on template-content elements; the composition does not suppress them (H15), so after N1 they compile, and `tpl.content.querySelector('b').dispatchEvent(new Event('click'))` runs the inline handler (spec: the template contents owner document has a null browsing context, `webref body html appropriate-template-contents-owner-document` step 1.1.1, so null). HEAD reads null because nothing delivers until a move out. A clone of template content (`tpl.content.cloneNode(true)`) keeps the page document as owner (`clone_subtree` threads `owner_document(src)`, `tree_clone.rs:208`) | H15 | **`#11-template-contents-owner-document` (C1) landing first.** A narrower clause-(b) facet ("composed root is a template-contents fragment") does not suffice: a clone of template content is not under such a fragment, so the inert owner must be carried per node, which is C1's part (a). C1's part (b) (adopt into the inert owner) is needed for `tpl.innerHTML = …` results |
| (d) | the pre-bind parse (G1) | none; R8 needs a bind-time pass because no consumer exists at parse (H9) | H9, H13 | nothing: the bind pass is K-4's own arm, not an interim. H13 makes it per-element idempotent, which D-1's marker gives. The same marker is needed under every mechanism (rev 1's D-3 included) once G1 is in scope |

**G1's effect on the comparison** (PM condition 3). G1 adds the bind pass (d). Under R8 it is the creation-equivalent
point for pre-bind elements: no script runs between the parse and the bind, so processing at bind is observably
processing at creation. Under rev 1's D-3 it would be a **second** delivery timing beside first insertion (bind for the
pre-bind tree, insertion for everything else), which is the strangler mix "One issue, one way" forbids. So G1 makes the
case for R8 stronger, and it forces D-1's per-element once-key whatever the mechanism (H13).

### §1.3 Verdict

**R8 lands directly, with no interim: yes.** The order is **T3 → N1 → C1 → `EH-attrs-at-creation`** (decisive
evidence: rows (a)–(c); each blocker is a population whose null-reading is someone else's fix, and none of the three
depends on this work). Rev 1's interim (`Pending`) is withdrawn (§4.3 R-D3).

The order has three costs, recorded for the PM:

1. **It reverses PM decision 17 (i)** if taken as one PR: N1 would land first and could not carry C8b(iv) or C8c's
   second half (N1 memo NR7), which would move to this work's cells. Between N1 and this work, a compiled handler moved
   into a DOMParser document reads null under N1's arm 3 (HEAD: a new function from the attribute). It runs in neither;
   both differ from the spec (the original function, which runs).
2. **C1 is a deliberately deferred, edge-dense slot** (nested templates × shadow-in-template × clone, adopt
   special-casing, inert-document GC; trigger "the per-document inert-document infra lands for another reason"). It has
   no driver.
3. **G1 is production-visible** (every inline handler in initial HTML is inert until a script moves its element) and
   would wait behind all three.

### §1.4 The recommended landing: two slices, no interim code

The populations that need T3 / N1 / C1 are DOMParser documents and template contents. Every other population (the
pre-bind tree, live-document elements, moves) needs nothing. So the mechanism lands in two slices, and **slice a adds
nothing that slice b deletes**:

- **Slice a (now; no prerequisite; N1 depends on it).** D-1 marker; D-2 functions; the bind pass (D-5, G1); the Insert
  arm processes once (K-1); D-4 in its restricted form; shadow-including walks (G2's reachable part). Template contents
  are not walked (HEAD parity: they are processed when moved out, as at HEAD).
- **Slice b (after T3, N1, C1).** Creation-time call sites for every bound creation path (fragment build, DOMParser
  build, clone, template contents); D-4 unrestricted; the Insert arm becomes the fail-safe fallback with a detector
  (§10 O6); the `inert_document.rs` docstring (P3). Closes K-2 / K-3 for all elements, G2's remaining cases, G5, K2.

What slice a leaves until slice b (all present at HEAD; none introduced by slice a): a pre-insertion write on a
never-inserted post-bind element with **two or more** deliverable handler attributes is overwritten at first insertion
(K-2 / K-3 residual); detached clones read null (G5); the body-clone cross-origin case (K2, §5.5). The single slice-a
element that slice b changes is D-4's restriction ("no other deliverable attribute"), one guard condition in
`note_handler_write`, which slice b deletes. That is the One-issue-one-way question for plan-review (§10 O2). Proposed
slot text (for the PM to register if the split is adopted):

> `#11-eh-attrs-creation-time-call-sites` — owner Layout lane. Slice a of `EH-attrs-at-creation` processes an
> element's creation-time handler content attributes at first bind or first insertion, and `note_handler_write` skips
> elements with two or more deliverable handler attributes. Slice b adds the creation-time call sites (fragment build,
> DOMParser build, clone, template contents) and deletes that guard. Trigger: T3 (`#11-event-handler-target-determination`
> facet 1), N1, and `#11-template-contents-owner-document` landed. Re-eval: when the last of the three lands.

With the split, PM decision 17 (i) stands as is (N1 depends on slice a, §8), and N1 needs only its dependency name
updated.

## §2. Coupled invariants

### §2.0 Axis count (PM condition 2)

Rev 1 counted 8 (once, order, origin-key, freshness, first-insert completeness, policy, DOMParser inertness, layering).
With G1, G2 and G5 included and the mechanism changed, the axes are:

| axis | source | slice |
|---|---|---|
| I1 once (per element, across moves and rebinds) | K-1, H13 | a |
| I2 writer order (a write is never overwritten) | K-2, K-3 | a (restricted), b |
| I3 origin key (state on the origin, not the target) | D-1 | a |
| I4 freshness (new elements start unprocessed; clone does not copy) | H8 | a |
| I5 coverage (every creation path, the pre-bind tree, shadow trees, template contents) | K-4, K-6, G1, G2 | a, b |
| I6 policy | `:405`, `:447` | a |
| I7 inert-document reads (DOMParser, template) | K-5, H5, H15 | b (N1, C1) |
| I8 target determination (step 3) | §1.2 (b) | b (T3) |
| I9 layering / re-entry | Layering mandate; `mutation_event.rs` | a |

**Count: 9** (≥ 3). The edge-dense rule applies, and whether this prerequisite is itself split is §10 O1.

### §2.1 Pairs (slice a in full; slice b's pairs are listed for its own plan)

| pair | intersect? | resolution |
|---|---|---|
| I1×I2 | yes | A processed element is marked; writers act on its entries directly (HEAD). `note_handler_write` on a marked element is a no-op. Cells C1, C4, U5 |
| I1×I3 | yes | The marker is per origin. A second `<body>` (a clone) is unmarked and processes once to the Window. Cells C8, U2 |
| I1×I4 | yes | Clone does not copy the marker (H8; clone-policy row). Cells C9, U6 |
| I1×I5 | yes | The bind pass and the Insert arm are the same function; the marker makes the bind pass idempotent across H13's repeated binds. Cells E3, U7 |
| I1×I6 | yes | The marker is set when the element has any handler content attribute, excluded or not, so an excluded-only element is not rescanned. Cell U3 |
| I1×I9 | yes | The once check is inside `process_handler_content_attrs`; the VM has no once logic |
| I2×I3 | yes | `note_handler_write` marks the `<body>` receiver for a delegated write. Cell C8 |
| I2×I4 | yes | A clone starts unmarked; a write on the source never marks the clone. Cell C9 |
| I2×I5 | yes | Slice a: a write on an unmarked element with no other deliverable attribute marks it, so first insertion skips it (C4, C5, C11). With two or more, first insertion processes all (the residual, §1.4) |
| I2×I6 | yes | "Deliverable" = in `EVENT_HANDLER_ATTRS` and installed by the policy. An excluded name never reaches `note_handler_write` from the AttributeChange arm (returns at `:405-407`); its IDL setter is not installed |
| I2×I9 | yes | Both VM setters call the engine-independent `note_handler_write` (D-6) |
| I3×I5 | yes | The bind pass resolves `<body>` WEH rows to the Window, which exists at bind (H14). Cell E4 |
| I3×I8 | yes (b) | Slice a keeps HEAD's target (no step 3). The bind pass reaches only the active document's tree, where step 3 does not fire. A DOMParser `<body>` is still delivered to the page Window at its first insertion (HEAD, G3) |
| I4×I5 | yes | Fragment-parsed nodes and clones are processed at first insertion in slice a (H6 placement), at creation in slice b. Cells C10 |
| I5×I6 | yes | The bind pass and the Insert arm apply the same `policy.installs` filter as HEAD (`:447`). Cell U3 |
| I5×I7 | yes | Slice a's bind pass starts at `dom.document_root()` (`dom/mod.rs:425`), so it never reaches a DOMParser document (created by `create_document_node`, which does not set the root, `:397-418`), and it does not walk template contents (H10). H-P13 stays true. Cell U8 |
| I5×I9 | yes | The bind pass runs inside `initialize_consumers`, before the dispatcher is installed, so no `MutationEvent` re-enters (`vm_api.rs:313-314`) |
| I6×I7, I6×I8, I6×I9, I7×I9, I8×I9 | no | — |
| I7×I8 | yes (b) | Both are null-reading for inert documents; slice b needs both prerequisites |
| I2×I7, I2×I8, I4×I7, I3×I7 | yes (b) | Slice b's: an unrestricted ensure-before-write on a DOMParser node materialises its siblings as raw values, which read null only under N1 (I7) and do not reach the page Window only under T3 (I8). This is why D-4 is restricted in slice a |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | event handler content attribute *attribute change steps* 1–5.6 | step 1 non-handler name → return; step 3 null target → return; step 4 value null → deactivate; step 5.5 set raw uncompiled handler; step 5.6 activate | `process_handler_content_attrs` (NEW) runs them for creation-time attributes; `handle_attribute_change` `crates/script/elidex-script-session/src/event_handler_consumer.rs:389` for writes (calls `note_handler_write` first) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determining the target of an event handler* 1–4 | non-body → self; not WEH and not the reflecting set → self; step 3 inactive node document → null; else relevant global | `resolve_handler_target` `event_handler_consumer.rs:328` (unchanged here; step 3 is T3, the reflecting set is facet 4, both slot `#11-event-handler-target-determination`) | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4 | target null → return; value null → deactivate; else set value + activate | `native_event_handler_set` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:344`, `native_body_weh_set` `:459` → `note_handler_write` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2 | raw value in a null-browsing-context document → null | `ensure_event_handler_current` `event_handler_attrs.rs:521` (unchanged here; N1's node clause; template contents need `#11-template-contents-owner-document`) | ✗ | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers rows | WEH rows on `<body>` delegated (HEAD) | `EVENT_HANDLER_ATTRS` `event_handler_consumer.rs:61`; cells C8, E4 use `onhashchange` | ✗ | no |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4 (*handle attribute changes*) | every attribute present at creation runs the attribute change steps once | `process_handler_content_attrs` (NEW): the realisation of those per-attribute runs | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token*, "Append attribute to element" | parser path: every token attribute is appended at creation | parser bakes `Attributes` at spawn (`crates/dom/elidex-html-parser/src/convert.rs:258`; strict `tree_builder/insert.rs:61`, `:119`, `shadow.rs:162`); processed at bind (pre-bind) or at first insertion (slice a) | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | the fragment's new `Document` | fragment elements are created in a throwaway document | `begin_detached_fragment` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:318` (unchanged in slice a) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5 | a clone's attributes are appended at its creation | clone-policy table `crates/core/elidex-ecs/src/dom/tree_clone.rs:19-34` (row added: marker non-copy) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* | node document changes; no event-handler step | `EcsDom::adopt_subtree` `teardown.rs:285` (unchanged; the marker survives) | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert*, *insertion steps* | HTML defines no event-handler insertion steps | `handle_insert` `event_handler_consumer.rs:427` → process once | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including descendant* | walks include shadow trees | `for_each_shadow_inclusive_descendant` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:33` | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents*; *appropriate template contents owner document* | contents are not descendants; their owner document has a null browsing context | not walked in slice a (H10); slice b after `#11-template-contents-owner-document` | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` | `text/html` branch only | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged in slice a) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter | also outerHTML / setHTMLUnsafe | fragment placement fires Insert (H6; unchanged) | ✗ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author-controlled markup or script values. The mechanism stores no user string: the marker is a
unit component, and the handler sources it writes are HEAD's (`set_inline_handler`). The bind pass newly makes
initial-HTML handler sources reachable by the compiler, which is the spec's behaviour (they are raw values at creation);
they compile only through HEAD's compile gate. No row adds a parse of user input.

## §4. Mechanism

### §4.1 Home / kind (slice a)

```rust
// crates/script/elidex-script-session/src/event_handler_consumer.rs (NEW items)
/// The attribute change steps (WHATWG HTML §8.1.8.1) have run for this element's
/// creation-time handler content attributes (WHATWG DOM §4.9 "append an attribute"
/// step 4). Absent = unprocessed. Sparse; on the origin element, never the target.
pub struct HandlerAttrsProcessed;
pub fn process_handler_content_attrs(policy: SpecLevelPolicy, element: Entity, dom: &mut EcsDom);
pub fn process_handler_content_attrs_in_subtree(policy: SpecLevelPolicy, root: Entity, dom: &mut EcsDom);
pub fn note_handler_write(policy: SpecLevelPolicy, origin: Entity, event_type: &str, dom: &mut EcsDom);
impl EventHandlerAttributeConsumer { pub fn initialize_from_tree(&mut self, dom: &mut EcsDom); }
```

The marker is `Send + Sync`, per-entity, and not a per-VM identity handle, so per CLAUDE.md's side-store→component
rule it is a component. Despawn removes it; adopt keeps it.

- **`process_handler_content_attrs(policy, element, dom)`**: return if `element` is not an element or is marked. Collect
  its attributes named in `EVENT_HANDLER_ATTRS` (attribute order). If none, return (no marker). For each: skip it if the
  policy excludes it (`:447`'s filter); otherwise resolve the target (`resolve_handler_target`) and call
  `set_inline_handler`. Then insert the marker. Collect-then-write (HEAD's borrow discipline).
- **`process_handler_content_attrs_in_subtree(policy, root, dom)`**: collect `root` and its shadow-including descendants
  (`for_each_shadow_inclusive_descendant`, H10), then process each. Template contents are not visited (H10).
- **`note_handler_write(policy, origin, event_type, dom)`**: return if `origin` is not an element or is marked. If
  `origin` carries no deliverable handler content attribute other than `on{event_type}`, insert the marker (slice a's
  restriction; slice b replaces this body with `process_handler_content_attrs(policy, origin, dom)`, §1.4).
- **`initialize_from_tree`**: if `dom.document_root()` is `Some(root)`, process its subtree.

### §4.2 Rewrite mapping (HEAD → slice a)

| # | site | HEAD | slice a |
|---|---|---|---|
| 1 | `handle_insert` `event_handler_consumer.rs:427-463` | light-tree walk, re-delivers on every Insert | `process_handler_content_attrs_in_subtree(policy, node, dom)` |
| 2 | `handle_attribute_change` `:389-421` | after the policy check (`:405`), resolves and writes | after the policy check, `note_handler_write(policy, node, event_type, dom)`, then HEAD's write and removal arms unchanged |
| 3 | `native_event_handler_set` `event_handler_attrs.rs:344` | `activate_event_handler(..)` | `note_handler_write` first (no-op for Document / Window / Worker receivers), then the same call; callable and null alike |
| 4 | `native_body_weh_set` `:459` | delegates to the Window | `note_handler_write` on the **`<body>` receiver** (I3), then delegates |
| 5 | `ConsumerDispatcher::initialize_consumers` `crates/script/elidex-js/src/vm/consumer_dispatcher.rs:135-137`, doc `:113-134` | `base_url` only; doc says other consumers need no init | adds `self.event_handler_attrs.initialize_from_tree(dom)`; the doc states why (pre-bind parse, H9) and that it is idempotent per element (H13) |
| 6 | `tree_clone.rs:30` "deliberate non-copy" row | — | adds `HandlerAttrsProcessed` (elidex-script-session): DOM §4.4 *clone a single node* step 2.5 appends the clone's attributes at its own creation |
| 7 | module docs `event_handler_consumer.rs:1-33`; `consumer_dispatcher.rs:58-66` field doc | "Arm 2 walks the inserted subtree's baked `on*` attributes" | the processing rule (D-3) and the bind arm |
| 8 | `crates/script/elidex-script-session/src/lib.rs:36-41` | re-exports | adds `HandlerAttrsProcessed`, `note_handler_write` (VM), the two process functions (tests) |

`inert_document.rs` and `attribute_reconcile.rs` are not touched in slice a (P3, G6 stay MINOR until a slice touches
them).

### §4.3 Alternatives rejected

- **R-D3: rev 1's mechanism** (`HandlerAttrDelivery::Pending { superseded }` → `Delivered` at first insertion). It
  delivers at insertion and records writes so delivery can skip them. Rejected: §1.3 shows R8 lands with no interim, and
  `Pending` exists only to bridge a deferral that R8 removes; with G1 it would also be a second timing beside bind
  (§1.2 G1 paragraph). Its "absence = undelivered, fail toward HEAD" property is kept by D-1.
- **R1: a new `MutationEvent` (or AttributeChange) at every spawn.** AttributeChange at creation fans out to every
  consumer (custom-element `attributeChangedCallback`, canvas, base URL, form, MutationObserver records), changing each
  one's meaning. A dedicated creation event is one of slice b's two candidate call-site shapes (§10 O9), not slice a's.
- **R2: key on the target's entry.** The Window's entries are shared by every `<body>` and by `window.on*`, so a body
  clone would never deliver; the IDL null leaves no entry (`activate_event_handler` `:683-692`).
- **R3: a connectedness filter.** Remove-then-append re-delivers; a DOMParser document is connected; not the spec's
  condition.
- **R4: a positive "unprocessed" marker at spawn.** The spawn sites are below `elidex-script-session`, and a forgotten
  site leaves dead handlers. D-1's absence-means-unprocessed fails toward HEAD's delivery.
- **R5: compare source text.** No spec basis; "once" as a value coincidence.
- **R6: a session or `HostData` set.** Violates the side-store→component rule.
- **R7: unrestricted ensure-before-write in slice a.** On a DOMParser node it materialises raw siblings, which HEAD's
  getter compiles (needs N1), and on a DOMParser `<body>` it delivers WEH siblings to the page Window (needs T3). It is
  slice b's D-4, after those land.
- **R9: a dense marker** (mark every element at first sight, so writers need no rule). It avoids D-4's guard, but it
  costs one archetype move per element ever inserted or bound, for a marker whose information is "nothing to do". D-4's
  restricted rule gives the same K-1 with the marker only where handler attributes exist (§10 O3).
- **R10: bind pass at the first bind only** (no per-element once-key at bind). It would avoid the per-bind walk, but it
  misses elements created or written while unbound after the first bind (§7 G7, reachability unverified). Kept as the
  fallback if O3's measurement says the walk is too costly.

### §4.4 ECS-native and Layering checks

- Idiom mapping: the OO "element.attributesProcessed flag" → a marker component on the entity; "process the document at
  attach" → a system over the bound tree run at the dispatcher's install point; "writers notify the element" →
  writers calling one engine-independent function. No registry, no new dispatcher variant, no side-store.
- `vm/host/` adds two calls into `elidex_script_session` (marshalling). `consumer_dispatcher.rs` is the composer, not
  host code; it gains one delegate call, like `base_url`'s.
- Inside `dispatch` the consumer writes only `EventListeners` and the marker (I9).

### §4.5 Docstring citation table (Axis 4)

| doc site | citation |
|---|---|
| `HandlerAttrsProcessed`, `process_handler_content_attrs` | WHATWG HTML §8.1.8.1 Event handlers (attribute change steps); WHATWG DOM §4.9 Interface Element (*append an attribute* step 4) |
| `process_handler_content_attrs_in_subtree` | WHATWG DOM §4.8 Interface ShadowRoot (*shadow-including descendant*); WHATWG HTML §4.12.3 The template element (contents are not descendants) |
| `note_handler_write` | WHATWG HTML §8.1.8.1 (IDL setter steps; attribute change steps 4–5) |
| `initialize_from_tree`, `initialize_consumers` doc | WHATWG HTML §13.2.6.1 Creating and inserting nodes (attributes appended at creation) |
| clone-policy row | WHATWG DOM §4.4 Interface Node (*clone a single node* step 2.5) |

## §5. Populations and named risks

### §5.0 `Attributes` writers: silent vs firing (census, reproducible)

A **silent writer** is a production code path that writes an element's `Attributes` without firing
`MutationEvent::AttributeChange` for that write. Population: every hit of the patterns below in `crates/`.

**Classification rule.** A hit `path:line` is **test** if `path` matches the ERE
`/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support`, or if the file contains
`#[cfg(test)]` and `line` is greater than the line of its first occurrence. Otherwise it is **prod**. Command (run from
the repo root; `P` = prod, `T` = test):

```sh
c() { /usr/bin/grep -rnE --include='*.rs' "$1" crates | while IFS=: read -r f l _; do
  if echo "$f" | /usr/bin/grep -qE '/tests/|/tests\.rs$|/tests_[^/]*\.rs$|_tests\.rs$|/benches/|test_helpers|test_support'
  then echo "T $f:$l"; continue; fi
  t=$(/usr/bin/grep -n -m1 '#\[cfg(test)\]' "$f" | cut -d: -f1)
  if [ -n "$t" ] && [ "$l" -gt "$t" ]; then echo "T $f:$l"; else echo "P $f:$l"; fi; done; }
c '<pattern>' | /usr/bin/grep -c '^P'    # and '^T'; `| /usr/bin/grep '^P'` lists the prod sites
```

Run at `fd6955e8` on 2026-10-25:

| pattern (ERE) | total | test | prod | production sites → class |
|---|---|---|---|---|
| `get::<&mut (elidex_ecs::)?Attributes>` | 21 | 17 | 4 | `attribute.rs:215` (`write_attribute_no_dispatch`), `:305` (`remove_attribute`: **fires** when present, `:318`), `tree_clone.rs:68` (`clone_attributes`: **silent**, no prod caller), `content/iframe/lifecycle.rs:358` (iframe `src` rewrite: **silent**, `src` only) |
| `insert_one\([^)]*(attrs\|Attributes\|attributes)` | 7 | 5 | 2 | `attribute.rs:224` (`write_attribute_no_dispatch`), `tree_clone.rs:72` (`clone_attributes`) |
| `builder\.add\(attrs\)` | 1 | 0 | 1 | `tree_clone.rs:368` (in `clone_node_shallow_unchecked` `:270`, the cloneNode path: **silent**, all attributes) |
| `\.create_element(_with_owner\|_ns)?\(` | 2210 | 2187 | 23 | 17 pass `Attributes::default()` (`teardown.rs:327`, `inert_document.rs:120`, `document.rs:235`, `entity_spawn.rs:57`, `select.rs:329`, `document_props.rs:337`, `anonymous_table.rs:159`, `grid.rs:422`, `:467`, `table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359`, `:448`, `:512`); 2 are shims (`dom/mod.rs:335`, `:380`); **4 bake parser attributes (silent)**: `elidex-html-parser/src/convert.rs:258`, `elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:119`, `tree_builder/shadow.rs:162` |
| `\.set_attribute_without_dispatch\(` | 1 | 0 | 1 | `crates/dom/elidex-form/src/value_mode.rs:64`: **silent**, `value` only |
| `\.clone_attributes\(` | 1 | 1 | 0 | test only (`dom/tests/clone.rs:14`) |
| `\.set_attribute\(` | 141 | 134 | 7 | all **fire** via the `attribute.rs:125` chokepoint: `mutation/mod.rs:903`, `select/mod.rs:156`, `:182`, `element/attrs.rs:208`, `:256`, `char_data/attr.rs:172`, `:333` |
| `\.remove_attribute\(` | 24 | 18 | 6 | 5 **fire** when present (`mutation/mod.rs:925`, `select/mod.rs:159`, `:178`, `dialog.rs:80`, `char_data/attr.rs:243`); 1 is a comment (`vm/host/attr_proto.rs:44`) |

Parser attribute merging (an in-body `<html>` / `<body>` start tag adding attributes) has no elidex-side writer:
`/usr/bin/grep -rn --include='*.rs' -i -E "already present|merge_attr|add_missing|set_attribute|write_attribute"
crates/dom/elidex-html-parser-strict/src crates/dom/elidex-html-parser/src` lists no prod hit by the rule above.

Result:
- (a) The silent writers that can write an `on*` attribute are exactly the creation-time bakes: the four parser sites
  and `clone_node_shallow_unchecked` (plus the prod-unreachable `clone_attributes`). These define "creation-time
  attributes" in §0.5.
- (b) The other silent writers write fixed non-handler names (`value`, `src`).
- (c) Every post-creation `on*` write fires AttributeChange while a dispatcher is installed (7 + 5 firing sites).
  While **unbound** (no dispatcher, H13) a firing write is not observed; that is G7 (§7).

### §5.1 Handler-state writers and readers

Commands: `/usr/bin/grep -rn --include='*.rs' -E
"set_uncompiled|clear_uncompiled|take_uncompiled|uncompiled_source|mark_cleared|is_handler_cleared" crates` and
`/usr/bin/grep -n -E "add_event_handler|find_event_handler" crates/script/elidex-script-session/src/event_handler_consumer.rs
crates/script/elidex-js/src/vm/host/event_handler_attrs.rs`.

| op | production sites | role in slice a |
|---|---|---|
| `set_uncompiled` | `event_handler_consumer.rs:474` (in `set_inline_handler`, from `:411` and the Insert arm) | `:411` unchanged; the Insert-arm and bind calls go through `process_handler_content_attrs` only |
| `clear_uncompiled` | `event_handler_attrs.rs:677`, `:687` (`activate_event_handler`) | unchanged; preceded by `note_handler_write` |
| `take_uncompiled` | `event_handler_attrs.rs:567` | unchanged (read-side compile) |
| `mark_cleared` | `event_handler_consumer.rs:487` (`clear_inline_handler`, from `:418`) | unchanged |
| `is_handler_cleared` | `event_handler_attrs.rs:568` | unchanged |
| `uncompiled_source` | none in production (`event_listener.rs:282`) | the store oracle for C2, U1, U2 |
| `add_event_handler` | `event_handler_consumer.rs:473`, `event_handler_attrs.rs:674` | unchanged |

Readers of `ensure_event_handler_current` (`/usr/bin/grep -rn --include='*.rs' 'ensure_event_handler_current('
crates/script/elidex-js/src`): `dispatch_target.rs:183`, `natives_promise.rs:548`, `engine.rs:233`. All read.

### §5.2 `MutationEvent::Insert` consumers

Command: `/usr/bin/grep -rn --include='*.rs' -E "MutationEvent::Insert" crates` (non-test arms): `LiveRangeBridge`
(`range/live.rs:121`), `BaseUrlMaintainer` (`document_base.rs:293`), `FormControlReconciler` (`reconciler.rs:44`),
`CustomElementReactionConsumer` (`consumer.rs:77`), this consumer (`event_handler_consumer.rs:381`). None reads
`EventListeners` (`consumer_dispatcher.rs:58-66`). No existing test moves an element carrying a handler content
attribute (`/usr/bin/grep -rln --include='*.rs' -E "setAttribute\(.on[a-z]+|onclick=" crates/script/elidex-js/src` →
`tests_event_handler_attrs.rs`, `tests_call_listener.rs`; neither does). No shell test asserts an initial-HTML inline
handler (`/usr/bin/grep -rn --include='*.rs' 'onclick=' crates/shell` → nothing), which is why G1 survived.

### §5.3 Creation paths and their processing point

| path | site | slice a | slice b |
|---|---|---|---|
| main-document / iframe parse (pre-bind) | `pipeline.rs:850-858` (from_loaded), `load.rs:471` (srcdoc) | **first bind** (G1) | same |
| fragment (innerHTML / outerHTML / setHTMLUnsafe) | `teardown.rs:318`; `convert.rs:258`; strict `insert.rs:61`, `:119` | first insertion (placement, H6) | fragment build end |
| DOMParser | `inert_document.rs:110-200` | first insertion into a document after return | build end (after N1, T3) |
| `createElement` + `setAttribute` | `dom/mod.rs:344` + AttributeChange | the write marks (D-4) | same |
| `cloneNode` | H17 | first insertion | clone time |
| declarative shadow (parser) | `shadow.rs:162` | bind (pre-bind) / host insertion (fragment) — shadow-inclusive walk | creation |
| template contents | `dom/mod.rs:560` | when moved out (Insert per child, HEAD) | creation (after C1) |

### §5.4 Interaction matrix

| interaction | slice a outcome |
|---|---|
| innerHTML / outerHTML / setHTMLUnsafe | fresh elements, processed once at placement |
| cloneNode | fresh, unmarked (I4); processed at first insertion; a write on the clone with no other deliverable attribute marks it (C5, C11) |
| importNode / adoptNode | not exposed (P1; `#11-cross-document-adopt-on-insert`); `adopt_subtree` keeps the marker |
| shadow trees (G2) | a host's shadow tree is processed with the host (bind, insertion). An element inserted directly under a shadow root fires no Insert (H2) and is processed at creation only in slice b |
| `<body>` WEH delegation | marker on the `<body>`; target the Window (`:328`). Cells C8, E4 |
| policy-excluded attributes | filtered as `:445-449`; they count toward the marker. Cell U3 |
| attribute removed while outside the tree | AttributeChange clears; processing reads `Attributes`, so the removed name is never processed (C12) |
| repeated binds | the bind pass skips marked elements (E3, U7) |

### §5.5 Named risks

- **K1: an unknown creation path.** An element baked by a path that slice b forgets stays unmarked and is processed at
  the next bind (if in the bound tree) or its first insertion: HEAD-like, never loss. Slice b adds a detector (§10 O6).
- **K2: body-clone cross-origin write** (`c = document.body.cloneNode(); window.onhashchange = fn; append(c)`): the spec
  processes `onhashchange` at clone creation and `fn` wins; slice a processes at insertion and the raw value wins.
  Pre-existing; closes in slice b.
- **K3: detached reads** (G5): a never-inserted clone of a live element reads null (spec: a compiled function).
  Pre-existing; closes in slice b.
- **K4: walk cost.** The Insert arm keeps HEAD's walk (now shadow-inclusive) and adds one `get::<&HandlerAttrsProcessed>`
  per element. The bind pass walks the bound document's shadow-including tree on every bind (H13), like
  the base-URL maintainer's `initialize_from_tree` (`document_base.rs:276-286` → `initialize_base_url_for_document` `:203`).
  §10 O3.
- **K5: G1 exposes G4.** Initial-HTML `<body onload>` / `onfocus` / … are now delivered, onto the **body** (P2), where
  `load` never arrives and `focus`-type events at the body run them. Before slice a nothing was delivered. The fix is
  facet 4 of `#11-event-handler-target-determination`, independent of this work.

## §6. Test and coverage plan

*Red* = must fail at `fd6955e8` and pass after; the implementer runs each red cell against `fd6955e8` first and
records the failure in the PR description (red-first). *Guard* = passes at both. The "HEAD" predictions come from H1 /
H3 / H9 and are not measurements.

### §6.1 VM integration and unit cells (slice a)

VM, new file `crates/script/elidex-js/src/vm/tests/tests_event_handler_delivery.rs` (NEW), registered in
`crates/script/elidex-js/src/vm/tests/mod.rs`, reusing the `with_el` / `entity_of` shape of
`tests_event_handler_attrs.rs:38-76` with `el` connected. Dispatch is `dispatchEvent(new Event('click'))` (the VM has no
`click()`, N1 memo H-P14).

| cell | kind | shape | HEAD | oracle |
|---|---|---|---|---|
| C1 | red | `el.innerHTML = '<div id=t onclick="A">'`; `t.onclick = fn`; `el.append(t)` (move) | move re-raws A | `t.onclick === fn`; dispatch runs `fn` |
| C2 | red | `t = createElement('div')`; `el.append(t)`; `t.setAttribute('onclick', S)`; dispatch (compiles); `f = t.onclick`; `doc2.body.appendChild(t)` (N1 C8b(iv) shape) | move re-raws S | store: `click` entry `uncompiled_source == None`; `t.onclick === f` |
| C3 | red | baked A via innerHTML; `f = t.onclick`; move twice live→live | each move recompiles | `t.onclick === f` after each |
| C4 | red | `c = createElement('div')` (never inserted); `c.setAttribute('onclick', A)`; dispatch on `c` (compiles); `f = c.onclick`; `doc2.body.append(c)` (N1 R2's R-a) | first insert re-raws A | store `uncompiled_source == None`; `c.onclick === f` |
| C5 | red | DOMParser `<div id=d onclick="A">`; `d.onclick = fn`; variants: (i) `dp2.body.append(d)`, (ii) `el.append(d)`, (iii) `c = d.cloneNode()`, `c.onclick = fn`, `el.append(c)` (N1 R2's R-b, single-attribute form) | first insert raws A over `fn` | `d.onclick === fn` (`c.onclick === fn`); dispatch runs `fn` in (ii), (iii) |
| C6 | guard | DOMParser `<div id=d onclick="A">` appended into `el` | delivered at insert | `typeof d.onclick === 'function'`; dispatch runs A |
| C7 | guard | delivered `t`, `f = t.onclick`; `t.setAttribute('onclick', B)` | — | `t.onclick !== f`; dispatch runs B |
| C8 | red | `b = createElement('body')`; `el.append(b)`; `b.setAttribute('onhashchange', X)`; `window.onhashchange = fn`; move `b` | move re-raws X onto the Window | `window.onhashchange === fn`; store: marker on `b`, none on the Window |
| C9 | guard | `t` delivered and IDL-set to `fn`; `c = t.cloneNode()`; insert `c` | — | before insert: `c` unmarked; after: `c.onclick` is compiled A (not `fn`) |
| C10 | red (G2) | `h = createElement('div')`; `sr = h.attachShadow({mode:'open'})`; `sr.innerHTML = '<button id=b onclick="…">'`; `el.append(h)` | shadow content never delivered (H10) | dispatch on `sr.getElementById('b')` runs the inline body |
| C11 | red | `c = createElement('div')`; `c.setAttribute('onclick', A)`; `c.onclick = fn`; `el.append(c)` | first insert raws A over `fn` | `c.onclick === fn` |
| C12 | guard | clone of `onclick=A`; `c.removeAttribute('onclick')`; insert | — | `c.onclick === null` |
| C13 | guard | parser tiers: `el.innerHTML = '<button onclick=…>'` (strict) and a foster-parenting `<table>` string (tolerant fallback) | delivered | dispatch runs the body on both |

Unit, new file `crates/script/elidex-script-session/src/tests_event_handler_delivery.rs` (NEW), `#[cfg(test)]` in
`lib.rs`, driving `EcsDom` with `EventHandlerAttributeConsumer` installed, through real `append_child` /
`set_attribute` calls so `Attributes` and the events agree.

| cell | kind | shape | oracle |
|---|---|---|---|
| U1 | red | element spawned with `onclick=A`, appended; `take_uncompiled`; appended elsewhere | `uncompiled_source == None` (HEAD `Some("A")`) |
| U2 | red | `<body onhashchange=X>` with a Window; append; `take_uncompiled` on the Window entry; move | Window entry `None`; marker on body only |
| U3 | guard | `BrowserCore`, `<body onstorage=…>` appended twice | no `storage` entry; body marked after the first append |
| U4 | guard | element with no handler attributes appended | unmarked |
| U5 | red | (i) unmarked element with only `onclick=A`: `set_attribute(onclick, B)` → marked; (ii) with `onclick=A onmouseover=C`: `set_attribute(onclick, B)` → unmarked; `note_handler_write` on (iii) a marked element → no change | (i) marked, then append leaves `click` taken-state intact; (ii) unmarked; (iii) unchanged |
| U6 | guard | `clone_node_shallow` of a marked element | clone unmarked |
| U7 | red (G1) | tree built **without** a dispatcher (pre-bind shape) containing `<button onclick=A>`; call `initialize_from_tree` twice, with `take_uncompiled` between | after the first call the entry is raw A and the button marked; after the second the entry is still `None` (not re-raw) |
| U8 | guard | `initialize_from_tree` with (i) a DOMParser-shaped second document (`create_document_node`) holding `<div onclick>`, (ii) a `<template>` whose contents hold `<div onclick>`, (iii) a shadow host with `<button onclick>` in its shadow root | (i), (ii) no entry and unmarked (H-P13, H10); (iii) delivered |

### §6.2 End-to-end pins (G1; PM condition 1)

New file `crates/shell/elidex-shell/src/content_initial_handler_tests.rs` (NEW), registered in
`crates/shell/elidex-shell/src/content/mod.rs` beside `iframe_security_tests` (`:796-797`). `synth_cross_origin_loaded`
(`content_iframe_security_tests.rs:399`) and `run_oop_and_collect_posts` (`:423`) move into `content_test_support.rs`
(288 lines) so both files use them. **Each pin is run on `fd6955e8` and its failure recorded in the PR description
before the fix commit** (consumer-level cells do not count as this pin). Consumer-level U7 does not substitute for
any of them.

| pin | path | shape | HEAD | oracle |
|---|---|---|---|---|
| E1 | main document, `build_pipeline_from_loaded` → bind (`pipeline.rs:839`) | `synth_cross_origin_loaded(html, url)` with `<button id=b onclick="document.body.dataset.hit=(+document.body.dataset.hit\|\|0)+1"></button><script>document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>`, passed to `build_pipeline_from_loaded` | no entry, nothing runs | `probe`-style read of `<body data-hit>` == `"1"` after the build (script-initiated dispatch inside the initial-scripts bind) |
| E2 | same pipeline, a later bind | `PipelineResult::dispatch_event` (`lib.rs:293`) with a click at `b` | nothing runs | `data-hit` == `"2"` (UA dispatch in its own bracket) |
| E3 | same pipeline, identity across rebinds | `eval_script("window.f=document.getElementById('b').onclick; typeof window.f")` → `"function"`; `dispatch_event` (another bind); `eval_script("document.getElementById('b').onclick===window.f")` | first read `"object"` | `"function"`, then `true` (K-1 across H13's rebinds) |
| E4 | main document, `<body>` WEH | `<body onhashchange="document.body.dataset.h=1">` … `<script>dispatchEvent(new HashChangeEvent('hashchange'))</script>` (or `new Event('hashchange')`) | nothing runs | `data-h` == `"1"` (delivered to the Window at bind, H14) |
| E5 | out-of-process iframe, `build_pipeline_from_loaded` (`load.rs:327`) | `make_out_of_process_entry(synth_cross_origin_loaded(…, "https://other.example/"), PreEvalFrameState { unsandboxed, … })` with `<button id=b onclick="postMessage('eh-hit','*')"></button><script>postMessage('ran','*');document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>`; `run_oop_and_collect_posts` | posts = `["ran"]` | posts contain `"ran"` and `"eh-hit"` (`"ran"` is the non-vacuity control) |
| E6 | in-process srcdoc iframe (`load.rs:471`) | `build_test_content_state_with_url` with `<iframe srcdoc="…E1's markup…">`; the in-process entry's pipeline | nothing runs | the child's `data-hit` == `"1"` |
| E7 | declarative shadow in initial HTML (G2) | E1's harness with `<div id=h><template shadowrootmode=open><button id=b onclick="…"></button></template></div><script>h.shadowRoot.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>` | nothing runs | hit recorded |

The same-origin URL iframe path (`load.rs:221`) needs a network fixture (§10 Q5); E5 and E6 cover the other two
iframe builders, and all three reach the same `run_scripts_and_finalize` bind.

### §6.3 Slice b's cells (listed for its own plan)

R-a / R-b for multi-attribute elements (DOMParser `<div onclick=A onmouseover=B>`; `d.setAttribute('onclick', X)`;
compile; insert → compiled X survives, `mouseover` raw B); G5 (a detached clone reads a compiled function); K2; template
contents read null before a move out and compile after (needs C1); a DOMParser `<body onpageshow>` delivers nothing,
before and after a move into the page (needs T3); `sr.innerHTML` and direct shadow-root insertion processed at creation;
the Insert-arm detector (§10 O6).

### §6.4 Kept green, mutation guards, WPT

Kept green: `tests_event_handler_attrs.rs` (35 tests: `/usr/bin/grep -c '#\[test\]'
crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` = 35) and the consumer's `spec_level_tests`
(`event_handler_consumer.rs:500`).

Mutation guards (slice a): dropping the marker check in `process_handler_content_attrs` fails C1–C3, C8, U1, U2, E3,
U7; deleting the bind delegate fails E1, E2, E4–E7, U7; a light-tree walk instead of the shadow-inclusive one fails
C10, E7, U8 (iii); a `note_handler_write` that never marks fails C2, C4, C5, C11, U5 (i); one that marks regardless of
other attributes fails U5 (ii); the bind pass starting anywhere but `document_root` (or walking template contents)
fails U8 (i) / (ii).

WPT: no subset is claimed (engine-unit, VM-integration and shell end-to-end only).

### §6.5 Touch-time 1000-line check (`wc -l` at `fd6955e8`)

| file | lines | after (est.) | action |
|---|---|---|---|
| `crates/script/elidex-script-session/src/event_handler_consumer.rs` | 590 | ≈ 660 | none; tests go to the new file |
| `crates/script/elidex-script-session/src/lib.rs` | 62 | ≈ 68 | none |
| `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | 726 | ≈ 732 | none |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 153 | ≈ 165 | none |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 197 | 198 | none |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | ≈ 426 | none |
| `crates/shell/elidex-shell/src/content/mod.rs` | 858 | 862 | none |
| `crates/shell/elidex-shell/src/content_test_support.rs` | 288 | ≈ 340 | none (receives the two hoisted helpers) |
| `crates/shell/elidex-shell/src/content_iframe_security_tests.rs` | 667 | ≈ 615 | none (helpers move out) |

`crates/shell/elidex-shell/src/tests.rs` (1082 lines) is not touched; the pins go to the new file. No touched file
exceeds 1000 lines.

## §7. Gaps: dispositions

Inclusion criterion (PM decision 18 (2)): a gap whose root cause is the delivery mechanism this work builds is in
scope; any other becomes a slot proposal.

- **G1, initial-HTML handlers never wired** (H9). **In scope** (slice a, D-5; E1–E7, U7). Root cause: elements created
  before any consumer existed are never processed, which is this mechanism's coverage (I5). Confirmed by static trace
  (H9) and by the absence of any shell test (§5.2).
- **G2, shadow-tree elements never delivered** (H2, H10). **In scope.** Root cause: the processing walk is light-tree
  only, and an element inserted directly under a shadow root fires no Insert. Both are this mechanism's coverage. Slice
  a makes every walk shadow-inclusive (C10, E7, U8 (iii)); the direct-under-shadow-root case closes by creation-time
  processing in slice b. `fire_after_insert`'s shadow-root filter (`mutation.rs:289`) is **not** changed: it serves every
  consumer, and slice b does not need it.
- **G3, target step 3 not modelled.** Slot `#11-event-handler-target-determination` (facet 1). Not here; it is slice
  b's prerequisite T3 (§1.2 (b)).
- **G4, the Window-reflecting body set.** Slot `#11-event-handler-target-determination` (facet 4, merged by the PM).
  Not here. Interaction: K5.
- **G5, reads never deliver** (K3). **In scope (slice b).** Root cause: processing deferred past creation, the
  mechanism's timing. It closes by construction once clones are processed at creation; slice a does not change it.
- **G6, stale docstring** `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23` (says the deferred session
  flush writes `Attributes` without the chokepoints; `apply_set_attribute` now routes through `EcsDom::set_attribute`,
  `mutation/mod.rs:897-909`). Not touched by slice a: **MINOR**.
- **G7 (new), writes and creations while unbound are not observed.** With no dispatcher installed (H13), a
  `setAttribute('onclick', …)` applied by an unbound `session.flush` (`pipeline.rs:138-165`: `flush_with_ce_reactions`
  runs `session.flush(dom)` outside any bracket) would fire AttributeChange to no consumer. Root cause: dispatcher
  absence while unbound, not this mechanism (the bind pass mitigates it only for unmarked elements). **Slot proposal**
  (for the PM): reachability under the VM is unverified (the code comment says the VM-native case is empty; §10 Q1).

## §8. Edges and hand-offs

- **→ N1** (read-only here). With the split: N1's dependency names slice a (`EH-attrs-at-creation` slice a, branch
  `layout-eh-insert-once`) instead of `EH-insert-once`; C8b(iv) and C8c's second half hold after slice a (cells C2, C3);
  H-P13 stays true after slice a (U8 (i)); N1's R2 requirements R-a / R-b hold after slice a for single-attribute
  elements (C4, C5) and fully after slice b (§6.3). Without the split (§1.3), N1 lands first, drops its dependency, and
  C8b(iv) / C8c's second half move here. Either way N1's H-P13 needs no rewrite (P9). The PM routes the N1 edit.
- **→ T3** (`#11-event-handler-target-determination` facet 1): slice b's prerequisite (§1.2 (b)).
- **→ C1** (`#11-template-contents-owner-document`): slice b's prerequisite (§1.2 (c)). This adds a trigger to that slot
  ("`EH-attrs-at-creation` slice b"), for the PM.
- **→ slice b**: its own per-PR plan and `/elidex-plan-review` before implementation (edge-dense rule), with §6.3's
  cells and §10 O6 / O9.

## §9. Definition of done (slice a)

- `HandlerAttrsProcessed`, `process_handler_content_attrs`, `process_handler_content_attrs_in_subtree`,
  `note_handler_write`, and `EventHandlerAttributeConsumer::initialize_from_tree` exist in `event_handler_consumer.rs`
  and are re-exported from `lib.rs`. `handle_insert` contains no direct `set_inline_handler` call
  (`/usr/bin/grep -n 'set_inline_handler(' crates/script/elidex-script-session/src/event_handler_consumer.rs` → the
  definition, `handle_attribute_change`, and `process_handler_content_attrs` only).
- `/usr/bin/grep -n 'traverse_descendants' crates/script/elidex-script-session/src/event_handler_consumer.rs` → no hit.
- `initialize_consumers` calls `event_handler_attrs.initialize_from_tree`; its doc no longer says the consumer needs no
  init.
- Both VM element setters call `note_handler_write` before `activate_event_handler`; no other VM code touches the marker
  outside tests.
- Clone-policy row added; module and field docs updated.
- Cells C1–C13, U1–U8 and pins E1–E7 present; every red cell and every pin was run red at `fd6955e8` and the failures
  recorded in the PR description; all pass at the head.
- `cargo test -p elidex-script-session --all-features`, `cargo test -p elidex-js --all-features`,
  `cargo test -p elidex-shell --all-features`, and `mise run ci` are green.

## §10. Open items for plan-review

- **O1: split this prerequisite further?** (PM condition 2; 9 axes, §2.0.) **Recommendation: split into slice a and
  slice b (§1.4).** Cut lines considered:
  1. *a / b by prerequisite* (recommended): a has none and carries G1 and N1's dependency; b waits for T3, N1, C1. No
     interim code; one guard condition changes (O2).
  2. *a further into a1 (once at insertion + writer marking; N1's dependency) and a2 (the bind pass + shell pins, G1)*:
     a2 needs a1's marker and adds one delegate plus the pins, so the cut buys little review isolation; not recommended,
     but clean (a1 leaves G1 at HEAD).
  3. *No split* (§1.3): zero interim, but the whole work waits behind T3, N1 (re-scoped) and C1, and reverses PM
     decision 17 (i).
- **O2: One issue, one way.** Under the split, slice a's processing point for post-bind creations is first insertion,
  and `note_handler_write` carries a guard ("no other deliverable attribute") that slice b deletes. No component,
  function, or call site is deleted later. Is this an acceptable increment, or an interim the rule forbids? The proposed
  slot text is in §1.4.
- **O3: walk cost** (K4). Slice a walks the bound document on every bind (H13). Measure it (a large parsed document,
  repeated `dispatch_event`) before approval of the implementation; if too costly, R10 (first bind only), which then
  needs G7 settled. The dense-marker alternative R9 is rejected for archetype churn; confirm.
- **O4: writer completeness.** Is §5.1 + §5.0 complete for handler state? In particular any VM path that writes
  `EventListeners` handler state outside `activate_event_handler`.
- **O5: N1's R2 requirements.** Under the split, R-a / R-b hold in slice a only for elements with one deliverable
  handler attribute (C4, C5). Is that sufficient for N1's landing, given that N1's own cells do not use multi-attribute
  elements (N1 memo §6)?
- **O6 (slice b): the Insert-arm fallback.** Keep it as a release fail-safe with a `debug_assert!` that fires when it
  finds an unmarked element with a deliverable handler attribute (a creation path that skipped processing), or delete
  it. This also answers rev 1's "silent `on*` writer guard" question.
- **O7: G7** — the PM decides whether to slot it after Q1.
- **O8: the N1 edits** (§8) — the PM routes them to N1's designer.
- **O9 (slice b): the creation-time call-site shape** — a dedicated `MutationEvent` variant fired by each silent build
  through the dispatcher it restores (the dispatcher holds the policy; the fragment build must fire it before
  `finish_detached_fragment`'s adopt, so step 3 sees the throwaway document), or direct calls from the engine-independent
  wrappers (`elidex-dom-api` clone, `elidex-form` DOMParser, the session's fragment apply), as `BaseUrlMaintainer` does
  for DOMParser (`inert_document.rs:196`).

Questions for a scout (facts this rev could not establish by its own reads):

- **Q1** Under the VM, can `SessionCore::flush` outside a bracket (`pipeline.rs:138-165`) carry a `SetAttribute` or a
  fragment insertion of an `on*`-bearing element? Does any shell path create such elements in the main `EcsDom` while
  unbound?
- **Q2** At every bind site, is the bound `document` equal to `dom.document_root()` (main pipeline and the three iframe
  builders)? The bind pass and T3 both assume it.
- **Q3** Does elidex fetch or fire `load` / `error` for an `<img>` in template contents or in a clone of them? (The size
  of §1.2 (c)'s exposure.)
- **Q4** Can `elidex-form` and `elidex-dom-api` call an `elidex-script-session` function (O9's second shape)?
- **Q5** Is there a hermetic harness for the same-origin URL iframe path (`load.rs:221`)?
- **Q6** Does any `MutationEvent` consumer match exhaustively, so that a new variant (O9's first shape) would need edits
  there?
