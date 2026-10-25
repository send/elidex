# EH-insert-once — event-handler content attributes delivered once per element (per-PR plan-memo)

Standalone prerequisite PR, label **`EH-insert-once`**. Owner: Layout lane. It sits **outside** the IBP-sandbox program
and is not part of that umbrella's DAG (PM decision). IBP-sandbox N1 (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`,
branch `layout-ibp-sandbox-handler-gates`) depends on it: N1's C8b(iv) needs a compiled handler to survive a move into a
DOMParser document, and HEAD re-raws it on every move (§0.2 H1).

**Rev 1** (2026-10-25). First draft, before `/elidex-plan-review`. It folds in two requirements from N1's plan-review R2 (R-a, R-b) as contract clauses K-2 and K-3 (§0.5), the write-time-raw alternative (§4.3 R8), and the silent-writer census (§5.0).

## §0. Frame, premises, decisions

### §0.1 Coordinate frame

Every `file:line` is a **`fd6955e8`** coordinate (origin/main at the branch cut; branch `layout-eh-insert-once`).
Every code reference comes from `/usr/bin/grep -n` over that tree. Every § number ↔ title pair and every algorithm
step comes from `.claude/tools/webref` (`heading --exact`, `dfn`, `body`) run on 2026-10-25.

### §0.2 Premises re-measured

| id | claim | command | result |
|---|---|---|---|
| H1 | The Insert arm re-delivers baked `on*` attributes on **every** `MutationEvent::Insert` | `sed -n 376,384p` and `sed -n 424,463p crates/script/elidex-script-session/src/event_handler_consumer.rs` | **true.** `handle` (`:376`) routes `MutationEvent::Insert { node, .. }` to `handle_insert` (`:381`). `handle_insert` (`:427-463`) walks `node` plus `traverse_descendants`, reads every `Attributes` entry whose name is in `EVENT_HANDLER_ATTRS`, and calls `set_inline_handler` (`:462`) → `EventListeners`' `set_uncompiled` (`:474`). There is no state check |
| H2 | `MutationEvent::Insert` fires for every insertion, including a move | `sed -n 267,298p crates/core/elidex-ecs/src/dom/tree/mutation.rs`; `/usr/bin/grep -n 'fire_after_insert(' crates/core/elidex-ecs/src/dom/tree/mutation.rs`; `sed -n 199,221p crates/core/elidex-ecs/src/dom/mod.rs` | **true.** Three fire sites: `append_child` `:47`, `insert_before` `:123`, `replace_child` `:201`. Each runs after the implicit detach of an already-inserted child, so a move fires Insert. `fire_after_insert` (`:282-298`) filters only shadow roots (`:288`). `dispatch_event` (`mod.rs:199`) has no connectedness filter |
| H3 | `set_uncompiled` makes a compiled handler raw again | `sed -n 203,243p crates/script/elidex-script-session/src/event_listener.rs`; `sed -n 521,581p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `set_uncompiled` (`event_listener.rs:207`) stores `uncompiled = Some(..)` and resets `cleared`. On the next read or dispatch, `ensure_event_handler_current` (`event_handler_attrs.rs:521`) runs `take_uncompiled` (`:567`), compiles, and overwrites the callable (`set_handler_callable`). That yields a new function identity, and an IDL-set callable is replaced |
| H4 | The IDL setter supersedes inline source | `sed -n 664,692p crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | **true.** `activate_event_handler` (`:664`) calls `clear_uncompiled` (`:677` set, `:687` null). A later Insert undoes it (H1) |
| H5 | The DOMParser build suppresses every consumer | `sed -n 110,113p` and `sed -n 196,200p crates/dom/elidex-form/src/inert_document.rs` | **true.** `take_mutation_dispatcher` `:112`, restored `:198-199`. Untouched DOMParser nodes carry **no** `EventListeners` entry. N1 relies on this (N1 memo H-P13) |
| H6 | Fragment parses (innerHTML / outerHTML / setHTMLUnsafe) are also built dispatch-suppressed and fire Insert at caller placement | `sed -n 318,336p crates/core/elidex-ecs/src/dom/tree/teardown.rs`; `/usr/bin/grep -n 'begin_detached_fragment' crates/dom/elidex-html-parser/src/lib.rs crates/dom/elidex-html-parser-strict/src/tree_builder/mod.rs` | **true.** `begin_detached_fragment` (`teardown.rs:318`) takes the dispatcher (`:325`). Callers: `elidex-html-parser/src/lib.rs:142` and `elidex-html-parser-strict/src/tree_builder/mod.rs:178`. The test `set_inner_html_fires_insert_at_caller_placement` (`crates/script/elidex-script-session/src/mutation/html_fragment.rs:565`) pins the re-fire at placement |
| H7 | Every script attribute write reaches the AttributeChange arm | `/usr/bin/grep -rn --include='*.rs' -E 'set_attribute_without_dispatch\|get::<&mut Attributes>' crates` (non-test hits); `sed -n 897,909p crates/script/elidex-script-session/src/mutation/mod.rs`; `sed -n 292,330p crates/core/elidex-ecs/src/dom/attribute.rs` | **true.** The session flush `apply_set_attribute` (`mutation/mod.rs:897`) calls `EcsDom::set_attribute`, which dispatches. `remove_attribute` (`attribute.rs:292`) dispatches only when an attribute was present (`:318`). The only non-dispatching writer, `set_attribute_without_dispatch` (`attribute.rs:173`), is documented as consumer-internal; its one caller writes `value`. No production writer adds an `on*` attribute without AttributeChange |
| H8 | Clone does not copy `EventListeners` or any non-listed component | `sed -n 13,36p` and `sed -n 260,330p crates/core/elidex-ecs/src/dom/tree_clone.rs` | **true.** The per-component clone-policy table (`:19-34`) is a deliberate enumeration. `EventListeners` is in the "deliberate non-copy" row (`:30`). `clone_node_shallow_unchecked` copies an explicit set only |
| H9 | Production parses the main document **before** the VM binds, and bind seeds only base URL | `sed -n 839,870p crates/shell/elidex-shell/src/pipeline.rs`; `sed -n 296,318p crates/script/elidex-js/src/vm/vm_api.rs`; `sed -n 116,137p crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | **true.** `build_pipeline_from_loaded` receives a parsed `dom` (`:850-858`). `initialize_consumers` (`consumer_dispatcher.rs:135-137`) calls only `base_url.initialize_from_tree`. So nothing delivers a main-document `<button onclick>` until that node is inserted after bind. This is out of scope here; see §7 G1 |
| H10 | Light-tree walks skip shadow roots | `sed -n 294,302p crates/core/elidex-ecs/src/dom/tree/walkers.rs`; `sed -n 320,330p crates/core/elidex-ecs/src/dom/tree/navigation.rs` | **true.** `traverse_descendants` uses `children_iter_rev`, which skips `ShadowRoot` entities. Together with H2's shadow-root filter, an element inserted directly under a shadow root is never delivered (§7 G2) |
| H11 | `remove_listener_and_prune_back_ref` keeps the `EventListeners` entry | `sed -n 49,61p crates/script/elidex-js/src/vm/inner.rs` | **true.** It removes only the stored callable and the abort back-ref. The entry, and its registration position, survive a compile failure and an IDL null |
| H12 | VM bind/unbind keeps the listener store | `sed -n 125,143p` and `sed -n 210,225p crates/script/elidex-js/src/vm/vm_api.rs` | **true.** `HostData` (which owns `listener_store`) persists across the per-call bind brackets, and the Window entity is reused. No path relies on re-delivery to restore callables after a rebind |

### §0.3 Premise errors in the brief (reported)

- **P1: `importNode` / `adoptNode` do not exist.** `/usr/bin/grep -rn -i --include='*.rs' -E 'importnode|adoptnode' crates`
  returns nothing. The memo analyses the spec algorithms (§3 rows 9–10) and elidex's internal adopt
  (`EcsDom::adopt_subtree`, `crates/core/elidex-ecs/src/dom/tree/teardown.rs:285`, used by fragment teardown). It plans
  no cell on either method.
- **P2: the Window-reflecting body set is not delegated.** The brief treats `<body>` delegation as covering the handlers
  that act on Window. HEAD's `resolve_handler_target` (`event_handler_consumer.rs:328`) delegates only
  `HandlerScope::Window` rows, which are the WindowEventHandlers. WHATWG HTML §8.1.8.2 also routes the *Window-reflecting
  body element event handler set* to the Window: `onblur`, `onerror`, `onfocus`, `onload`, `onresize`, `onscroll`
  (`webref body html event-handlers-on-elements,-document-objects,-and-window-objects`). HEAD's table marks those
  `Global` (`EVENT_HANDLER_ATTRS` `:61`; e.g. `onblur` `:68`, `onscroll` `:126`), so `<body onload>` lands on the body element. This is a pre-existing gap (§7 G4). This PR's
  delegated cell uses `onhashchange`, a WindowEventHandlers row.
- **P3: a docstring overstates a scripting rule.** `inert_document.rs:78-84` says "HTML §8.1.3.4 does NOT process
  event-handler attrs when scripting is disabled". The attribute change steps of §8.1.8.1 have no scripting condition.
  It is *get the current value of the event handler* step 3.2 that returns null for a raw value when the document's
  browsing context is null. The observable outcome matches (null), and the docstring is corrected in this PR (§4.2 row
  6) because it describes this consumer's delivery timing.
- **P4: `clone_attributes` is not the cloneNode writer** (from N1 plan-review R2's requirement R-b). `EcsDom::clone_attributes`
  (`crates/core/elidex-ecs/src/dom/tree_clone.rs:53-75`) has one caller, the test
  `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`. cloneNode's silent `Attributes` write is `builder.add(attrs)` in
  `clone_node_shallow_unchecked` (`tree_clone.rs:368`). The conclusion is unchanged: clone is a silent writer (§5.0).

### §0.4 Decisions (this PR)

- **D-1: once-key.** The once-key is a per-**origin**-element component, `HandlerAttrDelivery` (NEW), with variants
  `HandlerAttrDelivery::Pending` (NEW) and `HandlerAttrDelivery::Delivered` (NEW). It is never
  keyed on the handler's target (the Window, for `<body>` delegation). Absence means "not yet delivered, nothing
  superseded".
- **D-2: delivery.** Delivery of an element's handler content attributes runs at most once, in one engine-independent
  function, `deliver_handler_content_attrs` (NEW). The Insert arm calls it for every element of the inserted subtree.
  After delivery the element is `Delivered` (terminal) when it had any handler content attribute. Otherwise its
  component is removed.
- **D-3: order.** A deferred delivery never overwrites a handler that was written after the element's creation. Every
  writer to an undelivered origin records the written event type in `Pending { superseded }`, and delivery skips those
  names. The writers are the AttributeChange arm and both IDL setters, including null. Recording is a pure component
  write. It materialises nothing, so untouched siblings stay entry-less (H5 is preserved).
- **D-4: timing unchanged.** Delivery still happens at first insertion (HEAD's timing), not at creation. Creation-time
  delivery is the spec's timing, and §4.3 R1 records why it is rejected here. Reads never deliver.
- **D-5: VM surface.** The VM's two element-receiver setters (`native_event_handler_set`, `native_body_weh_set`) call
  `record_handler_write` (NEW) before `activate_event_handler`. Marshalling only; the rule lives in
  `elidex-script-session` (Layering mandate).

### §0.5 Contract (single citable statement; N1 and later memos quote this section)

For an element E, call "first insertion" the first `MutationEvent::Insert` whose inserted subtree contains E. "Handler
state" means the `EventListeners` entry for an event type on E's resolved target, plus that entry's stored callable.

- **K-1, no re-delivery.** After E's first insertion, no later insertion or move of E writes any handler state derived
  from E's content attributes. A compiled callable keeps its identity across moves, in the live document and into a
  DOMParser document alike.
- **K-2, an AttributeChange write wins over deferred delivery** (N1 R2 requirement R-a). Suppose a handler for event
  type T was written through the AttributeChange arm (`setAttribute` / `removeAttribute` of `on<T>`) while E was not yet
  inserted. Then E's first insertion does not write T's handler state. The value set by that write survives, including
  a callable compiled from it, whether E is first inserted into the live document or a DOMParser document.
- **K-3, an IDL write wins over deferred delivery** (N1 R2 requirement R-b). If `E.on<T>` (or, for a `<body>` E, a
  delegated WindowEventHandlers accessor) was assigned a callable or null while E was not yet inserted, E's first
  insertion does not write T's handler state. This holds whatever wrote E's `Attributes` silently, i.e. without
  AttributeChange: the parser, the fragment parser, the DOMParser build, or cloneNode.
- **K-4, unsuperseded attributes are delivered at first insertion.** Every handler content attribute present on E at
  its first insertion whose event type is not covered by K-2 / K-3 is delivered then, as an internal raw uncompiled
  handler on its resolved target, subject to the session's `SpecLevelPolicy`.
- **K-5, DOMParser inertness.** Until its first insertion, an untouched DOMParser-built element has no `EventListeners`
  entry. A write per K-2 / K-3 creates only the entry for the written event type.

K-2 and K-3 are realised by D-3 (supersession recorded by the writer, consumed by delivery), K-1 by D-1 / D-2 (the
origin's terminal `Delivered`), and K-4 / K-5 by D-2 / D-4. Red cells: K-1 C1–C3; K-2 C6, C15, U4; K-3 C4, C5, C7, C8,
C16; K-4 C9, C10, C14; K-5 C14.

## §1. Shape and verdict

The PR has one engine-independent mechanism: a component plus two functions in `event_handler_consumer.rs`. It has one
VM marshalling touch (two calls) and doc rows in the clone-policy table and `inert_document.rs`. There are no new
crates, traits, side-stores, or `MutationEvent` variants, and dispatcher ordering does not change.

The work is edge-dense: the invariant axes are once, order, origin-key, freshness, first-insert completeness, policy,
DOMParser inertness, and layering (§2). So this per-PR memo goes through `/elidex-plan-review` before any code. It is a
terminal slice: one subsystem, the handler content-attribute delivery, and it is not split further.

## §2. Coupled invariants

- **I1 once.** For each origin element, the handler content attributes present at creation are delivered at most once.
  `Delivered` is terminal and is removed only by despawn.
- **I2 order.** Delivery never writes an event type that a post-creation writer has already written for this origin.
- **I3 origin key.** Delivery state lives on the origin element, never on the target. Body delegation writes the Window's
  `EventListeners`, and the state stays on the `<body>`.
- **I4 fresh.** Every newly created element starts with no delivery state: parser, fragment, DOMParser, `createElement`,
  or clone (H8, policy row in §4.2).
- **I5 completeness.** An undelivered element with handler content attributes is delivered at its first insertion,
  which is HEAD's first-insert behaviour. A DOMParser node first inserted into the live document is delivered then.
- **I6 policy.** A policy-excluded handler attribute (`onstorage` under `BrowserCore` / `App`,
  `event_handler_consumer.rs:306`, `:445-449`) is never wired by any arm. Exclusion does not reopen I1.
- **I7 inertness.** No `EventListeners` entry is created for an untouched DOMParser node (H5; N1 H-P13). Recording
  supersession (D-3) creates none.
- **I8 layering / re-entry.** The algorithm lives in `elidex-script-session`. VM host code only marshals. Inside
  `dispatch`, the consumer writes components only (no tree mutation; `mutation_event.rs` re-entry contract).

| pair | intersect? | resolution |
|---|---|---|
| I1×I2 | yes | `Delivered` ends recording. `record_handler_write` is a no-op on a `Delivered` origin, because after delivery the entries hold the value and writers act on them directly (HEAD behaviour). Cells C1, C4 |
| I1×I3 | yes | Once is per origin. A second `<body>` (a clone) has its own state, so it delivers once to the Window even though the Window already holds an entry. Cells C8, U2 |
| I1×I4 | yes | Clone does not copy `HandlerAttrDelivery`, so a clone of a delivered element delivers once. Cell C11, U6 |
| I1×I5 | yes | Completeness is guaranteed only for the first insertion; later insertions are no-ops. Cells C1, C10 |
| I1×I6 | yes | `Delivered` is set when the element has any handler content attribute, excluded or not, so an excluded-only element is not rescanned on every move. Cell U3 |
| I1×I7 | yes | A DOMParser node gains state only when written to (Pending) or inserted (Delivered). Cell C14 |
| I1×I8 | yes | The once check is inside `deliver_handler_content_attrs`. No VM-side once logic exists |
| I2×I3 | yes | Supersession is recorded on the origin (the `<body>` receiver), keyed by event type, though the write lands on the Window. Cell C8 |
| I2×I4 | yes | A clone starts with an empty superseded set. Writes to its source never carry over. Cell C11 |
| I2×I5 | yes | First-insert delivery skips superseded names and delivers the rest. Cells C4, C7, U4 |
| I2×I6 | no | An excluded name is never written by the AttributeChange arm (it returns at `:405-407`), so nothing is recorded for it. The IDL setter for an excluded name is not installed (the A2 seam) |
| I2×I7 | yes | A write on a DOMParser node records Pending and does not deliver siblings. Cell C14 |
| I2×I8 | yes | Both IDL setters record through the engine-independent `record_handler_write` (D-5). The AttributeChange arm records in-crate |
| I3×I4 | yes | A `<body>` clone's own state starts empty. The Window's entries are untouched by the clone |
| I3×I5 | yes | A first insertion delivers WindowEventHandlers rows to the Window target via `resolve_handler_target`, which is unchanged. Cell U2 |
| I3×I6 | yes | `onstorage` is a Window row and policy-gated before target resolution (`:445`) |
| I3×I7 | yes | A DOMParser `<body>` delivers to the page Window at first insertion. This is HEAD behaviour and a pre-existing gap (§7 G3). This PR adds no new trigger for it, because D-3 delivers nothing on a write |
| I3×I8 | no | Target resolution stays in `elidex-script-session` |
| I4×I5 | yes | Clones and fragment-parsed nodes are delivered at their first insertion. Cells C9, C11 |
| I4×I6 | no | Policy does not depend on provenance |
| I4×I7 | yes | DOMParser nodes are created entry-less (H5) and stateless (I4) |
| I4×I8 | no | Clone policy is documented in `elidex-ecs`. The component is defined in `elidex-script-session`, as `EventListeners` already is |
| I5×I6 | yes | Delivery applies the same `policy.installs(event_handler_attr_spec_level(..))` filter HEAD uses (`:447`) |
| I5×I7 | yes | First insertion of a DOMParser node delivers it, while untouched nodes stay entry-less until then. Cells C10, C14 |
| I5×I8 | no | — |
| I6×I7 | no | — |
| I6×I8 | no | The policy is the consumer's construction-time `SpecLevelPolicy` (`:359-371`). `record_handler_write` needs none |
| I7×I8 | yes | `inert_document.rs` keeps the dispatcher suppression. Only its docstring changes (§0.3 P3) |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §8.1.8.1 Event handlers | event handler content attribute *attribute change steps* 1–5.6 | step 1 non-handler name → return; step 3 null target → return; step 4 value null → deactivate; step 5.5 set raw uncompiled handler; step 5.6 activate. "run even if oldValue and value are identical" | `handle_attribute_change` `crates/script/elidex-script-session/src/event_handler_consumer.rs:389` (unchanged write path; records supersession, D-3) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *determining the target of an event handler* 1–4 | non-body → self; not WindowEventHandlers and not the Window-reflecting set → self; step 3 inactive node document → null; else relevant global | `resolve_handler_target` `event_handler_consumer.rs:328` (unchanged; step 3 and the reflecting set are §7 G3 / G4) | ✗ | no |
| WHATWG HTML §8.1.8.1 Event handlers | event handler IDL attribute setter 1–4; *deactivate an event handler* 1–6 | target null → return; value null → deactivate; else set value + activate | `native_event_handler_set` `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs:344`, `native_body_weh_set` `:459` → `record_handler_write` (NEW) | ✓ | yes |
| WHATWG HTML §8.1.8.1 Event handlers | *get the current value of the event handler* step 3 | raw → compile (or null when step 3.2 holds); callback → returned as is; a read writes no content-attribute state | `ensure_event_handler_current` `event_handler_attrs.rs:521` (unchanged; reads never deliver, D-4) | ✓ | no |
| WHATWG HTML §8.1.8.2 Event handlers on elements, Document objects, and Window objects | WindowEventHandlers rows; *Window-reflecting body element event handler set* | WEH rows delegated (HEAD); reflecting set not delegated (§0.3 P2, §7 G4) | `EVENT_HANDLER_ATTRS` `event_handler_consumer.rs:61`; cell C8 uses `onhashchange` | ✗ | no |
| WHATWG DOM §4.9 Interface Element | *create an element* (no attributes); *append an attribute* step 4; *handle attribute changes* step 3 | every attribute present at creation is appended after creation and runs the attribute change steps once | `deliver_handler_content_attrs` (NEW): the deferred realisation of these per-attribute runs (D-2, D-4) | ✓ | yes |
| WHATWG HTML §13.2.6.1 Creating and inserting nodes | *create an element for a token* step 17.2 "Append attribute to element" | parser path: every token attribute is appended at creation | parser backends bake `Attributes` at spawn (`crates/dom/elidex-html-parser/src/convert.rs:257`); delivered at first insertion | ✓ | yes |
| WHATWG HTML §13.4 Parsing HTML fragments | step 4 new `Document`; parser associated with it | fragment elements are created in a throwaway document | `begin_detached_fragment` `crates/core/elidex-ecs/src/dom/tree/teardown.rs:318` (unchanged) | ✗ | yes |
| WHATWG DOM §4.4 Interface Node | *clone a single node* step 2.5 (append each copied attribute) | a clone is a new element; its attributes run the change steps at creation | clone-policy table `crates/core/elidex-ecs/src/dom/tree_clone.rs:19-34` (row added: non-copy) | ✓ | yes |
| WHATWG DOM §4.5 Interface Document | *adopt* steps 1–3 | node document changes; no event-handler step | `EcsDom::adopt_subtree` `teardown.rs:285` (unchanged; state survives adopt) | ✓ | no |
| WHATWG DOM §4.2.3 Mutation algorithms | *insert* step 7.1 (adopt), *insertion steps* | HTML defines no event-handler insertion steps (§8.1.8.1 checked for insert / adopt / clone / move: none) | `handle_insert` `event_handler_consumer.rs:427` → once delivery | ✓ | yes |
| WHATWG HTML §8.5.1 The DOMParser interface | `parseFromString` steps 3–4 ("Since document does not have a browsing context, scripting is disabled") | `text/html` branch only | `parse_into_inert_document` `crates/dom/elidex-form/src/inert_document.rs:110` (docstring only, §0.3 P3) | ✗ | yes |
| WHATWG HTML §8.5.4 The innerHTML property | setter: parse HTML for insertion, then *replace all* | also `outerHTML` (§8.5.5, *replace* with fragment) and `setHTMLUnsafe` (§8.5.2): same fresh-element path | `apply_set_inner_html` / `apply_set_outer_html` via `crates/script/elidex-js/src/vm/host/dom_inner_html.rs:146`, `:361` (unchanged) | ✗ | yes |

### §3.1 User-input touch audit

Rows marked *yes* carry author-controlled markup or script values: attribute values, handler sources, and which
elements are moved where. The mechanism stores event-type names (from the fixed `EVENT_HANDLER_ATTRS` table) and never
user strings, so the only user-controlled datum it stores is which element was written. Delivery keeps HEAD's policy
gate (I6). No row adds a parse of user input.

## §4. Mechanism

### §4.1 Home / kind

```rust
// crates/script/elidex-script-session/src/event_handler_consumer.rs (NEW items)
/// Delivery state of an element's event-handler content attributes (WHATWG
/// HTML §8.1.8.1 attribute change steps, run per attribute at creation by DOM
/// §4.9 "append an attribute"). Absent = not yet delivered, nothing superseded.
pub enum HandlerAttrDelivery {
    /// Not yet delivered; these event types were written after creation and
    /// must not be overwritten when the deferred delivery runs (I2).
    Pending { superseded: Vec<String> },
    /// Delivered (terminal, I1).
    Delivered,
}
pub fn deliver_handler_content_attrs(policy: SpecLevelPolicy, origin: Entity, dom: &mut EcsDom);
pub fn record_handler_write(origin: Entity, event_type: &str, dom: &mut EcsDom);
```

The component lives on the origin element (I3) and is defined beside the consumer that owns its semantics. It is
`Send + Sync` and not a per-VM identity handle, so per CLAUDE.md's side-store→component rule it is a component, not a
`HostData` or session map. Despawn removes it. The one-enum shape makes Pending and Delivered mutually exclusive by
construction.

**`deliver_handler_content_attrs(policy, origin, dom)`**: return if `origin` is not an element or is `Delivered`. Collect
`origin`'s attributes whose names are in `EVENT_HANDLER_ATTRS`. If there are none, remove any `Pending` component and
return. For each, in attribute order: skip it if it is policy-excluded (I6) or its event type is in `superseded` (I2).
Otherwise resolve the target with `resolve_handler_target` and call `set_inline_handler`. Then insert `Delivered`. The
two-phase collect-then-write shape is kept from `handle_insert` (borrow discipline).

**`record_handler_write(origin, event_type, dom)`**: return if `origin` is not an element or is `Delivered`, or if
`origin`'s `Attributes` lack `on{event_type}` (nothing to supersede, so the component stays sparse). Otherwise add
`event_type` to `Pending { superseded }`, inserting the component if absent.

### §4.2 Rewrite mapping (HEAD → this PR)

| # | site | HEAD | this PR |
|---|---|---|---|
| 1 | `handle_insert` `event_handler_consumer.rs:427-463` | re-delivers the whole subtree on every Insert | collects the subtree as before, then calls `deliver_handler_content_attrs` per element |
| 2 | `handle_attribute_change` `:389-421`, `Some(src)` arm `:411` | `set_inline_handler(target, ..)` | `record_handler_write(node, event_type, dom)` and then the same `set_inline_handler`. The removal arm (`:418`) is unchanged: the attribute is gone, so delivery cannot reach it |
| 3 | `native_event_handler_set` `event_handler_attrs.rs:344-356` | `activate_event_handler(..)` | `record_handler_write(entity, &event_type, dom)` and then the same call. It is a no-op for Document / Window / Worker receivers (non-elements). Callable and null are both recorded |
| 4 | `native_body_weh_set` `:459-479` | delegates to the Window | records on the **`<body>` receiver** (I3), then delegates as before |
| 5 | `crates/core/elidex-ecs/src/dom/tree_clone.rs:30` clone-policy "deliberate non-copy" row | — | adds `HandlerAttrDelivery` (elidex-script-session): per DOM §4.4 *clone a single node* step 2.5 a clone's attributes run the change steps at its own creation, so the clone's delivery state starts fresh |
| 6 | `crates/dom/elidex-form/src/inert_document.rs:78-84` | "§8.1.3.4 does NOT process event-handler attrs" | states the spec split: the change steps set a raw value, and *get the current value* step 3.2 nulls it in a browsing-context-less document. elidex realises this by not delivering until first insertion (§0.3 P3). Suppression is unchanged |
| 7 | module docs `event_handler_consumer.rs:1-33`; `consumer_dispatcher.rs:58-66` field doc | "Arm 2 walks the inserted subtree's baked `on*` attributes" | Arm 2 delivers **once per element**; writers record supersession |
| 8 | `crates/script/elidex-script-session/src/lib.rs:36-41` | re-exports | adds `HandlerAttrDelivery`, `record_handler_write` (VM uses the latter; tests read the former) |

### §4.3 Alternatives rejected

- **R1: creation-time routing.** This would fire `AttributeChange`, or a new `ElementCreated` event, at spawn, or run the
  change steps at parser append. It is the spec's timing, and it is rejected for this PR for four reasons.
  - (a) Every bulk creation path runs dispatch-suppressed (H5, H6, H9), so events at creation are dropped.
  - (b) An AttributeChange at creation fans out to every consumer: custom-element `attributeChangedCallback`, canvas,
    base URL, form, and MutationObserver records. Each would change meaning.
  - (c) A raw value on a DOMParser node at creation would be compiled by HEAD's getter, whose compile gate reads only the
    HostData flag (N1 memo §4.2). That regresses every DOMParser getter until N1's node clause lands, and N1 depends on
    this PR.
  - (d) The spec's target at creation in a fragment's throwaway document is null for body WEH (step 3), which elidex
    does not model (§7 G3).

  R1 remains the end state: once N1's node clause and §7 G3 exist, delivery can move to creation (or first read), and
  `Pending` disappears. Handed to §8.
- **R2: key on the target's entry** ("skip a name whose target already has an entry"; no component). This fails I3: the
  Window's entries are shared by every `<body>` and by `window.on*`, so a body clone would never deliver. It also fails
  I2 for the IDL null setter, which leaves no entry (`activate_event_handler` `:683-692` creates none).
- **R3: connectedness filter** (deliver only when `!was_connected` and now connected). Remove-then-append re-delivers
  (N1's C8b(ii) shape). A DOMParser document is connected, so the key is arbitrary. Connectedness is not the spec's
  condition.
- **R4: a positive pending marker attached at spawn.** This fails unsafe: a spawn path that forgets the marker would leave
  dead handlers. The spawn sites (`EcsDom::create_element_with_owner`, `clone_node_shallow_unchecked`, parser
  `attach_derived`) are in crates below `elidex-script-session` and would need the handler table. D-1's
  absence-means-pending fails safe: an unknown creation path gets HEAD's first-insert delivery.
- **R5: compare the source text** (skip when the attribute value equals the last delivered source). No spec basis, and it
  encodes "once" as a value coincidence. It would also need per-handler string storage.
- **R6: a session or `HostData` `HashSet<Entity>`.** This violates the side-store→component rule (CLAUDE.md). The value
  is `Send + Sync` and per-entity.
- **R7: deliver-before-write** (writers deliver all siblings, then write). This is spec-order-faithful without per-name
  state, and it was rejected after analysis. A handler write on a DOMParser node would materialise its untouched
  siblings as raw entries, which HEAD's getter compiles (spec: null), breaking I7. On a DOMParser `<body>`, a sanitizer's
  first `removeAttribute('onload')` would deliver the body's remaining WEH attributes (`onpageshow`, …) to the
  **page** Window through the step-3 gap (§7 G3), clobbering page handlers mid-sanitization. D-3 records instead and
  materialises nothing.
- **R8: write-time raw recording for silent writers** (proposed after N1's plan-review R2). Every writer of
  `Attributes` would record the raw value at write time, on the attribute change steps' own path, including the parser,
  fragment parser, DOMParser build, and cloneNode. Insertion would then never deliver anything. This is R1 restated per
  writer, and it is the spec's timing. Each writer was weighed against D-3 for K-2 / K-3:
  - **AttributeChange writers.** Already write-time at HEAD (`handle_attribute_change`). D-3 keeps this, so R8 and D-3
    agree here. K-2 holds under both.
  - **DOMParser build** (`inert_document.rs:112`, `:198-199`). Recording at parse means a raw entry for every handler
    attribute of every parsed node. That breaks K-5 and N1's H-P13, and HEAD's getter would compile them (spec: null,
    *get the current value* step 3.2) until N1's node clause lands, a regression on every DOMParser read. Worse, a
    parsed `<body onpageshow=X>` would deliver X to the **page** Window at parse time, through the step-3 gap (§7 G3),
    with no script action needed. A sanitizer that only parses and inspects would clobber page handlers. So R8 needs N1
    and G3 to be safe, and N1 depends on this PR.
  - **Fragment parser** (innerHTML / outerHTML / setHTMLUnsafe). The nodes are unobservable between creation and
    placement (H6), so R8 and D-3 produce identical results. R8 gains nothing here, and it costs a dispatcher-independent
    write path inside `begin_detached_fragment`'s suppressed window.
  - **cloneNode** (VM shim `node_methods_extras.rs:201` → `clone_node_shallow_unchecked`; note: `EcsDom::clone_attributes`
    `tree_clone.rs:53` has one caller, the test `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`, and is not on the
    cloneNode path). Recording at clone time would be spec-exact for clones whose node document is the live document. For
    a clone of a DOMParser node, it hits the same getter regression as above. A clone-only R8 leaves two delivery timings
    (clone at creation, everything else at insertion), which is the strangler state CLAUDE.md "one issue, one way"
    forbids.
  - **Pre-bind main-document parse** (H9). No consumer exists at parse, so R8 still needs a bind-time pass (§7 G1).

  Verdict: on K-2 / K-3, R8 and D-3 are observably equal. Both requirement cases are met by D-3's recorded supersession
  (cells C6, C15 for K-2; C4, C7, C16 for K-3). R8 differs only where its write-time recording introduces exposures
  that need N1's node clause and G3 first. So R8 is the end state, together with R1 (§8), and is not this PR's
  mechanism.

### §4.4 ECS-native and Layering checks

- Idiom mapping: the OO "per-element 'attributes processed' flag" maps to an ECS component on the entity. The
  "supersede on write" observer maps to writers recording into that component. A single function then realises the
  deferred change steps. No registry and no new dispatcher variant.
- `vm/host/` adds two calls into `elidex_script_session` (marshalling). There is no `EcsDom::*` use beyond the existing
  `host.dom()` borrow and no algorithm in host code.
- Inside `dispatch` the consumer still writes only `EventListeners` and the new component (I8).

### §4.5 Docstring citation table (Axis 4)

| doc site | citation |
|---|---|
| `HandlerAttrDelivery`, `deliver_handler_content_attrs` | WHATWG HTML §8.1.8.1 Event handlers (attribute change steps); WHATWG DOM §4.9 Interface Element (*append an attribute* step 4) |
| `record_handler_write` | WHATWG HTML §8.1.8.1 (IDL setter steps 3–4; attribute change steps 4–5) |
| clone-policy row | WHATWG DOM §4.4 Interface Node (*clone a single node* step 2.5) |
| `inert_document.rs` correction | WHATWG HTML §8.1.8.1 (*get the current value* step 3.2); WHATWG HTML §8.5.1 The DOMParser interface |

## §5. Populations and named risks

### §5.0 `Attributes` writers: silent vs firing (census)

Definition, by property: a **silent writer** is any production code path that writes an element's `Attributes`
component without firing `MutationEvent::AttributeChange` for that write. Population: every write to `Attributes`
across `crates/`. Classification script:
`/private/tmp/claude-501/-Users-kazuaki-orca-workspaces-elidex-wt-cv-plan/2baf5b5d-5487-4b33-932a-765bd64385e9/scratchpad/census/census.py`.
It runs `/usr/bin/grep -rnE --include=*.rs <pattern> crates` per pattern, then classifies each hit as test when the path
is a test file or dir (`/tests/`, `tests_*.rs`, `*_tests.rs`, `/benches/`, `test_helpers`) or the line follows the file's
first `#[cfg(test)]`. Verified 2026-10-25 at `fd6955e8`.

| pattern | total | test | prod | production sites → class |
|---|---|---|---|---|
| `get::<&mut (elidex_ecs::)?Attributes>` | 21 | 17 | 4 | `attribute.rs:215` (`write_attribute_no_dispatch`, see row 6), `:305` (`remove_attribute`: **fires** when present, `:318`), `tree_clone.rs:68` (`clone_attributes`: **silent**), `crates/shell/elidex-shell/src/content/iframe/lifecycle.rs:358` (iframe `src` rewrite: **silent**, writes only `src`) |
| `insert_one(…, attrs\|Attributes\|attributes` | 2 | 0 | 2 | `attribute.rs:224` (row 6), `tree_clone.rs:72` (`clone_attributes`: **silent**) |
| `builder.add(attrs)` | 1 | 0 | 1 | `tree_clone.rs:368` (`clone_node_shallow_unchecked`, the cloneNode path: **silent**, all attributes) |
| element spawn carrying `attrs` | — | — | 1 | `mod.rs:349-355` `create_element_with_owner` spawn: **silent** creation bake. Its `create_element*` callers are below |
| `.create_element(_with_owner\|_ns)?(` | 2210 | 2187 | 23 | 17 pass `Attributes::default()` (nothing baked: `teardown.rs:327`, `inert_document.rs:120`, `document.rs:235`, `entity_spawn.rs:57`, `select.rs:329`, `document_props.rs:337`, `anonymous_table.rs:159`, `grid.rs:422`, `:467`, `table_mutation/mod.rs:116`, `:127`, `:138`, `:148`, `:355`, `:359`, `:448`, `:512`); 2 are the shims `mod.rs:335`, `:380`; **4 bake parser attributes (silent)**: tolerant `crates/dom/elidex-html-parser/src/convert.rs:258`, strict `crates/dom/elidex-html-parser-strict/src/tree_builder/insert.rs:61`, `:119`, declarative-shadow template `crates/dom/elidex-html-parser-strict/src/tree_builder/shadow.rs:162` |
| `.set_attribute_without_dispatch(` | 1 | 0 | 1 | `crates/dom/elidex-form/src/value_mode.rs:64`: **silent**, writes only `value` |
| `.clone_attributes(` | 1 | 1 | 0 | no production caller (test `crates/core/elidex-ecs/src/dom/tests/clone.rs:14`) |
| `.set_attribute(` | 141 | 134 | 7 | all **fire** via the `attribute.rs:125` chokepoint: `mutation/mod.rs:903` (every script `setAttribute`), `select/mod.rs:156`, `:182`, `attrs.rs:208`, `:256`, `char_data/attr.rs:172`, `:333` |
| `.remove_attribute(` | 24 | 18 | 6 | 5 code sites (verified 2026-10-25) **fire** when present (`mutation/mod.rs:925`, `select/mod.rs:159`, `:178`, `dialog.rs:80`, `char_data/attr.rs:243`); 1 is a comment (`attr_proto.rs:44`) |

Parser attribute merging (an in-body `<html>` / `<body>` start tag adding missing attributes) has no elidex-side writer:
`/usr/bin/grep -rn --include='*.rs' -i -E "already present|merge_attr|add_missing|set_attribute|write_attribute"
crates/dom/elidex-html-parser-strict/src crates/dom/elidex-html-parser/src` (non-test) returns nothing. The tolerant
backend's html5ever merge happens in its RcDom before `convert.rs:258` bakes, and the strict backend has no merge arm.
The DOMParser build and every fragment parse write through the four parser sites above; the dispatcher suppression
(H5, H6) adds no writer of its own.

Result, which carries K-3's scope:
- (a) Silent writers that can write an **`on*`** attribute are exactly the creation-time bakes: the four parser sites
  and `clone_node_shallow_unchecked`, plus the production-unreachable `clone_attributes`.
- (b) The remaining silent writers write fixed non-handler names (`value`, `src`).
- (c) Every post-creation `on*` write fires AttributeChange (the complement: 7 + 5 firing sites, verified 2026-10-25).

So at E's first insertion, E's handler content attributes are its creation-time set, plus the names written by the
AttributeChange arm, which D-3 records. That is the precondition D-3 needs. K1 (§5.5) covers a future silent `on*`
writer, and a guard is proposed in O5.

### §5.1 Pending-source state: every writer and reader

Commands: `/usr/bin/grep -rn --include='*.rs' -E
"set_uncompiled|clear_uncompiled|take_uncompiled|uncompiled_source|mark_cleared|is_handler_cleared" crates` and
`/usr/bin/grep -n -E "add_event_handler|find_event_handler" crates/script/elidex-script-session/src/event_handler_consumer.rs
crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` (verified 2026-10-25).

| op | production sites | role under this PR |
|---|---|---|
| `set_uncompiled` | `event_handler_consumer.rs:474` (in `set_inline_handler`, called from `:411` AttributeChange and `:462` Insert) | `:411` unchanged (a real write; records first). `:462` reached only through once delivery |
| `clear_uncompiled` | `event_handler_attrs.rs:677`, `:687` (`activate_event_handler`) | unchanged; preceded by `record_handler_write` (§4.2 rows 3–4) |
| `take_uncompiled` | `event_handler_attrs.rs:567` (`ensure_event_handler_current`) | unchanged (read-side compile) |
| `mark_cleared` | `event_handler_consumer.rs:487` (`clear_inline_handler`, from `:418`) | unchanged |
| `is_handler_cleared` | `event_handler_attrs.rs:568` | unchanged |
| `uncompiled_source` | none in production (`event_listener.rs:282` definition; tests `:470-496`) | the store-level oracle for cells C2, U1, U2, U4 |
| `add_event_handler` | `event_handler_consumer.rs:473`, `event_handler_attrs.rs:674` | unchanged |
| test writers | `crates/script/elidex-js/src/tests_call_listener.rs:132`, `:200` (`set_uncompiled` on a fixture) | unaffected (no insertion) |

Callers of the reconcile reader `ensure_event_handler_current` (`/usr/bin/grep -rn --include='*.rs'
'ensure_event_handler_current(' crates/script/elidex-js/src`): `dispatch_target.rs:183`, `natives_promise.rs:548`,
`engine.rs:233`. All read; none deliver.

### §5.2 `MutationEvent::Insert` consumers: does any depend on re-run?

Command: `/usr/bin/grep -rn --include='*.rs' -E "MutationEvent::Insert" crates` (non-test arms).

| consumer | arm | depends on the handler consumer re-running? |
|---|---|---|
| `LiveRangeBridge` (`crates/dom/elidex-dom-api/src/range/live.rs:121`) | boundary adjust | no; it reads no `EventListeners` |
| `BaseUrlMaintainer` (`crates/dom/elidex-dom-api/src/element/document_base.rs:293`) | `<base>` on insert | no; it re-runs its own state by design (tree-position dependent) |
| `FormControlReconciler` (`crates/dom/elidex-form/src/reconciler.rs:44`) | FCS attach | no |
| `CustomElementReactionConsumer` (`crates/dom/elidex-custom-elements/src/consumer.rs:77`) | connected reactions | no |
| `EventHandlerAttributeConsumer` (`event_handler_consumer.rs:381`) | this PR | — |
| test probe (`mutation/html_fragment.rs:577`) | counts Inserts | no |

`ConsumerDispatcher`'s field doc already records that no other consumer reads `EventListeners`
(`consumer_dispatcher.rs:58-66`). Existing tests with handler attributes and a move: of the files `/usr/bin/grep -rln
--include='*.rs' -E "setAttribute\(.on[a-z]+|onclick=" crates/script/elidex-js/src`, which are `tests_event_handler_attrs.rs`
and `tests_call_listener.rs`, none moves an element carrying a handler content attribute. The DOMParser cells
(`tests_event_handler_attrs.rs:567`, `:597`, `:670`, `:691`, `:719`) parse markup without handler attributes (N1 memo
H-P13). So no existing cell depends on re-delivery.

### §5.3 Element-creation paths (where an element's attributes are baked without the change steps)

| path | site | first delivery under this PR |
|---|---|---|
| main-document parse (pre-bind) | `pipeline.rs:850-858` | first insertion after bind (HEAD parity; §7 G1) |
| tolerant fragment / document convert | `convert.rs:257` `create_element_ns` | caller placement Insert (H6) |
| strict tree builder | `tree_builder/mod.rs:178` | caller placement Insert |
| DOMParser | `inert_document.rs:110-200` (via `apply_set_inner_html`) | first insertion after return (I5); suppression is unchanged |
| `createElement` + `setAttribute` | `EcsDom::create_element_with_owner` `mod.rs:344`, then AttributeChange | the write records Pending; first insertion delivers only unsuperseded names |
| `cloneNode` | `tree_clone.rs` `clone_node_shallow_unchecked`; VM shim `crates/script/elidex-js/src/vm/host/node_methods_extras.rs:201` | first insertion (fresh state, row 5) |
| template contents | detached content fragment (HTML §4.12.3) | when moved out (`appendChild(tpl.content)` fires Insert per child) or cloned |

### §5.4 Interaction matrix (brief items)

| interaction | outcome |
|---|---|
| `innerHTML` / `outerHTML` / `setHTMLUnsafe` / fragment parsing | fresh elements, delivered once at placement. Re-setting `innerHTML` creates new entities; there is no reuse |
| `cloneNode` | a clone is a new element with fresh state (I4). The source's state and entries never carry over (C11). A pre-insertion write on the clone is honoured (C4, C5) |
| `importNode` / `adoptNode` | not exposed (§0.3 P1). Spec adopt has no handler step, and the component survives `adopt_subtree` (component-preserving, `teardown.rs:285-291`) |
| shadow roots | `fire_after_insert` skips Insert under a shadow root and walks skip shadow content (H10). Such elements are never delivered: HEAD parity, pre-existing gap §7 G2. Writes on them work as at HEAD |
| `<body>` WindowEventHandlers delegation | key = the `<body>` origin (I3). The target stays the Window via `resolve_handler_target` `:328`. The IDL body setter records on the receiver (§4.2 row 4). Cells C8, U2 |
| policy-excluded attributes (`onstorage`) | filtered at delivery exactly as at `:445-449`, and they count toward `Delivered` (I1×I6). Cell U3 |
| attribute removed while outside the tree | `remove_attribute` dispatches (H7), and HEAD clears an existing entry. Delivery later reads `Attributes`, so the removed name is never delivered (C12) |
| attribute set while outside the tree, then compiled, then inserted | recorded as superseded, so first insertion keeps the compiled callable (C6) |

### §5.5 Named risks

- **K1: an unknown writer.** A writer that does not call `record_handler_write` falls back to HEAD behaviour: first
  insertion may overwrite its write. That is fail-safe toward HEAD, not toward loss. The writer population is §5.1 plus
  H7. A future writer is the VM's `document.open` *erase all event listeners and handlers*, which is not implemented.
- **K2: residual R-x.** Consider a cross-origin write to a delegated target between a `<body>` clone's creation and its
  first insertion, e.g. `c = document.body.cloneNode(); window.onhashchange = fn; append(c)`. The spec delivers
  `onhashchange` at clone creation and then `fn` wins; this PR delivers at insertion and the raw value wins. The defect
  is a deferral artefact, pre-existing at HEAD, and closes with R1's end state. No cell (the PR does not fix it).
- **K3: detached reads.** A never-inserted clone of a live element reads null (spec: compiled function) because reads do
  not deliver (D-4). Pre-existing; closes with R1's end state. §7 G5.
- **K4: walk cost.** HEAD walks the whole inserted subtree on every Insert, and this PR keeps the walk. Each element costs
  one `get::<&HandlerAttrDelivery>` plus HEAD's attribute scan. The component is inserted only on elements with handler
  attributes, so no archetype churn hits ordinary elements.

## §6. Test and coverage plan

*Red* = must fail at `fd6955e8` and pass after; the implementer runs each red cell against HEAD first and records the
failure (red-first). The reasoning in the "HEAD" column comes from H1 / H3 and is a prediction, not a measurement.
*Guard* = passes at both and pins a preserved behaviour.

VM integration, new file `crates/script/elidex-js/src/vm/tests/tests_event_handler_delivery.rs` (NEW), registered in
`crates/script/elidex-js/src/vm/tests/mod.rs`. It reuses the `with_el` / `entity_of` harness shape of
`tests_event_handler_attrs.rs:38-76` and connects `el` to the document first, so "live" means connected.

| cell | kind | shape | HEAD | oracle |
|---|---|---|---|---|
| C1 | red | live `<div onclick=A>` (via `innerHTML` into connected `el`); `t.onclick = fn`; move `t` (append to `el` again) | move re-raws A; `t.onclick` compiles A | `t.onclick === fn`; dispatch runs `fn`, not A |
| C2 | red | live `t.setAttribute('onclick', S)`; dispatch once (compiles; ran = 1); `f = t.onclick`; `doc2 = DOMParser…`; `doc2.body.appendChild(t)` (N1 C8b(iv) shape) | move re-raws S | store: `EventListeners` entry for `click` on `t` has `uncompiled_source == None`; `t.onclick === f`. No dispatch assertion in `doc2`: HEAD's invoke gate suppresses it, and N1 relaxes that |
| C3 | red | live baked A; `f = t.onclick`; move twice (live→live) | each move recompiles | `t.onclick === f` after each move |
| C4 | red | `c = t.cloneNode()` (`t` has onclick=A); `c.onclick = fn`; insert `c` | first insert re-raws A | `c.onclick === fn`; dispatch runs `fn` |
| C5 | red | clone as C4; `c.onclick = null`; insert | first insert raws A | `c.onclick === null`; dispatch runs nothing |
| C6 | red | `c = document.createElement('div')`; `c.setAttribute('onclick', S)`; `f = c.onclick`; insert | first insert re-raws S | `c.onclick === f` |
| C7 | red | DOMParser `<div id=d onclick=A>`; `d.onclick = fn`; append `d` into the live `el` | first insert raws A | `d.onclick === fn`; dispatch runs `fn` |
| C8 | red | `b = document.createElement('body')`; `b.setAttribute('onhashchange', X)`; `window.onhashchange = fn`; append `b` to `el` | first insert re-raws X onto the Window | `window.onhashchange === fn`; store: `HandlerAttrDelivery` is on `b`'s entity (Delivered), and the Window entity has none |
| C9 | guard | parser-created markup first inserted: `el.innerHTML = '<button onclick=…>'` on the strict path, plus a markup string that forces the tolerant fallback (e.g. foster-parented content in `<table>`) | delivered | dispatch runs the inline body on both paths |
| C10 | guard | DOMParser `<div onclick=A>` appended into the live `el` | delivered at insert | `typeof d.onclick === 'function'`; dispatch runs A |
| C11 | guard | `t` delivered and IDL-set to `fn`; `c = t.cloneNode()`; insert `c` | — | before insert: `c` carries no `HandlerAttrDelivery`. After: `c.onclick` is compiled A (not `fn`, not `t.onclick`) |
| C12 | guard | clone of onclick=A; `c.removeAttribute('onclick')`; insert | — | `c.onclick === null` |
| C13 | guard | delivered `t`, `f = t.onclick`; `t.setAttribute('onclick', B)` | — | `t.onclick !== f`; dispatch runs B (change steps run on every set, §8.1.8.1) |
| C14 | guard | DOMParser `<div id=d onclick=A onmouseover=B>`; `d.onclick = fn` | — | store: `d` has no `mouseover` entry and `HandlerAttrDelivery::Pending{["click"]}` (I7). After appending `d` live: the `mouseover` entry is raw B, and `d.onclick === fn` |
| C15 | red (K-2, N1 R-a) | `c = document.createElement('div')` (never inserted); `c.setAttribute('onclick', A)`; `c.dispatchEvent(new Event('click'))` (compiles; ran = 1); `f = c.onclick`; `doc2 = DOMParser…`; `doc2.body.append(c)`. The first insertion goes **into a DOMParser document** (C6 is the live-document twin) | first insert re-raws A | store: `click` entry on `c` has `uncompiled_source == None`; `c.onclick === f` |
| C16 | red (K-3, N1 R-b) | `dp1` parses `<div id=d onclick="A">`; `d.onclick = fn` (`clear_uncompiled`); `dp2.body.append(d)`. Variant (ii): `document`-connected `el.append(d)` (C7 shape). Variant (iii): `c = d.cloneNode()` (clone-silent `Attributes`), `c.onclick = fn`, `dp2.body.append(c)` | first insert raws A over `fn` | store: `click` entry `uncompiled_source == None`; `d.onclick === fn` (and `c.onclick === fn`) in every variant |

Engine-independent unit cells, new file `crates/script/elidex-script-session/src/tests_event_handler_delivery.rs` (NEW),
`#[cfg(test)]` in `lib.rs`. They drive `EcsDom` with `EventHandlerAttributeConsumer` installed as the dispatcher, using
the shape of `event_handler_consumer.rs:552-589`, but through real `append_child` / `set_attribute` calls so `Attributes`
and the event agree (fixture realism: no synthetic events against an unwritten `Attributes`).

| cell | kind | shape | oracle |
|---|---|---|---|
| U1 | red | element spawned with `onclick=A`, appended (entry raw A); `take_uncompiled`; appended again elsewhere | `uncompiled_source == None` after the second append (HEAD: `Some("A")`) |
| U2 | red | `<body onhashchange=X>` with a Window entity; append; `take_uncompiled` on the Window entry; move body | Window entry `uncompiled_source == None`; `HandlerAttrDelivery::Delivered` on body, none on the Window |
| U3 | guard | `BrowserCore` policy, `<body onstorage=…>` appended twice | no `storage` entry; body is `Delivered` after the first append |
| U4 | red | detached element `onclick=A onmouseover=B`; `set_attribute(onclick, C)`; `take_uncompiled(click)`; append | `click` `uncompiled_source == None` (HEAD: `Some("C")`); `mouseover` raw B |
| U5 | guard | element with no handler attributes appended | carries no `HandlerAttrDelivery` |
| U6 | guard | `clone_node_shallow` of a `Delivered` element | the clone carries no `HandlerAttrDelivery` |

Existing coverage kept green: `tests_event_handler_attrs.rs` (all 35 tests: `/usr/bin/grep -c '#\[test\]'
crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` = 35 at `fd6955e8`) and the consumer's `spec_level_tests` (`event_handler_consumer.rs:501`).

WPT: no subset is claimed. The surface is engine-unit and VM-integration only (supported-surface testing).

### §6.1 Touch-time 1000-line check (`wc -l` at `fd6955e8`)

| file | lines | after (est.) | action |
|---|---|---|---|
| `crates/script/elidex-script-session/src/event_handler_consumer.rs` | 590 | ≈ 680 | none; tests go to the new file |
| `crates/script/elidex-script-session/src/lib.rs` | 62 | ≈ 66 | none |
| `crates/script/elidex-js/src/vm/host/event_handler_attrs.rs` | 726 | ≈ 735 | none |
| `crates/script/elidex-js/src/vm/tests/mod.rs` | 197 | 198 | none |
| `crates/script/elidex-js/src/vm/tests/tests_event_handler_attrs.rs` | 735 | 735 (untouched) | none; new cells go to a sibling file. This avoids growing it toward the seam and avoids colliding with N1's rewrites in it |
| `crates/core/elidex-ecs/src/dom/tree_clone.rs` | 425 | ≈ 426 | none |
| `crates/dom/elidex-form/src/inert_document.rs` | 287 | ≈ 290 | none |
| `crates/script/elidex-js/src/vm/consumer_dispatcher.rs` | 153 | ≈ 154 | none |

No touched file exceeds 1000 lines, so no prerequisite split is needed.

## §7. Out of scope: gaps found (reported to PM, not slotted)

- **G1: pre-bind main-document handlers are never delivered** (H9). A `<button onclick>` in the initial HTML has no
  `EventListeners` entry until a script moves it. `initialize_consumers` would be the seam: deliver every element of the
  bound document once. This PR's `deliver_handler_content_attrs` is exactly that per-element function, so the fix becomes
  a one-call delegate. That makes it urgent once known; PM to slot or fold.
- **G2: shadow-tree elements are never delivered** (H10). This covers `sr.innerHTML = '<button onclick>'` and
  declarative shadow content.
- **G3: determining the target, step 3, is not modelled.** A `<body>` whose node document is not the active document
  (DOMParser, fragment throwaway) still resolves to the page Window, both at delivery and in `native_body_weh_get` /
  `_set` (`event_handler_attrs.rs:437`, `:459`). HEAD parity; this PR adds no new trigger (I3×I7).
- **G4: the Window-reflecting body set is not delegated** (§0.3 P2).
- **G5: reads do not deliver** (K3), so a detached live-document clone reads null.
- **G6: a stale docstring.** `crates/core/elidex-ecs/src/dom/attribute_reconcile.rs:21-23` says the deferred session flush writes `Attributes` without
  the chokepoints. `apply_set_attribute` now routes through `EcsDom::set_attribute` (H7).

## §8. Edges and hand-offs

- **→ N1** (`docs/plans/2026-10-ibp-sandbox-handler-gates.md`, read-only here):
  - C8b(iv) becomes satisfiable: after this PR a move preserves the compiled callable, which N1's arm 2 then runs.
  - N1's H-P13 stays true for untouched DOMParser nodes. A *write* on a DOMParser node now also inserts a
    `HandlerAttrDelivery::Pending` component, and still creates no sibling entries (I7). N1 may cite C14.
  - N1 rewrites `ensure_event_handler_current` (`:521-581`) and the invoke gates. This PR touches only the two native
    setters (`:344`, `:459`) in that file, so the merge surface is disjoint.
- **→ R1 end state** (creation-time or first-read delivery): unblocked by N1's node clause plus G3. When it lands,
  `Pending` and the Insert-arm delivery are deleted together (one issue, one way), and K2 / K3 / G5 close.

## §9. Definition of done

- `HandlerAttrDelivery`, `deliver_handler_content_attrs`, and `record_handler_write` exist in
  `event_handler_consumer.rs`, re-exported from `lib.rs`. `handle_insert` contains no direct `set_inline_handler` call
  (`/usr/bin/grep -n 'set_inline_handler' crates/script/elidex-script-session/src/event_handler_consumer.rs` shows only
  the definition, `handle_attribute_change`, and `deliver_handler_content_attrs`).
- Both VM element setters call `record_handler_write` before `activate_event_handler`. No other VM code reads or writes
  `HandlerAttrDelivery` outside tests.
- Clone-policy row added (§4.2 row 5); `inert_document.rs` docstring corrected (row 6); module and field docs updated
  (row 7).
- Cells C1–C16 and U1–U6 are present. The red cells were run red at `fd6955e8` and recorded in the PR description, and
  all pass at the PR head.
- `cargo test -p elidex-script-session --all-features`, `cargo test -p elidex-js --all-features`, and `mise run ci` are
  green.

## §10. Open items for plan-review

- **O1: I2's writer set.** Is §5.1 + H7 complete? In particular: any VM path that writes `EventListeners` handler state
  outside `activate_event_handler`. The worker and VmObject homes are non-element and out of scope by construction.
- **O2: `Pending` granularity.** It is keyed by event type. Two names with the same event type cannot occur in
  `EVENT_HANDLER_ATTRS` (event type = name minus `on`, `event_handler_consumer.rs:208-215`), but confirm.
- **O3: K2 and K3 stay unfixed here** (deferral artefacts; R1 end state). PM to confirm they need no slot beyond §8's
  hand-off.
- **O4: G1's priority.** It is a production-visible gap (initial-HTML inline handlers are inert until moved), adjacent
  to this PR's mechanism. PM to decide: fold into this PR as one `initialize_consumers` delegate plus a pipeline-level
  cell, or keep it separate.
- **O5: a silent-`on*`-writer guard.** §5.0's result (a) is a measured property of `fd6955e8`, not a structural one. A
  future silent writer of an `on*` attribute on an existing element would bypass D-3's recording and fall back to HEAD's
  overwrite (K1). Plan-review should decide whether this PR adds a guard. One option is a `debug_assert` in
  `write_attribute_no_dispatch` that the name is not in `EVENT_HANDLER_ATTRS`, which would need the table to be
  visible from `elidex-ecs`, a layering question. The other is a trip-wire over §5.0's patterns.
