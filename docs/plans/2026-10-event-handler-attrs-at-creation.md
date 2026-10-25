# EH-attrs-at-creation — event-handler content attributes processed once, at the element's creation (umbrella memo)

Standalone program, label **`EH-attrs-at-creation`**. Owner: Layout lane. It sits **outside** the IBP-sandbox program and
is not part of that umbrella's DAG (PM decision 17 (i), restated by PM decision 21). It has two nested PRs, **slice a**
and **slice b** (§1). IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, rev 3 `0f99538d`, branch
`layout-ibp-sandbox-handler-gates`) depends on slice a (§1.3 edge x1). Rev 1's label `EH-insert-once` and file
`2026-10-event-handler-insert-once.md` are retired. The branch keeps its name `layout-eh-insert-once` (not pushed).

**Approval scope.** Approving this umbrella **decides**: the program's nested-PR set, their order and edges (§1.2–§1.3,
PM decision 21); which slice owns each invariant axis, each coupled pair, each contract clause and each gap (§1.1, §2,
§0.5's landing qualifiers, §7); each fact's home and representation kind (§4.1's *Home/kind* line); the D-items (§0.4);
the spec-semantics decisions (which algorithm, which steps, which outcome: §0.5, §3's *Step* and *Branch* columns); the
statement that slice a is not an interim and the one guard slice b deletes (§1.4, D-8); the E-pin paths and the
red-first rule (§6.2's *path* column and its first paragraph); the per-slice DoD (§10); and the slots and records (§8).
§4's mechanisms below the *Home/kind* line, §5's populations and named risks, §6's cell shapes and oracles, §3's
*Touch* column and §6.5's estimates are **input to each slice's own plan-review**, which re-derives them. A review
finding against that input changes this umbrella only when it changes a decision; otherwise it is recorded in the owning
slice's plan. The per-PR plan for slice a is a separate later memo; this umbrella is meant to be precise enough that it
can be derived from §0.5, §1, §2, §4.1–§4.2, §6.1–§6.2 and §10.1.

**Approval vehicle**: open (§11 O10).

**Rev 3** (2026-10-25). Folds PM decision 21: the memo becomes the program's umbrella. Slices a / b replace rev 2's
recommendation (§1.4 → §1); rev 2's O1 (split) and the landing order are decided (§11). D-8 fixes the shape of
`note_handler_write` so that the guard is the only line slice b deletes. G1's end-to-end pins are organised by path
(main document, URL-loaded iframe, srcdoc iframe), each red on the pre-fix base with a non-vacuity control (§6.2). The
coupled-invariant table lists all 36 pairs with an owning slice (§2.1). The rev 2 cells are assigned to slices with
nothing dropped; rev 2's §6.3 prose list becomes cells B1–B8 (§6.3). G7 stays a pending slot proposal (§8.2). The N1 edits
are a hand-off list for N1's rev 4 (§8.4). *Rev 2* (`00b863dd`): folded PM decision 18 (landing-order comparison, R8
over rev 1's `Pending` mechanism; G1 in scope). *Rev 1* (`28efbb67`): first draft.

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fd6955e8`** coordinate (the branch cut). origin/main is `ad80b5ca` at this rev and
`git diff --stat fd6955e8 ad80b5ca -- crates` is empty, so the coordinates hold on it too. Every code reference comes from
`/usr/bin/grep -n` (never the default ugrep) or `sed -n` over that tree, run on 2026-10-25 by this rev's author. Every
§ number ↔ title pair and every algorithm step comes from `.claude/tools/webref` (`heading`, `dfn`, `body`) run on
2026-10-25. N1's memo is cited at its rev 3 head `0f99538d` (`git show 0f99538d:docs/plans/2026-10-ibp-sandbox-handler-gates.md`;
read-only here).

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
| H13 | **Every** bind rebuilds the dispatcher and re-runs `initialize_consumers`; every unbind removes it | `sed -n 28,36p crates/script/elidex-js/src/vm/lifecycle.rs`; `/usr/bin/grep -rn --include='*.rs' 'with_bound(' crates/shell \| wc -l` | **true.** Unbind calls `clear_mutation_dispatcher` (`lifecycle.rs:34`); bind installs a fresh one (`vm_api.rs:292-314`). The shell has 15 `with_bound(` call sites (initial scripts `pipeline.rs:355`, per-event brackets `lib.rs:299-529`, `pipeline.rs:133`, `:165`). So a bind-time pass runs once per bracket and **must be idempotent per element** |
| H14 | At `initialize_consumers` the Window entity exists | `sed -n 213,230p` and `:313` of `vm_api.rs` | **true.** `we` is resolved or created (`:222-229`) before the dispatcher is built and initialised. `resolve_handler_target` (`event_handler_consumer.rs:328-334`) finds it through `dom.window_entity()` |
| H15 | Template contents are owned by the template's own document, and the clause-(b) composition does not suppress them | `sed -n 539,580p` and `sed -n 820,856p crates/core/elidex-ecs/src/dom/mod.rs`; `sed -n 95,113p crates/script/elidex-script-session/src/scripting.rs` | **true.** `attach_template_contents` (`mod.rs:560`) creates the fragment with `owner_doc` = the template's document (C2 approximation, `:551-557`; slot `#11-template-contents-owner-document`). For a node under that fragment, `scripting_disabled_for_platform_object` takes the composed root (`scripting.rs:104`), which is the fragment, not a Document, so it falls back to `owner_document` (`:108`), which follows the fragment's `AssociatedDocument` (`mod.rs:851-853`) to the page document: not suppressed. The doc comment names this false negative (`scripting.rs:59-60`) |
| H16 | Every shell pipeline builder parses first and binds in `run_scripts_and_finalize` | `/usr/bin/grep -n 'run_scripts_and_finalize\|parse_progressive_str' crates/shell/elidex-shell/src/pipeline.rs`; `/usr/bin/grep -rn 'build_pipeline_from_loaded(\|build_pipeline_interactive_shared(' crates/shell/elidex-shell/src` | **true.** `build_pipeline_interactive` (`:565`; parse `:567`, bind `:588`), `build_pipeline_interactive_with_network` (`:646`; `:656`, `:675`), `build_pipeline_interactive_shared` (`:735`; `:747`, `:771`), `build_pipeline_from_loaded` (`:839`; `:872`). Production main-document callers of `build_pipeline_from_loaded`: `content/mod.rs:673`, `content/navigation.rs:276`, `app/navigation.rs:323`. So one bind-time pass covers every builder |
| H17 | The three iframe paths | `sed -n 52,81p` and `/usr/bin/grep -n 'build_pipeline\|fn build_iframe_pipeline' crates/shell/elidex-shell/src/content/iframe/load.rs` | **true.** URL-loaded same-origin: `build_pipeline_from_loaded` (`load.rs:221`). URL-loaded out-of-process: `build_pipeline_from_loaded` (`:327`). srcdoc / about:blank / no-src (`:62-79`, taken when `iframe_data.srcdoc` is present or `src` is absent, `:54`): `build_iframe_pipeline` (`:465`) → `build_pipeline_interactive_shared` (`:471`; `pipeline.rs:735`), **not** `build_pipeline_from_loaded`. All three reach `run_scripts_and_finalize` (H16) |
| H18 | cloneNode's path | `sed -n 201,215p crates/script/elidex-js/src/vm/host/node_methods_extras.rs`; `/usr/bin/grep -n 'clone_node_with_shadow_honor\|clone_node_shallow\|attach_template' crates/dom/elidex-dom-api/src/node_methods/clone.rs`; `/usr/bin/grep -n 'fn clone_node_shallow\|builder.add(attrs)' crates/core/elidex-ecs/src/dom/tree_clone.rs` | VM `native_node_clone_node` (`node_methods_extras.rs:201`) → `invoke_dom_api("cloneNode")` → `elidex-dom-api` `node_methods/clone.rs:154` → `clone_node_with_shadow_honor` (`:227`) → `clone_recording` → `EcsDom::clone_node_shallow` (`clone.rs:323`; `tree_clone.rs:251`) → `clone_node_shallow_unchecked` (`tree_clone.rs:270`), whose silent `Attributes` copy is `builder.add(attrs)` at `tree_clone.rs:368`. The template post-pass attaches a fresh fragment (`clone.rs:422`) |
| H19 | The VM holds the policy the dispatcher is built with | `/usr/bin/grep -n 'spec_level_policy' crates/script/elidex-js/src/vm/mod.rs crates/script/elidex-js/src/vm/vm_api.rs crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | **true.** `VmInner::spec_level_policy` (`vm/mod.rs:2715`) is passed to `ConsumerDispatcher::new` (`vm_api.rs:296`), which builds `EventHandlerAttributeConsumer::with_policy` from it (`consumer_dispatcher.rs:97`, `:107`) |
| H20 | `AttributeChange` fires after the `Attributes` write | `sed -n 125,130p crates/core/elidex-ecs/src/dom/attribute.rs` | **true.** `set_attribute` (`:125`) calls `write_attribute_no_dispatch` (`:126`) and fires afterwards |

### §0.3 Premise errors (reported)

- **P1: `importNode` / `adoptNode` do not exist** in elidex (`/usr/bin/grep -rn -i --include='*.rs' -E
  'importnode|adoptnode' crates` → nothing). Insertion adoption itself is slot `#11-cross-document-adopt-on-insert`. This
  memo analyses only elidex's internal adopt, `EcsDom::adopt_subtree` (`teardown.rs:285`), used by fragment teardown.
- **P2: the Window-reflecting body set is not delegated** (G4). `onblur` `:68`, `onerror` `:90`, `onfocus` `:91`,
  `onload` `:99`, `onresize` `:125`, `onscroll` `:126` are `HandlerScope::Global`, so `resolve_handler_target` (`:328`)
  keeps them on the body. Merged by the PM into slot `#11-event-handler-target-determination` (facet 4). Not here.
- **P3: a docstring overstates a scripting rule** (`inert_document.rs:78-84`): the attribute change steps have no
  scripting condition; *get the current value of the event handler* step 3.2 nulls a raw value. The correction belongs to
  the slice that touches that file (slice b); slice a does not touch it (MINOR until then).
- **P4: `EcsDom::clone_attributes` is not the cloneNode writer** (`tree_clone.rs:57`; one caller, the test
  `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`). The cloneNode path is H18.
- **P5 (rev 2 brief): the iframe coordinates name URL-loaded iframes only.** `load.rs:221` / `:327` are the
  same-origin-URL and out-of-process paths; srcdoc iframes are built by `build_pipeline_interactive_shared` (H17).
- **P6 (rev 2 brief): `vm_api.rs:312-314`.** `ConsumerDispatcher::new` is at `:292`; `initialize_consumers` at `:313`,
  the install at `:314`.
- **P7 (rev 2 brief): `tree_clone.rs:368` is a line inside `clone_node_shallow_unchecked`** (defined at `:270`), not its
  definition (H18).
- **P8 (rev 2 brief): there is a third R8 blocker** (template contents, §9.2 (c)).
- **P9 (rev 2 brief): H-P13 need not be rewritten** (§9.2 (a)).
- **P10 (rev 3 brief): the "rev 2 premise errors" on `vm_api.rs:292` and `tree_clone.rs:270` / `:368` were already
  corrected in rev 2** (P6, P7, H9, H17 of rev 2). Rev 3 re-measured them (H9, H18) and they hold; nothing to fix.
- **P11 (rev 3 brief): slice b's prerequisites include N1**, not only T3 and C1. §9.2 row (a): a raw value that slice b
  writes at a DOMParser node's creation reads null only under N1's compile-gate node clause. Decision 21's order (slice
  a → N1 → …, and C1's handling decided after N1 lands) satisfies it transitively; §1.3 records it as its own edge (x4)
  so that a later re-ordering cannot drop it silently.
- **P12 (rev 3 brief): "the only thing slice b deletes is one guard condition" holds only if O6 keeps the Insert arm.**
  O6's "delete" alternative (rev 2 §10 O6) would also delete slice a's Insert-arm processing call. §1.4 states the
  conditional; §11 O6 carries it.
- **P13 (rev 3 brief): the brief's "G2 part in slice a" is "shadow-inclusive walks".** The remaining G2 case (an
  element inserted directly under a shadow root fires no Insert, H2) is slice b's (§7).

### §0.4 Decisions

- **D-1: once-key.** `HandlerAttrsProcessed` (NEW), a unit marker component on the **origin** element (never on the
  handler's target, the Window for `<body>` delegation). Meaning: *this element carries no handler content attribute
  whose creation-time value is still unprocessed.* Sparse: inserted only on an element that had a handler content
  attribute when processed. Absent = unprocessed (fails toward HEAD's delivery, never toward loss).
- **D-2: processing.** One engine-independent function, `process_handler_content_attrs` (NEW), runs the attribute
  change steps (WHATWG HTML §8.1.8.1) for every handler content attribute E carries, at most once per element (D-1),
  subject to the session's `SpecLevelPolicy`, with the target from `resolve_handler_target`.
  `process_handler_content_attrs_in_subtree` (NEW) applies it over a shadow-including subtree (H10).
- **D-3: timing.** E's creation-time handler content attributes are processed at the **earliest consumer sighting** of
  E: E's creation, when a consumer exists then (slice b); the first bind of E's DOM, for elements created before any
  consumer existed (G1; slice a); E's first handler write (D-4); E's first insertion (slice a's processing point for
  post-bind creations; after slice b, the fallback for a creation path that skipped processing, O6). Insertion never
  re-processes (D-1).
- **D-4: writers.** A handler write (the AttributeChange arm, and both VM element setters) first ensures E is
  processed (`note_handler_write`), then writes. Its shape is fixed by D-8.
- **D-5: bind.** `EventHandlerAttributeConsumer::initialize_from_tree` (NEW) processes the bound document's
  shadow-including tree; `ConsumerDispatcher::initialize_consumers` calls it. It is idempotent per element (D-1), which
  H13 makes mandatory. It stays after slice b (the pre-bind parse has no consumer at creation, H9).
- **D-6: VM surface.** The two element setters (`native_event_handler_set` `:344`, `native_body_weh_set` `:459`) call
  `note_handler_write` before `activate_event_handler`, passing `VmInner::spec_level_policy` (H19). Marshalling only
  (Layering mandate).
- **D-7: program shape (PM decision 21).** Two nested PRs, slice a and slice b, with the scope, edges and order of §1.
  This replaces rev 2's §1.4 recommendation and rev 2's O1.
- **D-8: the guard is one line, and it is the only line slice b deletes.** Slice a's `note_handler_write` is
  ```text
  if origin is not an element, or is marked: return
  if origin carries a deliverable handler content attribute other than on<event_type>: return   // the guard
  process_handler_content_attrs(policy, origin, dom)
  ```
  Slice b deletes the guard line and nothing else in this function. With the guard passing, processing touches at most
  `on<event_type>` itself, which the write that follows overwrites (H20: an AttributeChange write already holds the new
  value; an IDL write replaces the raw value through `activate_event_handler`), so slice a's outcome equals rev 2's
  "marker only" body. The guard is recorded under slot `#11-eh-attrs-creation-time-call-sites` (§8.2).
- **D-9: G1 pins by path, red first.** Slice a carries end-to-end pins on three paths: the main document, a URL-loaded
  iframe, and a srcdoc iframe (§6.2). Each pin is run on the pre-fix base and its failure is recorded before the fix
  lands; each has a non-vacuity control that is green on that base.

### §0.5 Contract (single citable statement, with landing qualifiers)

For an element E, "creation-time attributes" are the `Attributes` written without `MutationEvent::AttributeChange` at
E's creation (the four parser bakes and the clone copy, §5.0 (a)). "Handler state" is the `EventListeners` entry for an
event type on E's resolved target, plus its stored callable. "Processed" = the attribute change steps have run for E's
creation-time handler content attributes (D-1).

- **K-1, once.** E's creation-time handler content attributes are processed at most once. After that, no insertion,
  move, or rebind writes handler state derived from them. A compiled callable keeps its identity across moves and
  rebinds, in the live document and into a DOMParser document alike. *From slice a, for every element.*
- **K-2, an AttributeChange write wins.** Processing never overwrites a handler written by `setAttribute` /
  `removeAttribute` of `on<T>` on E. Its value survives, including a callable compiled from it. *From slice a for every
  element with at most one deliverable handler attribute; from slice b for every element.*
- **K-3, an IDL write wins.** Processing never overwrites a callable or null assigned through `E.on<T>` (or a `<body>`'s
  delegated WindowEventHandlers accessor), whatever wrote E's `Attributes` silently. *Same qualifier as K-2.*
- **K-4, timing.** For every creation path that runs while a consumer exists, processing happens at creation, before
  any script can observe E. For elements created with no consumer (the pre-bind parse), it happens at the first bind,
  before that bind's first script. Processing writes the internal raw uncompiled handler on the resolved target. *The
  bind arm from slice a; the creation arm from slice b.*
- **K-5, inert documents read null.** A raw value that processing writes on a node whose node document has a null
  browsing context (a DOMParser document, template contents) reads as null and never runs. *From slice b (needs N1's
  compile gate and C1).* Slice a writes no such value (it walks neither DOMParser documents nor template contents, U8).
- **K-6, bind completeness (G1).** At every bind, every element of the bound document's shadow-including tree is
  processed before any script of that bind runs. *From slice a.*

Red cells per clause (§6): K-1 C1–C3, C8, U1, U2, E3; K-2 C4, C11, U5, B1, B8; K-3 C5, U5, B1, B8; K-4 E1, E2, U7, B6;
K-5 B4, B5; K-6 E1, E2, E4–E7, U7.

## §1. Program shape (decided)

### §1.1 The nine invariant axes and their slices

| axis | source | slice a | slice b |
|---|---|---|---|
| I1 once (per element, across moves and rebinds) | K-1, H13 | owns it: marker, once check in the Insert arm and the bind pass | extends it to the creation-time call sites |
| I2 writer order (a write is never overwritten) | K-2, K-3 | restricted (D-8 guard) | full (guard deleted) |
| I3 origin key (state on the origin, not the target) | D-1 | owns it | — |
| I4 freshness (new elements start unprocessed; clone does not copy) | H8 | owns it (clone-policy row) | clone processed at its creation |
| I5 coverage (every creation path, the pre-bind tree, shadow trees, template contents) | K-4, K-6, G1, G2 | the pre-bind tree (bind pass), shadow-inclusive walks, first insertion | creation-time call sites, template contents, direct insertion under a shadow root |
| I6 policy | `event_handler_consumer.rs:405`, `:447` | owns it (same `policy.installs` filter in every path) | — |
| I7 inert-document reads (DOMParser, template) | K-5, H5, H15 | not reached (U8 (i), (ii)); C2's move into a DOMParser document keeps a callable | owns it (needs N1, C1) |
| I8 target determination (step 3) | §9.2 (b) | not reached (the bind pass walks only the active document) | owns it (needs T3) |
| I9 layering / re-entry | Layering mandate; `mutation_event.rs` | owns it for the VM setters and the bind pass | owns it for the new call sites (O9, Q4) |

**Count: 9** (≥ 3): edge-dense, so the work is a multi-PR program and each slice gets its own `/elidex-plan-review`
before implementation (CLAUDE.md *Edge-dense work* (a), (b)). Slice a holds seven axes (I1–I6, I9) and slice b seven (I1,
I2, I4, I5, I7, I8, I9; it owns I7 and I8 and extends the other five); neither is re-split, under the base case (c): each is a narrowly-scoped per-PR slice of
this approved umbrella, and its own plan-review decides whether it recurses.

### §1.2 Nested PRs (decided)

| nested PR | scope | prerequisites |
|---|---|---|
| **slice a** `EH-attrs-at-creation-a` (branch `layout-eh-insert-once`) | D-1 marker `HandlerAttrsProcessed` on the origin element; D-2 functions; the Insert arm processes once (K-1); the bind-time pass (D-5, G1, K-6); shadow-inclusive walks (G2's walk part); D-8 restricted writer marking in the AttributeChange arm and both VM setters (D-6); clone-policy row; G1 pins on three paths (§6.2) | none |
| **slice b** `EH-attrs-at-creation-b` | creation-time call sites for every bound creation path (fragment build, DOMParser build, clone, template contents; shape O9); D-8 guard deleted (unrestricted writers); G2's remaining case (direct insertion under a shadow root); G5; K2; the `inert_document.rs` docstring (P3); the Insert arm's role after slice b (O6) | T3, C1 (PM decision 21); N1 (P11) |

**Order (PM decision 21):** slice a → IBP-sandbox N1 → … ; slice b after T3 and C1. **Slice b's start: after C1's
handling is decided.** The PM decides C1's handling once slice a and N1 have landed.

### §1.3 Edges (decided)

| # | edge | kind | reason |
|---|---|---|---|
| x1 | slice a → IBP-sandbox N1 (external) | semantic precondition | N1's C8b(iv) needs a compiled handler to survive a move into a DOMParser document, and C8c's second half discriminates only when a re-insertion does not turn the value raw (N1 memo NR7, `0f99538d:575`). Slice a's K-1 gives both (cells C2, C3) |
| x2 | slice a → slice b | code predecessor + data flow | slice b extends slice a's marker, `process_handler_content_attrs` and `note_handler_write` with creation-time call sites and deletes the D-8 guard; it adds nothing that replaces them |
| x3 | T3 (`#11-event-handler-target-determination` step 3; owner Layout) → slice b | semantic precondition | slice b processes a DOMParser (or fragment-throwaway) `<body>` at creation; without step 3 its WindowEventHandlers attributes land on the **page** Window (§9.2 (b), I8) |
| x4 | N1 → slice b | semantic precondition (P11) | slice b writes raw values at DOMParser-node creation; they read null only under N1's compile-gate node clause (§9.2 (a), I7) |
| x5 | C1 (`#11-template-contents-owner-document`; no driver; deliberately deferred) → slice b | semantic precondition; slice b starts after C1's **handling is decided** | slice b processes template contents at creation; without C1 the composition does not suppress them, so they compile and run (§9.2 (c), H15, I7) |

Acyclic. x4 is satisfied transitively by the order (x1, and x5's decision point after N1 lands); it is listed so that a
re-ordering cannot drop it.

### §1.4 Slice a is not an interim

CLAUDE.md "One issue, one way" forbids an interim that a later PR deletes. Slice a is an increment, not an interim:

- **Kept by slice b unchanged:** `HandlerAttrsProcessed` and its meaning (D-1); `process_handler_content_attrs` and the
  subtree form (D-2); the bind pass (D-5; still the only point for the pre-bind parse); the shadow-inclusive walk; the
  clone-policy row; the `note_handler_write` call sites in the AttributeChange arm and both VM setters (D-6).
- **Extended by slice b:** new call sites of `process_handler_content_attrs` at creation (O9).
- **Deleted by slice b:** exactly one line, the D-8 guard in `note_handler_write`. Its cell U5 (ii) inverts in slice b
  (an element with two deliverable attributes becomes marked on a write).
- **Conditional (P12):** if O6 decides to delete the Insert-arm processing call after slice b, that call is a second
  deleted item and this section's "exactly one line" no longer holds; O6 = keep (with its detector) preserves it.

Whether that guard is an acceptable increment or an interim the rule forbids is §11 O2.

### §1.5 What slice a leaves open until slice b (accepted windows)

Every window below exists at HEAD; slice a introduces none. Each equals HEAD's outcome or is closer to the spec.

- **K-2 / K-3 residual:** a pre-insertion handler write on a never-inserted post-bind element with **two or more**
  deliverable handler attributes is overwritten at first insertion (the D-8 guard; B1, B8).
- **G5 / K3:** a never-inserted clone reads null (B2).
- **K2:** the body-clone cross-origin write (B3).
- **G2 rest:** an element inserted directly under a shadow root is not processed (B6).
- **G3:** a DOMParser `<body>` with a WindowEventHandlers attribute is delivered to the page Window at its first
  insertion (HEAD; B5).
- **Template contents:** processed when moved out (HEAD; B4).

## §2. Coupled invariants

### §2.1 All pairs (36), with owner

*Owner* = the slice whose plan resolves the pair; *a → b* = slice a resolves it for its scope and slice b re-derives it
for the creation-time call sites.

| pair | intersect? | owner | resolution |
|---|---|---|---|
| I1×I2 | yes | a → b | A processed element is marked; writers act on its entries directly (HEAD). `note_handler_write` on a marked element is a no-op. Cells C1, C4, U5 (iii) |
| I1×I3 | yes | a | The marker is per origin. A second `<body>` (a clone) is unmarked and processes once to the Window. Cells C8, U2 |
| I1×I4 | yes | a → b | Clone does not copy the marker (H8; clone-policy row). Slice b processes the clone at its creation. Cells C9, U6, B2 |
| I1×I5 | yes | a → b | The bind pass and the Insert arm call the same function; the marker makes the bind pass idempotent across H13's repeated binds. Slice b's creation sites call it too. Cells E3, U7 |
| I1×I6 | yes | a | The marker is set when the element has any handler content attribute, excluded or not, so an excluded-only element is not rescanned. Cell U3 |
| I1×I7 | yes | a, b | Slice a: a compiled callable keeps its identity when moved into a DOMParser document (C2, N1's C8b(iv)). Slice b: a DOMParser node processed at creation is marked, so a later move into the page does not re-process it; its raw value compiles there (the spec's outcome) |
| I1×I8 | yes | b | A DOMParser `<body>` processed at creation under T3 writes nothing and is marked; a move into the page does not deliver it (insertion runs no change steps). Cell B5 |
| I1×I9 | yes | a | The once check is inside `process_handler_content_attrs`; the VM has no once logic |
| I2×I3 | yes | a | `note_handler_write` takes the `<body>` receiver for a delegated write (D-6). Cell C8 |
| I2×I4 | yes | a | A clone starts unmarked; a write on the source never marks the clone. Cell C9 |
| I2×I5 | yes | a → b | Slice a: a write on an unmarked element whose only deliverable attribute is the written one processes and marks it, so first insertion skips it (C4, C5, C11); with two or more, the guard returns and first insertion processes all (§1.5). Slice b deletes the guard (B1, B8) |
| I2×I6 | yes | a | "Deliverable" = in `EVENT_HANDLER_ATTRS` and installed by the policy. An excluded name never reaches `note_handler_write` from the AttributeChange arm (returns at `:405-407`); its IDL setter is not installed |
| I2×I7 | yes | b | Unrestricted ensure-before-write on a DOMParser node materialises its siblings as raw values, which read null only under N1 (x4). This is why the guard exists in slice a (R7) |
| I2×I8 | yes | b | The same ensure on a DOMParser `<body>` would deliver its WindowEventHandlers siblings to the page Window without T3 (x3) |
| I2×I9 | yes | a | Both VM setters call the engine-independent `note_handler_write` (D-6) |
| I3×I4 | yes | a | A clone of a `<body>` is a new origin: unmarked, processed once to the Window at its first insertion. Cells U2, C9 |
| I3×I5 | yes | a | The bind pass resolves `<body>` WindowEventHandlers rows to the Window, which exists at bind (H14). Cell E4 |
| I3×I6 | yes | a | An excluded WindowEventHandlers attribute (`onstorage` under `BrowserCore`) marks the `<body>` and writes nothing to the Window. Cell U3 |
| I3×I7 | yes | b | The marker for a DOMParser `<body>` is on that body; its target is decided by T3, its reads by N1 |
| I3×I8 | yes | b | Slice a keeps HEAD's target (no step 3); the bind pass reaches only the active document, where step 3 does not fire. Slice b processes inactive-document bodies at creation, which needs step 3 |
| I3×I9 | yes | a | The origin is chosen in engine-independent code; `native_body_weh_set` passes the `<body>` receiver, not the Window (§4.2 row 4) |
| I4×I5 | yes | a → b | Fragment-parsed nodes and clones are processed at first insertion in slice a (H6 placement), at creation in slice b. Cell C10 |
| I4×I6 | no | — | Clone does not copy the marker whatever the policy; the policy filter applies at processing (I1×I6, I1×I4 each alone) |
| I4×I7 | yes | b | A clone of a DOMParser node keeps the DOMParser document as owner (`clone_subtree` threads `owner_document(src)`, `tree_clone.rs:208`); processed at creation, its raw values read null under N1. A clone of template content keeps the page document as owner, which is why C1 is needed (§9.2 (c)) |
| I4×I8 | yes | b | A clone of a DOMParser `<body>` processed at creation resolves through step 3 to null |
| I4×I9 | yes | a | The marker's non-copy is a row of `elidex-ecs`'s clone-policy table that names an `elidex-script-session` component; `elidex-ecs` copies an explicit set, so no `elidex-ecs` code changes (§4.2 row 6) |
| I5×I6 | yes | a | The bind pass and the Insert arm apply the same `policy.installs` filter as HEAD (`:447`). Cell U3 |
| I5×I7 | yes | a, b | Slice a's bind pass starts at `dom.document_root()` (`dom/mod.rs:425`), so it never reaches a DOMParser document (created by `create_document_node`, which does not set the root, `:397-418`), and it does not walk template contents (H10). H-P13 stays true. Cell U8. Slice b reaches both at creation (x4, x5) |
| I5×I8 | yes | b | Slice b's DOMParser and fragment-build call sites run while the node document is inactive; step 3 must fire there (for the fragment build, before `finish_detached_fragment`'s adopt, O9) |
| I5×I9 | yes | a → b | The bind pass runs inside `initialize_consumers`, before the dispatcher is installed, so no `MutationEvent` re-enters (`vm_api.rs:313-314`). Slice b's call sites: O9, Q4, Q6 |
| I6×I7 | no | — | The policy filter is independent of the node document |
| I6×I8 | no | — | The policy filter runs before target resolution, on the name only |
| I6×I9 | yes | a | The VM passes `VmInner::spec_level_policy`, the value the dispatcher's consumer is built with (H19), so the writers and the consumer apply one policy |
| I7×I8 | yes | b | Both are null-reading for inert documents; slice b needs both prerequisites (x3, x4, x5) |
| I7×I9 | yes | b | The DOMParser call site is in `elidex-form`, the clone site in `elidex-dom-api`; whether they may call `elidex-script-session` is Q4 (O9's second shape) |
| I8×I9 | no | — | Step 3 lives in `resolve_handler_target` (engine-independent); no VM code is involved |

Count: 36 pairs; 32 intersect, 4 do not (I4×I6, I6×I7, I6×I8, I8×I9). Of the 32: slice a alone 14, slice b alone 10,
*a → b* 6, *a, b* 2 (re-count: `awk '/^### §2.1/{f=1} /^## §3/{f=0} f && /^\| I[0-9]×I[0-9]/' <memo> | awk -F'|' '$3 ~ /yes/
{print $4}' | sort | uniq -c`).

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | event handler content attribute *attribute change steps* 1–5.6 | step 1 non-handler name → return; step 3 null target → return; step 4 value null → deactivate; step 5.5 set raw uncompiled handler; step 5.6 activate | a: `process_handler_content_attrs` (NEW) runs them for creation-time attributes; `handle_attribute_change` `crates/script/elidex-script-session/src/event_handler_consumer.rs:389` for writes (calls `note_handler_write` first). b: the same function at creation | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determining the target of an event handler* 1–4 | non-body → self; not WEH and not the reflecting set → self; step 3 inactive node document → null; else relevant global | `resolve_handler_target` `event_handler_consumer.rs:328` (unchanged by either slice; step 3 is T3 and the reflecting set is facet 4, both slot `#11-event-handler-target-determination`; b depends on T3) | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4 | target null → return; value null → deactivate; else set value + activate | a: `native_event_handler_set` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:344`, `native_body_weh_set` `:459` → `note_handler_write` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3.2 | raw value in a null-browsing-context document → null | `ensure_event_handler_current` `event_handler_attrs.rs:521` (unchanged; N1's node clause; template contents need C1; b's K-5) | ✗ | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers rows | WEH rows on `<body>` delegated (HEAD) | `EVENT_HANDLER_ATTRS` `event_handler_consumer.rs:61`; cells C8, E4 use `onhashchange` | ✗ | no |
| WHATWG DOM §4.9 Interface Element | *append an attribute* step 4 (*handle attribute changes*) | every attribute present at creation runs the attribute change steps once | `process_handler_content_attrs` (NEW): the realisation of those per-attribute runs | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token*, "Append attribute to element" | parser path: every token attribute is appended at creation | parser bakes `Attributes` at spawn (`crates/dom/elidex-html-parser/src/convert.rs:258`; strict `tree_builder/insert.rs:61`, `:119`, `shadow.rs:162`); a: processed at bind (pre-bind) or at first insertion; b: at fragment / DOMParser build | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | the fragment's new `Document` | fragment elements are created in a throwaway document | `begin_detached_fragment` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:318` (unchanged in a; b's call site, O9) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5 | a clone's attributes are appended at its creation | a: clone-policy table `crates/core/elidex-ecs/src/dom/tree_clone.rs:19-34` (row added: marker non-copy); b: processing at clone time (H18 path) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* | node document changes; no event-handler step | `EcsDom::adopt_subtree` `teardown.rs:285` (unchanged; the marker survives) | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert*, *insertion steps* | HTML defines no event-handler insertion steps | a: `handle_insert` `event_handler_consumer.rs:427` → process once | ✓ | yes |
| WHATWG DOM §4.8 Interface ShadowRoot | *shadow-including descendant* | walks include shadow trees | a: `for_each_shadow_inclusive_descendant` `crates/core/elidex-ecs/src/dom/tree/walkers.rs:33` | ✓ | yes |
| WHATWG HTML §4.12.3 The template element | *template contents*; *appropriate template contents owner document* | contents are not descendants; their owner document has a null browsing context | a: not walked (H10); b: at creation, after C1 | ✗ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` | `text/html` branch only | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (unchanged in a; b's call site) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter | also outerHTML / setHTMLUnsafe | fragment placement fires Insert (H6; unchanged in a; b processes at build) | ✗ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author-controlled markup or script values. The mechanism stores no user string: the marker is a
unit component, and the handler sources it writes are HEAD's (`set_inline_handler`). Slice a's bind pass newly makes
initial-HTML handler sources reachable by the compiler, which is the spec's behaviour (they are raw values at creation);
they compile only through HEAD's compile gate. No row adds a parse of user input.

## §4. Mechanism

**Home/kind (decided):** the once-key is a unit marker **component** on the origin element, homed in
`crates/script/elidex-script-session/src/event_handler_consumer.rs` beside the consumer. It is `Send + Sync`,
per-entity, and not a per-VM identity handle, so per CLAUDE.md's side-store→component rule it is a component. Despawn
removes it; adopt keeps it; clone does not copy it. Everything below this paragraph is input to the slices'
plan-reviews.

### §4.1 Items (slice a)

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

- **`process_handler_content_attrs(policy, element, dom)`**: return if `element` is not an element or is marked. Collect
  its attributes named in `EVENT_HANDLER_ATTRS` (attribute order). If none, return (no marker). For each: skip it if the
  policy excludes it (`:447`'s filter); otherwise resolve the target (`resolve_handler_target`) and call
  `set_inline_handler`. Then insert the marker. Collect-then-write (HEAD's borrow discipline).
- **`process_handler_content_attrs_in_subtree(policy, root, dom)`**: collect `root` and its shadow-including descendants
  (`for_each_shadow_inclusive_descendant`, H10), then process each. Template contents are not visited (H10).
- **`note_handler_write(policy, origin, event_type, dom)`**: D-8's three lines.
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

### §4.3 Slice b's mechanism (input to its plan)

- Delete the D-8 guard line (§1.4).
- Call `process_handler_content_attrs_in_subtree` at the end of each bound creation path: the fragment build (before
  `finish_detached_fragment`'s adopt, so step 3 sees the throwaway document), the DOMParser build, the clone (H18), and
  template-contents attachment. The call-site shape is O9.
- Decide the Insert arm's role (O6).
- Correct `inert_document.rs:78-84` (P3).

### §4.4 Alternatives rejected

- **R-D3: rev 1's mechanism** (`HandlerAttrDelivery::Pending { superseded }` → `Delivered` at first insertion).
  Rejected: §9.3 shows R8 lands with no interim, and `Pending` exists only to bridge a deferral that R8 removes; with G1
  it would also be a second timing beside bind (§9.2 G1 paragraph). Its "absence = undelivered, fail toward HEAD"
  property is kept by D-1.
- **R1: a new `MutationEvent` (or AttributeChange) at every spawn.** AttributeChange at creation fans out to every
  consumer (custom-element `attributeChangedCallback`, canvas, base URL, form, MutationObserver records), changing each
  one's meaning. A dedicated creation event is one of slice b's two candidate call-site shapes (O9), not slice a's.
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
  slice b's form, after those land.
- **R9: a dense marker** (mark every element at first sight, so writers need no rule). It avoids D-8's guard, but it
  costs one archetype move per element ever inserted or bound, for a marker whose information is "nothing to do" (O3).
- **R10: bind pass at the first bind only** (no per-element once-key at bind). It would avoid the per-bind walk, but it
  misses elements created or written while unbound after the first bind (G7, reachability pending). Kept as the
  fallback if O3's measurement says the walk is too costly.

### §4.5 ECS-native and Layering checks

- Idiom mapping: the OO "element.attributesProcessed flag" → a marker component on the entity; "process the document at
  attach" → a system over the bound tree run at the dispatcher's install point; "writers notify the element" →
  writers calling one engine-independent function. No registry, no new dispatcher variant, no side-store.
- `vm/host/` adds two calls into `elidex_script_session` (marshalling). `consumer_dispatcher.rs` is the composer, not
  host code; it gains one delegate call, like `base_url`'s.
- Inside `dispatch` the consumer writes only `EventListeners` and the marker (I9).

### §4.6 Docstring citation table (Axis 4)

| doc site | citation |
|---|---|
| `HandlerAttrsProcessed`, `process_handler_content_attrs` | WHATWG HTML §8.1.8.1 Event handlers (attribute change steps); WHATWG DOM §4.9 Interface Element (*append an attribute* step 4) |
| `process_handler_content_attrs_in_subtree` | WHATWG DOM §4.8 Interface ShadowRoot (*shadow-including descendant*); WHATWG HTML §4.12.3 The template element (contents are not descendants) |
| `note_handler_write` | WHATWG HTML §8.1.8.1 (IDL setter steps; attribute change steps 4–5) |
| `initialize_from_tree`, `initialize_consumers` doc | WHATWG HTML §13.2.6.1 Creating and inserting nodes (attributes appended at creation) |
| clone-policy row | WHATWG DOM §4.4 Interface Node (*clone a single node* step 2.5) |

## §5. Populations and named risks (input)

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
| main-document parse (pre-bind) | `pipeline.rs:850-858` (`build_pipeline_from_loaded`; production callers H16) | **first bind** (G1) | same |
| URL-loaded iframe parse (pre-bind) | `load.rs:221` (same-origin), `:327` (out-of-process), both `build_pipeline_from_loaded` | **first bind** (G1) | same |
| srcdoc / about:blank iframe parse (pre-bind) | `load.rs:465` → `:471` → `pipeline.rs:735` | **first bind** (G1) | same |
| fragment (innerHTML / outerHTML / setHTMLUnsafe) | `teardown.rs:318`; `convert.rs:258`; strict `insert.rs:61`, `:119` | first insertion (placement, H6) | fragment build end |
| DOMParser | `inert_document.rs:110-200` | first insertion into a document after return | build end (after N1, T3) |
| `createElement` + `setAttribute` | `dom/mod.rs:344` + AttributeChange | the write processes and marks (D-8) | same |
| `cloneNode` | H18 | first insertion | clone time |
| declarative shadow (parser) | `shadow.rs:162` | bind (pre-bind) / host insertion (fragment) — shadow-inclusive walk | creation |
| template contents | `dom/mod.rs:560` | when moved out (Insert per child, HEAD) | creation (after C1) |

### §5.4 Interaction matrix (slice a)

| interaction | slice a outcome |
|---|---|
| innerHTML / outerHTML / setHTMLUnsafe | fresh elements, processed once at placement |
| cloneNode | fresh, unmarked (I4); processed at first insertion; a write on the clone whose only deliverable attribute is the written one marks it (C5, C11) |
| importNode / adoptNode | not exposed (P1; `#11-cross-document-adopt-on-insert`); `adopt_subtree` keeps the marker |
| shadow trees (G2) | a host's shadow tree is processed with the host (bind, insertion). An element inserted directly under a shadow root fires no Insert (H2) and is processed at creation only in slice b |
| `<body>` WEH delegation | marker on the `<body>`; target the Window (`:328`). Cells C8, E4 |
| policy-excluded attributes | filtered as `:445-449`; they count toward the marker. Cell U3 |
| attribute removed while outside the tree | AttributeChange clears; processing reads `Attributes`, so the removed name is never processed (C12) |
| repeated binds | the bind pass skips marked elements (E3, U7) |

### §5.5 Named risks

- **K1: an unknown creation path.** An element baked by a path that slice b forgets stays unmarked and is processed at
  the next bind (if in the bound tree) or its first insertion: HEAD-like, never loss. Slice b's detector (O6).
- **K2: body-clone cross-origin write** (`c = document.body.cloneNode(); window.onhashchange = fn; append(c)`): the spec
  processes `onhashchange` at clone creation and `fn` wins; slice a processes at insertion and the raw value wins.
  Pre-existing; closes in slice b (B3).
- **K3: detached reads** (G5): a never-inserted clone of a live element reads null (spec: a compiled function).
  Pre-existing; closes in slice b (B2).
- **K4: walk cost.** The Insert arm keeps HEAD's walk (now shadow-inclusive) and adds one `get::<&HandlerAttrsProcessed>`
  per element. The bind pass walks the bound document's shadow-including tree on every bind (H13), like
  the base-URL maintainer's `initialize_from_tree` (`document_base.rs:276-286` → `initialize_base_url_for_document` `:203`).
  O3.
- **K5: G1 exposes G4.** Initial-HTML `<body onload>` / `onfocus` / … are now delivered, onto the **body** (P2), where
  `load` never arrives and `focus`-type events at the body run them. Before slice a nothing was delivered. The fix is
  facet 4 of `#11-event-handler-target-determination`, independent of this program.

## §6. Test and coverage plan

*Red* = must fail on the slice's pre-fix base and pass after; the implementer runs each red cell against that base
first and records the failure in the PR description (red-first). *Guard* = passes at both. The "HEAD" predictions come
from H1 / H3 / H9 and are not measurements. Cell shapes and oracles are input; the assignment of cells to slices is
decided. Slice a's pre-fix base is its branch base (`fd6955e8` at this rev).

### §6.1 Slice a: VM integration and unit cells

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
| U5 | red | (i) unmarked element with only `onclick=A`: `set_attribute(onclick, B)` → marked; (ii) with `onclick=A onmouseover=C`: `set_attribute(onclick, B)` → unmarked (**the D-8 guard's pin; slice b inverts it**); `note_handler_write` on (iii) a marked element → no change | (i) marked, then append leaves `click` taken-state intact; (ii) unmarked; (iii) unchanged |
| U6 | guard | `clone_node_shallow` of a marked element | clone unmarked |
| U7 | red (G1) | tree built **without** a dispatcher (pre-bind shape) containing `<button onclick=A>`; call `initialize_from_tree` twice, with `take_uncompiled` between | after the first call the entry is raw A and the button marked; after the second the entry is still `None` (not re-raw) |
| U8 | guard | `initialize_from_tree` with (i) a DOMParser-shaped second document (`create_document_node`) holding `<div onclick>`, (ii) a `<template>` whose contents hold `<div onclick>`, (iii) a shadow host with `<button onclick>` in its shadow root | (i), (ii) no entry and unmarked (H-P13, H10); (iii) delivered |

### §6.2 Slice a: end-to-end pins (G1), by path

New file `crates/shell/elidex-shell/src/content_initial_handler_tests.rs` (NEW), registered in
`crates/shell/elidex-shell/src/content/mod.rs` beside `iframe_security_tests` (`:796-797`). `synth_cross_origin_loaded`
(`content_iframe_security_tests.rs:399`) and `run_oop_and_collect_posts` (`:423`) move into `content_test_support.rs`
(288 lines) so both files use them.

**Red-first rule (decided).** Each pin is committed and run on slice a's pre-fix base before the fix commit, and its
failure output is recorded in the PR description. A pin counts as red only if its **non-vacuity control** (a value the
initial script itself writes, so the harness and the script ran) is green on that base. Consumer-level cells (U7) do
not substitute for any pin.

Shared markup (`M`): `<body><button id=b onclick="document.body.dataset.hit=(+document.body.dataset.hit||0)+1"></button><script>document.body.dataset.ran=1;document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script></body>`.
Control: `data-ran == "1"`.

| path | pin | builder (H16, H17) | shape | base (pre-fix) | oracle |
|---|---|---|---|---|---|
| main document | E1 | `build_pipeline_from_loaded` (`pipeline.rs:839`; production callers `content/mod.rs:673`, `content/navigation.rs:276`, `app/navigation.rs:323`) | `synth_cross_origin_loaded(M, url)` passed to `build_pipeline_from_loaded` | `data-ran == "1"`, no `data-hit` | `data-hit == "1"` after the build (script-initiated dispatch inside the initial-scripts bind); `probe_attr` (`content_test_support.rs:273`)-style read |
| main document | E2 | same | then `PipelineResult::dispatch_event` (`lib.rs:293`) with a click at `b` | `data-hit` absent | `data-hit == "2"` (UA dispatch in its own bracket) |
| main document | E3 | same | `eval_script("window.f=document.getElementById('b').onclick; typeof window.f")`; `dispatch_event` (another bind); `eval_script("document.getElementById('b').onclick===window.f")` | first read `"object"` | `"function"`, then `true` (K-1 across H13's rebinds) |
| main document | E4 | same | `<body onhashchange="document.body.dataset.h=1"><script>document.body.dataset.ran=1;dispatchEvent(new Event('hashchange'))</script>` | `data-ran == "1"`, no `data-h` | `data-h == "1"` (delivered to the Window at bind, H14) |
| main document | E7 | same | `<div id=h><template shadowrootmode=open><button id=b onclick="…as M…"></button></template></div><script>document.body.dataset.ran=1;h.shadowRoot.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>` | `data-ran == "1"`, no `data-hit` | `data-hit == "1"` (G2's walk part) |
| URL-loaded iframe | E5 | `build_pipeline_from_loaded` via the out-of-process entry (`load.rs:327`) | `make_out_of_process_entry(synth_cross_origin_loaded(…, "https://other.example/"), PreEvalFrameState { unsandboxed, … })` with `<button id=b onclick="postMessage('eh-hit','*')"></button><script>postMessage('ran','*');document.getElementById('b').dispatchEvent(new MouseEvent('click'))</script>`; `run_oop_and_collect_posts` | posts = `["ran"]` | posts contain `"ran"` and `"eh-hit"` (`"ran"` is the control) |
| URL-loaded iframe | E5b (conditional, Q5) | `build_pipeline_from_loaded` via the same-origin URL entry (`load.rs:221`) | E5's markup through a hermetic same-origin loader, if Q5 finds one | as E5 | as E5. If Q5 finds none, E5b is not written and the PR description states that `:221` and `:327` call the same builder, which E5 pins |
| srcdoc iframe | E6 | `build_iframe_pipeline` (`load.rs:465`) → `build_pipeline_interactive_shared` (`load.rs:471`; `pipeline.rs:735`) — **not** `build_pipeline_from_loaded` | `build_test_content_state_with_url` (`content_test_support.rs:147`) with `<iframe srcdoc='…M…'>` (the shape of `content_iframe_security_tests.rs:107`); read the in-process child's body | child `data-ran == "1"`, no `data-hit` | child `data-hit == "1"` |

### §6.3 Slice b's cells (assigned; shapes are input to its plan)

| cell | kind | shape | oracle | needs |
|---|---|---|---|---|
| B1 | red | DOMParser `<div id=d onclick=A onmouseover=C>`; `d.setAttribute('onclick', X)`; compile; insert; and the IDL form `d.onclick = fn` (N1 R2's R-a / R-b, multi-attribute) | compiled X (or `fn`) survives; `mouseover` raw C | N1 |
| B2 | red (G5) | `c = t.cloneNode()` of a live `onclick=A` element, never inserted | `typeof c.onclick === 'function'` | — |
| B3 | red (K2) | `c = document.body.cloneNode(); window.onhashchange = fn; el.append(c)` | `window.onhashchange === fn` | — |
| B4 | red (I7) | `tpl.content.querySelector('b').dispatchEvent(new Event('click'))` before a move out; then moved out | null / does not run before; compiles and runs after | C1 |
| B5 | red (I8) | DOMParser `<body onpageshow=…>`, before and after a move into the page | no Window entry either time | T3 |
| B6 | red (G2 rest) | `sr.appendChild(x)` of an `onclick` element directly under a shadow root; `sr.innerHTML` | processed at creation; dispatch runs it | — |
| B7 | per O6 | the Insert-arm detector: a test-only creation path that skips processing | `debug_assert!` fires (if O6 = keep) | — |
| B8 | red | live post-bind `c` with `onclick=A onmouseover=C`, never inserted; `c.onclick = fn` (and the `setAttribute` form); `el.append(c)` | `c.onclick === fn` (the D-8 guard's residual) | — |
| U5 (ii) | inverted | slice a's U5 (ii) | marked after the write | — |

### §6.4 Kept green, mutation guards, WPT (input)

Kept green: `tests_event_handler_attrs.rs` (35 tests: `/usr/bin/grep -c '#\[test\]'
crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` = 35) and the consumer's `spec_level_tests`
(`event_handler_consumer.rs:500`).

Mutation guards (slice a): dropping the marker check in `process_handler_content_attrs` fails C1–C3, C8, U1, U2, E3,
U7; deleting the bind delegate fails E1, E2, E4–E7, U7; a light-tree walk instead of the shadow-inclusive one fails
C10, E7, U8 (iii); a `note_handler_write` that never processes fails C2, C4, C5, C11, U5 (i); one without the guard fails
U5 (ii); the bind pass starting anywhere but `document_root` (or walking template contents) fails U8 (i) / (ii).

WPT: no subset is claimed (engine-unit, VM-integration and shell end-to-end only).

### §6.5 Touch-time 1000-line check, slice a (`wc -l` at `fd6955e8`)

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

## §7. Gaps: dispositions (decided)

Inclusion criterion (PM decision 18 (2)): a gap whose root cause is the delivery mechanism this program builds is in
scope; any other becomes a slot proposal.

- **G1, initial-HTML handlers never wired** (H9). **In scope, slice a** (D-5; E1–E7, U7). Root cause: elements created
  before any consumer existed are never processed (I5). Confirmed by static trace (H9, H16, H17) and by the absence of
  any shell test (§5.2).
- **G2, shadow-tree elements never delivered** (H2, H10). **In scope.** Slice a: every walk shadow-inclusive (C10, E7,
  U8 (iii)). Slice b: the direct-under-shadow-root case, by creation-time processing (B6). `fire_after_insert`'s
  shadow-root filter (`mutation.rs:289`) is **not** changed: it serves every consumer, and slice b does not need it.
- **G3, target step 3 not modelled.** Slot `#11-event-handler-target-determination` (facet 1). Not here; it is slice
  b's prerequisite T3 (x3).
- **G4, the Window-reflecting body set.** Slot `#11-event-handler-target-determination` (facet 4, merged by the PM).
  Not here. Interaction: K5.
- **G5, reads never deliver** (K3). **In scope, slice b** (B2). Root cause: processing deferred past creation.
- **G6, stale docstring** `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23` (says the deferred session
  flush writes `Attributes` without the chokepoints; `apply_set_attribute` now routes through `EcsDom::set_attribute`,
  `mutation/mod.rs:897-909`). Not touched by either slice: **MINOR**.
- **G7, writes and creations while unbound are not observed.** With no dispatcher installed (H13), a
  `setAttribute('onclick', …)` applied by an unbound `session.flush` (`pipeline.rs:138-165`: `flush_with_ce_reactions`
  runs `session.flush(dom)` outside any bracket) would fire AttributeChange to no consumer. Root cause: dispatcher
  absence while unbound, not this mechanism (the bind pass mitigates it only for unmarked elements). **Pending slot
  proposal** (§8.2): a scout is checking reachability; not decided here (O7).

## §8. Slots, records, hand-offs

**Registration timing**: a gap that exists at HEAD is registered when the PM accepts the proposal; a gap created by code
a slice adds is registered at that slice's landing. The PM registers every slot; this memo writes defect texts only.

### §8.1 Existing slots cited

- `#11-event-handler-target-determination` (registered by the PM 2026-10-25; facets (1) step 3, (2) step 4, (3)
  frameset, (4) the Window-reflecting body set). Facet (1) is **T3**, slice b's prerequisite (x3). Facet (4) is G4 (K5).
- `#11-template-contents-owner-document` (carved from #380) is **C1**, slice b's prerequisite (x5). No driver;
  deliberately deferred; the PM decides its handling after slice a and N1 land.
- `#11-cross-document-adopt-on-insert`: importNode / adoptNode and insertion adoption (P1). Not touched.

### §8.2 Proposed slot texts (for the PM; defects only)

> **`#11-eh-attrs-creation-time-call-sites`** — owner Layout lane; created by slice a of `EH-attrs-at-creation`
> (register at slice a's landing). After slice a, an element created after bind is processed at its first insertion,
> not at its creation. Defects: (1) a handler write (`setAttribute` / `removeAttribute` of `on<T>`, or `E.on<T> = …`)
> on a never-inserted element with two or more deliverable handler content attributes is overwritten at the element's
> first insertion, because `note_handler_write` (`crates/script/elidex-script-session/src/event_handler_consumer.rs`)
> returns early for such an element (the guard line; it disappears in slice b's PR, when slice b lands); (2) a
> never-inserted clone of an element with a handler content attribute reads `null` from `on<T>`; (3) after
> `c = document.body.cloneNode()`, a `window.on<T>` assignment is overwritten by `c`'s attribute at `c`'s first
> insertion; (4) an element with a handler content attribute inserted directly under a shadow root is never processed;
> (5) DOMParser and template-content elements are processed at their first insertion into another document, not at
> creation. Trigger: T3 (`#11-event-handler-target-determination` facet 1) and N1 landed, and C1's handling decided.

> **G7 (pending; name proposed `#11-unbound-attribute-writes-unobserved`)** — owner to be decided. While no mutation
> dispatcher is installed (between VM brackets, H13), an attribute write or a node insertion fires `MutationEvent`s to
> no consumer, so an `on*` attribute written then leaves no handler state, and other consumers (base URL, forms, custom
> elements) miss the same mutations. Reachability: unverified (a scout is checking whether `SessionCore::flush` outside a
> bracket, `pipeline.rs:138-165`, can carry such a write under the VM). Register only if the scout finds a reachable
> path.

### §8.3 Records for existing slots (for the PM)

- `#11-template-contents-owner-document`: add "`EH-attrs-at-creation` slice b" as a dependant (x5): slice b cannot
  process template contents at creation until the inert owner is carried per node (§9.2 (c)).
- `#11-event-handler-target-determination`: add "`EH-attrs-at-creation` slice b" as a dependant of facet (1) (x3).

### §8.4 Hand-off: what N1's rev 4 must change (O8; N1 is not edited here)

Coordinates are N1 memo `0f99538d` lines.

1. **Rename the prerequisite.** Every `EH-insert-once` becomes "`EH-attrs-at-creation` slice a", and the path
   `docs/plans/2026-10-event-handler-insert-once.md` becomes `docs/plans/2026-10-event-handler-attrs-at-creation.md`:
   `:21`, `:29`, `:88`, `:188`, `:202`, `:282`, `:322`, `:358`, `:407`, `:447`, `:476`, `:559`, `:575`, `:587`, `:623`,
   `:633`, `:634`, `:671`, `:728`, `:733` (`:540` is revision history; N1's idiom decides).
2. **Restate P-N5 / H-P16 against K-1** (§0.5): "creation-time handler content attributes are processed at most once;
   no insertion, move or rebind writes handler state from them", not "delivered at first insertion".
3. **R2 G2 (R-a):** cite slice a's K-1 / K-2 with cell C4 for a single-attribute element; the multi-attribute case is
   slice b's B1. N1 decides whether the single-attribute form suffices for its cells (O5).
4. **R2 G3 (R-b):** cite K-3 with cell C5 (i)–(iii) for a single-attribute element; multi-attribute is B1.
5. **H-P13 stays** (P9); add U8 (i) as the evidence that slice a's bind pass does not reach a DOMParser document.
6. **NR7 (`:575`)**: "on a base without slice a".
7. **The `:623` row** ("the Insert arm turns handlers raw on every move"): fixed by slice a (K-1).
8. **§9 prerequisites (`:671`)**: the re-measure obligation on the post-slice-a base (N1 plan-review R2 G4) names
   slice a's landing commit.
9. **Check, not edit:** whether any N1 cell builds initial HTML through a shell pipeline. From slice a the bind pass
   delivers its inline handlers (G1), which may change such a cell's HEAD prediction.

## §9. Landing-order comparison and the R8-vs-rev-1 decision record (kept from rev 2)

### §9.1 The question

Rev 1 built an interim (`Pending { superseded }` → `Delivered` at first insertion) and named R8 (the change steps at
write time, creation included) as the end state. CLAUDE.md "One issue, one way" forbids landing an interim that a later
PR deletes. So: is there a landing order under which R8 lands with no interim?

### §9.2 Each blocker, its removal, and the evidence

R8's blockers are exposures that R8 creates on populations whose spec outcome HEAD cannot yet produce.

| # | population | exposure if R8 lands on `fd6955e8` | evidence | removed by |
|---|---|---|---|---|
| (a) | DOMParser nodes with handler attributes | parse-time raw entries; HEAD's compile gate reads the HostData flag only, so `d.onclick` compiles (spec: null, *get the current value* step 3.2) | N1 memo §4.2 / D-N2 (the node clause is added to the compile gate by N1); `scripting.rs:83-113` (the clause exists, but at HEAD the compile gate does not consult it) | **N1 landing first.** H-P13 does not need rewriting: it describes N1's base, which then does not contain R8's DOMParser processing. N1's own DOMParser cells write handlers after the parse (N1 memo H-P13), so none depends on R8 |
| (b) | a DOMParser (or fragment-throwaway) `<body>` with a WindowEventHandlers attribute | parse-time delivery to the **page** Window: `resolve_handler_target` returns `dom.window_entity()` for any `<body>` (`event_handler_consumer.rs:328-334`) | `webref body html determining-the-target-of-an-event-handler`: step 3 "If eventTarget's node document is not an active document, then return null." HEAD has no step 3 (slot facet 1) | **T3 landing first.** T3 can reuse the effective-node-document rule in `scripting.rs:100-111` against `EcsDom::document_root` (`dom/mod.rs:425`). With T3, R8 is spec-exact here: a DOMParser body's WEH attribute writes nothing, and moving that body into the page does not deliver it |
| (c) | template contents and their clones | parse-time raw entries on template-content elements; the composition does not suppress them (H15), so after N1 they compile, and `tpl.content.querySelector('b').dispatchEvent(new Event('click'))` runs the inline handler (spec: the template contents owner document has a null browsing context, `webref body html appropriate-template-contents-owner-document` step 1.1.1, so null). A clone of template content keeps the page document as owner (`clone_subtree` threads `owner_document(src)`, `tree_clone.rs:208`) | H15 | **C1 landing first.** A narrower clause-(b) facet ("composed root is a template-contents fragment") does not suffice: a clone of template content is not under such a fragment, so the inert owner must be carried per node, which is C1's part (a). C1's part (b) (adopt into the inert owner) is needed for `tpl.innerHTML = …` results |
| (d) | the pre-bind parse (G1) | none; R8 needs a bind-time pass because no consumer exists at parse (H9) | H9, H13 | nothing: the bind pass is K-4's own arm, not an interim. H13 makes it per-element idempotent, which D-1's marker gives |

**G1's effect on the comparison.** G1 adds the bind pass (d). Under R8 it is the creation-equivalent point for pre-bind
elements: no script runs between the parse and the bind, so processing at bind is observably processing at creation.
Under rev 1's D-3 it would be a **second** delivery timing beside first insertion, which is the strangler mix "One issue,
one way" forbids. So G1 makes the case for R8 stronger, and it forces D-1's per-element once-key whatever the mechanism.

### §9.3 Verdict (rev 2) and the decision (rev 3)

**R8 lands with no interim mechanism: yes.** Rev 1's `Pending` is withdrawn (R-D3). Rev 2 compared two landings:

1. **No split**: order T3 → N1 → C1 → the whole work. Costs: it reverses PM decision 17 (i) (N1 lands first and cannot
   carry C8b(iv) or C8c's second half, N1 memo NR7); C1 is a deliberately deferred, edge-dense slot with no driver;
   G1 is production-visible and would wait behind all three.
2. **Two slices** (rev 2's recommendation): slice a now, with no prerequisite, carrying G1 and N1's dependency; slice b
   after T3, N1, C1. No interim mechanism; one guard line changes (D-8).

**PM decision 21 adopts landing 2** (§1). The order is slice a → N1 → … ; slice b after T3 and C1, starting once C1's
handling is decided.

## §10. Definition of done

### §10.1 Slice a

- `HandlerAttrsProcessed`, `process_handler_content_attrs`, `process_handler_content_attrs_in_subtree`,
  `note_handler_write`, and `EventHandlerAttributeConsumer::initialize_from_tree` exist in `event_handler_consumer.rs`
  and are re-exported from `lib.rs`. `handle_insert` contains no direct `set_inline_handler` call
  (`/usr/bin/grep -n 'set_inline_handler(' crates/script/elidex-script-session/src/event_handler_consumer.rs` → the
  definition, `handle_attribute_change`, and `process_handler_content_attrs` only).
- `/usr/bin/grep -n 'traverse_descendants' crates/script/elidex-script-session/src/event_handler_consumer.rs` → no hit.
- `note_handler_write` has D-8's shape; its guard line carries a comment naming `#11-eh-attrs-creation-time-call-sites`.
- `initialize_consumers` calls `event_handler_attrs.initialize_from_tree`; its doc no longer says the consumer needs no
  init.
- Both VM element setters call `note_handler_write` with `VmInner::spec_level_policy` before `activate_event_handler`;
  no other VM code touches the marker outside tests.
- Clone-policy row added; module and field docs updated.
- Cells C1–C13, U1–U8 and pins E1–E7 (E5b if Q5 finds a harness) present; every red cell and every pin was run red on
  the pre-fix base, with its control green, and the failures recorded in the PR description; all pass at the head.
- `cargo test -p elidex-script-session --all-features`, `cargo test -p elidex-js --all-features`,
  `cargo test -p elidex-shell --all-features`, and `mise run ci` are green.
- The PM has `#11-eh-attrs-creation-time-call-sites` (§8.2) to register at landing.

### §10.2 Slice b

- Its own per-PR plan-memo has passed `/elidex-plan-review`; its base contains T3 and N1, and C1's handling is decided
  (and, if that handling is "land C1", C1 has landed).
- The D-8 guard line is deleted, and nothing else in `note_handler_write` changes (`git diff` of that function = one
  removed line plus its comment).
- Every bound creation path of §5.3's slice-b column calls `process_handler_content_attrs_in_subtree` at creation (O9's
  chosen shape); the Insert arm's role is O6's decision.
- `inert_document.rs:78-84` corrected (P3).
- Cells B1–B8 and the inverted U5 (ii) present, red-first on slice b's base; slice a's cells and pins stay green.
- `#11-eh-attrs-creation-time-call-sites` is closed by the PM.

### §10.3 Program

- Both slices have landed in the §1.3 order, each with its own reviewed plan-memo.
- K-1 to K-6 hold at slice b's landing without qualifiers.
- **Checker** (run; do not trust a stored verdict): `python3 .claude/skills/elidex-plan-review/preflight.py
  docs/plans/2026-10-event-handler-attrs-at-creation.md` exits 0.

## §11. Open for plan-review

Decided (no longer open):
- **O1, split:** decided by PM decision 21 (D-7, §1).
- **Order:** decided by PM decision 21 (§1.2).
- **O8, the N1 edits:** routed to N1's rev 4; the list is §8.4.

Open:
- **O2: One issue, one way.** Slice a's processing point for post-bind creations is first insertion, and
  `note_handler_write` carries the D-8 guard that slice b deletes. No component, function, or call site is deleted later
  (subject to O6, P12). Is this an acceptable increment, or an interim the rule forbids? The record is the slot text in
  §8.2.
- **O3: walk cost** (K4). Slice a walks the bound document on every bind (H13). Measure it (a large parsed document,
  repeated `dispatch_event`) before slice a's implementation is approved; if too costly, R10 (first bind only), which
  then needs G7 settled. The dense-marker alternative R9 is rejected for archetype churn; confirm.
- **O4: writer completeness.** Is §5.1 + §5.0 complete for handler state? In particular any VM path that writes
  `EventListeners` handler state outside `activate_event_handler`.
- **O5: N1's R-a / R-b.** In slice a they hold only for elements with one deliverable handler attribute (C4, C5). Is that
  sufficient for N1's landing, given that N1's own cells do not use multi-attribute elements (N1 memo §6)?
- **O6 (slice b): the Insert arm after slice b.** Keep it as a release fail-safe with a `debug_assert!` that fires when it
  finds an unmarked element with a deliverable handler attribute (B7), or delete it. Deleting it makes it a second item
  slice b deletes (P12, §1.4); keeping it preserves decision 21's "only the guard is deleted".
- **O7: G7.** Pending the scout's reachability result; the PM decides whether to register §8.2's text.
- **O9 (slice b): the creation-time call-site shape** — a dedicated `MutationEvent` variant fired by each silent build
  through the dispatcher it restores (the dispatcher holds the policy; the fragment build must fire it before
  `finish_detached_fragment`'s adopt, so step 3 sees the throwaway document), or direct calls from the engine-independent
  wrappers (`elidex-dom-api` clone, `elidex-form` DOMParser, the session's fragment apply), as `BaseUrlMaintainer` does
  for DOMParser (`inert_document.rs:196`). A clone-time precedent exists: `elidex_form::apply_clone_form_state` runs
  synchronously at clone time from the VM `cloneNode` marshalling shim, with no `MutationEvent` (clone-policy table,
  `tree_clone.rs:16`).
- **O10: approval vehicle.** Whether this umbrella lands as a docs-only approval PR (the IBP-sandbox idiom) or with
  slice a's PR. The PM decides.

Questions for a scout (facts this rev could not establish by its own reads):

- **Q1** Under the VM, can `SessionCore::flush` outside a bracket (`pipeline.rs:138-165`) carry a `SetAttribute` or a
  fragment insertion of an `on*`-bearing element? Does any shell path create such elements in the main `EcsDom` while
  unbound? (G7; the scout is running.)
- **Q2** At every bind site, is the bound `document` equal to `dom.document_root()` (main pipeline and the three iframe
  builders)? The bind pass and T3 both assume it.
- **Q3** Does elidex fetch or fire `load` / `error` for an `<img>` in template contents or in a clone of them? (The size
  of §9.2 (c)'s exposure.)
- **Q4** Can `elidex-form` and `elidex-dom-api` call an `elidex-script-session` function (O9's second shape)?
- **Q5** Is there a hermetic harness for the same-origin URL iframe path (`load.rs:221`)? (E5b.)
- **Q6** Does any `MutationEvent` consumer match exhaustively, so that a new variant (O9's first shape) would need edits
  there?
