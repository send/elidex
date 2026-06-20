# ScriptSession Mutation-Path Audit (Program B / B0)

Audit date: 2026-06-20 JST
Status: **DOC ONLY — no `.rs` change.** This is the B0 deliverable of the
philosophy-alignment umbrella (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md`,
Program B). It confirms the F3 mechanism end-to-end against current `main`
(HEAD `76c1677f`) and recommends the canonical path for B1/B2.
Audience: Claude / maintainers (and Codex via the review guidelines below).

> **§4 direction correction (this revision).** An earlier revision recommended
> Option 1 — making `MutationObserver` a consumer of the `EcsDom`
> `ConsumerDispatcher`. That recommendation was **anchored to the current code's
> shape** ("the code already fans out through `ConsumerDispatcher`, so ride it")
> rather than to the root mandate, and it is **wrong**. `CLAUDE.md` L16 makes the
> direction unique: *"ScriptSession as the sole Script↔ECS boundary … 書き込みは
> session mutation と flush point に集約し、SameObject・**MutationObserver**・
> atomic script-task visibility を同一機構で守る."* MutationObserver visibility is
> a **ScriptSession-seam responsibility** (mutation buffer + flush), not an
> `EcsDom`-layer one. Option 1 inverts this — it puts a *script-observable*
> responsibility into the engine-internal `EcsDom` layer and **bypasses** the
> ScriptSession boundary. §4 below is re-derived from the mandate: **the
> ScriptSession seam owns MutationObserver record production** (a corrected
> Option 2), while the `ConsumerDispatcher` is kept, honestly scoped, as the
> engine-internal *derived-state reconciliation* mechanism it actually is
> (§2.1 — it is **not** script-observable). See §4 for the flip and for how each
> of the six Codex-R2 findings (8YcV / 8YcT / 8YcR / 8YcO / 8YcW / 8YcL) is
> subsumed by this single structural correction.

> **Runtime caveat (read first).** The audit below traces the **elidex-js VM**
> (`crates/script/elidex-js`). But the **production shell still runs boa**:
> `crates/shell/elidex-shell/src/pipeline.rs:9`/`:68` constructs
> `elidex_js_boa::JsRuntime`, and `lib.rs:39` re-exports it; S5/boa removal
> (D-26 PR7) is **not yet done**. The `ConsumerDispatcher` is installed **only**
> by `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:279`) — there is **no**
> `set_mutation_dispatcher` install in `elidex-js-boa` or `elidex-shell`. So
> Mechanism A (§2.1) is **VM-only**; today's production (boa) reaches
> MutationObserver exclusively through the session buffer + `deliver_mutation_records`
> path (§2.2). The corrected §4 (ScriptSession-seam-owned MO) is **runtime-uniform
> in direction** — both runtimes route script-visible mutation through the seam —
> but the *flush→MO-delivery hook* must be wired in both; see §0, §2, §4.

> **Why B0 before B1/B2.** The original audit (F3 in
> `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`) framed the
> problem as "DOM write paths bypass the `ScriptSession` mutation buffer / its
> observers". The umbrella §2.3 already overturned the *attribute-write-bypass*
> half of that framing; this doc **re-verifies the reframe by direct code read**
> (not by trusting the umbrella prose) and then sharpens it further — the real gap
> is wider than §2.3 stated. But note the §2.3 reframe did **not** license putting
> MO production in the `EcsDom` layer: the mandate keeps MO on the ScriptSession
> seam (see the §4 direction-correction callout above). Every claim below carries
> a `file:line` anchor re-checked at HEAD `76c1677f`.
> Re-grep at PR-open; this doc is itself a snapshot.

---

## 0. TL;DR

- **The audit's central inference is wrong.** Direct `EcsDom::set_attribute` /
  `remove_attribute` calls do **not** bypass observers, reconcilers, style
  derivation, or live collections. `EcsDom::set_attribute`
  (`crates/core/elidex-ecs/src/dom/attribute.rs:101`) *is* the canonical
  attribute-write chokepoint: it runs `reconcile_attribute_derived_components`
  + `rev_version` and then `dispatch_event(MutationEvent::AttributeChange)` to
  the installed `MutationDispatcher`. Tree mutations
  (`append_child`/`remove_child`/`insert_before`,
  `crates/core/elidex-ecs/src/dom/tree/mutation.rs`) fire
  `MutationEvent::Insert`/`Remove` to the same dispatcher.
  - **Caveat:** the dispatcher *is* `ConsumerDispatcher` **only in the
    elidex-js VM** (installed by `Vm::bind`, `vm_api.rs:279`). In today's
    production shell (boa), **no dispatcher is installed**, so direct
    `EcsDom::set_attribute`/tree calls in the boa runtime fire `dispatch_event`
    into a no-op sink — the chokepoint still runs the *inline*
    `reconcile_attribute_derived_components` + `rev_version` (baked into the
    primitive, not consumers), but the consumer fan-out (live ranges,
    CE reactions, etc.) does not run there. So "the chokepoint notifies the
    `ConsumerDispatcher`" is a **VM-only / post-S5** statement.
  - **Two `dispatch_event`-side exceptions** that *suppress even the inline
    reconcile path or do not fire the event at all* (so they bypass any
    consumer regardless of runtime), called out below for §4: (a)
    `set_attribute_without_dispatch` (`attribute.rs:146`) writes `Attributes`
    and reconciles but fires **no** `MutationEvent` — used by form
    value-mode/type-change (`elidex-form/value_mode.rs:222`); (b) the
    session-buffer `apply_set_attribute`/`apply_remove_attribute`
    (`mutation/mod.rs:288-332`) and boa iframe writes record a `Mutation`
    and **self-generate** the `MutationRecord` without ever entering
    `EcsDom::set_attribute`, so they never fire a `MutationEvent` either.
- **The real gap is the JS-level `MutationObserver`, and it is broader than the
  umbrella §2.3 stated.** `MutationObserver` is *not* a `ConsumerDispatcher`
  consumer. It is fed exclusively by `Vm::deliver_mutation_records`, which in
  the elidex-js VM is reached from **only three production sites**: the
  innerHTML setter native (`dom_inner_html.rs:148`), the **outerHTML** setter
  native (`dom_inner_html.rs:362` — `native_element_set_outer_html`, **not**
  insertAdjacentHTML), and the shell's per-frame flush (`content/mod.rs:258`
  ← `re_render` ← `SessionCore::flush`). The session buffer
  (`SessionCore::pending`) is populated in production by **only the
  `SetInnerHtml` / `InsertAdjacentHtml` `Mutation` variants**
  (`elidex-dom-api/element/tree.rs:416`/`:476`). Every *other* JS DOM write —
  `setAttribute`, `removeAttribute`, every reflected IDL setter, **and
  `appendChild`/`removeChild`/`insertBefore`/`replaceChild` even through the
  bridge** — produces **no `MutationRecord`** and is therefore unobservable by
  `new MutationObserver(...)`. Note the shell's *initial-script /
  finalization* flush (`pipeline.rs:25-34` `flush_with_ce_reactions`) feeds
  flush records to **CE reactions only** and does **not** call
  `deliver_mutation_records`, so even innerHTML mutations done during page
  load are not delivered to MO via that path — only the per-frame
  `content/mod.rs:258` site delivers (§2.2).
- **There are two mechanisms, and they answer two *different* questions.** They
  are: (1) `EcsDom`'s `ConsumerDispatcher` — **engine-internal derived-state
  reconciliation**, synchronous at the chokepoint, driving the 7 consumers
  (live-range / node-iterator / base-url / form-control / event-handler-attr /
  canvas / custom-element), **none of which is script-observable**; and (2)
  `SessionCore`'s mutation buffer + `flush` → `deliver_mutation_records` — the
  **script-observable** path that feeds `MutationObserver`, but which is fed today
  only by innerHTML-class ops. They overlap only at innerHTML. The right reading
  is **not** "two competing canonical write paths" — it is one
  *engine-internal-reconcile* mechanism (the dispatcher) and one *script-visibility*
  mechanism (the seam). The bug is that script-visible mutations don't all reach
  the second one. **In production (boa) Mechanism A's consumer fan-out is not even
  installed**, which is fine — those consumers are not script-observable; the
  script-visibility gap is in Mechanism B.
- **Recommended canonical path (§4): the ScriptSession seam owns
  `MutationObserver` record production.** Per `CLAUDE.md` L16, MO visibility is a
  ScriptSession-seam responsibility (mutation buffer + flush), so the fix is to
  make **every script-visible mutation** (attribute / tree / characterData /
  Range / `normalize` / innerHTML) enter the seam via `record_mutation` and have
  `MutationObserver` drain at flush — **not** to make MO a `ConsumerDispatcher`
  consumer (the earlier draft's Option 1, now rejected). Rejection reasons,
  developed in §4: (a) Option 1 **inverts the mandate** — it puts a
  script-observable responsibility in the `EcsDom` layer and bypasses the seam;
  (b) Option 1 **breaks record shape** — innerHTML's single coalesced childList
  record (8YcO) splits into N per-node records when driven off per-node
  `Insert`/`Remove` events, whereas the seam's `apply_*` already produces the
  spec-correct bulk record by construction. The `ConsumerDispatcher` is **kept**,
  but honestly scoped to engine-internal reconcile (it is the EcsDom-internal
  detail *below* the seam, not a second script-visible write path). **The design
  §12 picture (writes via `session.record_mutation`) and ADR #17 (seam provides
  "consistent MutationObserver records" in a single mechanism) are NOT stale —
  they match the mandate; the *code* drifted.** This is a "One issue, one way"
  decision to bring the code back to the design/mandate, not the reverse. A
  flush→MO-delivery hook must be wired (it does not exist today, §2.2 / 8YcR);
  `session.flush` is the natural drain point.

---

## 1. VM `vm/host/` DOM Write-Site Map

Seeded from umbrella Appendix A, **re-verified at HEAD `76c1677f`**. Each site is
classified and tagged **bridge** (routes through
`dom_bridge::invoke_dom_api` → an `elidex-dom-api` `DomApiHandler`) or
**direct** (calls `EcsDom::*` straight from `vm/host/`). Note: "bridge" here
means *dispatch* routing — it does **not** imply the session buffer; see §1.5.

### 1.1 Attribute API (`element_attrs.rs` + Attr/NamedNodeMap)

| Site | Method | Path | Notes |
|---|---|---|---|
| `element_attrs.rs:106` (`attr_set`) | helper for reflected setters & toggleAttribute | **direct** → `EcsDom::set_attribute` (`:112`) | thin shim, marshalling-only |
| `element_attrs.rs:155` (`attr_remove`) | helper for `removeAttribute` etc. | **direct** → `EcsDom::remove_attribute` (`:177`) | **does VM-local work the bridge handler does not**: snapshot-freezes any JS-held `Attr` wrapper's `detached_value` + `invalidate_attr_cache_entry` (`element_attrs.rs:180-187`); the bridge `RemoveAttribute` handler (`element/props.rs:108-122`) invalidates only the ECS `AttrEntityCache`, no VM-local detach. See §4.4 (B2 constraint). |
| `element_attrs.rs:205` (`native_element_set_attribute`) | `Element.setAttribute` | **bridge** → `invoke_dom_api("setAttribute", …)` (`:218`) | dom-api `SetAttribute` handler bottoms out at `EcsDom::set_attribute` (props.rs:70) |
| `element_attrs.rs:191` (`native_element_get_attribute`) | `Element.getAttribute` | **bridge** → `invoke_dom_api("getAttribute", …)` (`:202`) | read |
| `element_attrs.rs:226` (`native_element_remove_attribute`) | `Element.removeAttribute` | **direct** → `attr_remove` (`:235`) | **asymmetry: a `"removeAttribute"` handler is registered, but the VM bypasses it** (see §3) |
| `attr_proto.rs:416` | `Attr.value =` setter | **direct** → `EcsDom::set_attribute` | reflected-attr value write |
| `named_node_map.rs:345`/`:431` | `NamedNodeMap.setNamedItem`/`removeNamedItem` | **direct** → `EcsDom::set_attribute`/`remove_attribute` | |
| `element_attrs.rs:414`/`:535` | `setAttributeNode`/`removeAttributeNode` | **direct** → `EcsDom::set_attribute`/`remove_attribute` | |

### 1.2 Reflected IDL setters (direct `EcsDom::set_attribute` / `remove_attribute`)

All **direct**. These are content-attribute reflections (HTML §2.6.x); each
writes the backing content attribute through the `EcsDom` chokepoint.

- `html_input_proto.rs:460`/`:544`/`:687`/`:853`; `html_input_value.rs:129`/`:182`/`:253`/`:501`/`:535`
- `html_button_proto.rs:183`/`:246`/`:283`/`:324`/`:358`
- `html_select_proto.rs:254`/`:299`/`:364`
- `html_textarea_proto.rs:306`/`:373`/`:509`
- `html_form_proto.rs:246`/`:296`/`:334`/`:371`/`:403`
- `html_element_proto.rs:430`/`:714`/`:724`/`:750`/`:791`/`:835`/`:871`/`:904`
- `html_iframe_proto.rs:241`/`:292`; `html_option_proto.rs:178`/`:255`/`:293`
- `html_optgroup_proto.rs:106`/`:142`; `html_label_proto.rs:143`
- `html_fieldset_proto.rs:135`/`:171`; `canvas/mod.rs:780`
- `form_state_sync.rs:82`/`:111`

> **Layering note (Axis 1a):** reflected setters perform the attribute write
> *in `vm/host/`* (direct `EcsDom::set_attribute`) rather than via a
> `DomApiHandler`. The attribute-change *algorithm* is fully inside the
> `EcsDom` chokepoint, so this is arguably marshalling (set one content
> attribute), but it is **not** routed through the same `invoke_dom_api` seam
> the `setAttribute` API uses — a uniformity gap B2 should weigh (§4.4).

### 1.3 Tree mutations — bridge

- `node_proto.rs:709` (`appendChild`) — **bridge** → `invoke_dom_api`.
  `removeChild`/`insertBefore`/`replaceChild` are likewise registered handlers
  reached through the bridge.

### 1.4 Tree mutations — direct (NOT bridge)

- `parentnode.rs:126/127/172/225/236/249`
- `childnode.rs:199/355/356/429/430/491/518/522/523/551`
- `element_insert_adjacent.rs:176/187/190/198/213/218`
- `html_select_proto.rs:790/797/821/849/901/907`
- `dom_bridge.rs:136`

> **VM `insertAdjacent*` coverage (correcting a coverage-map slip).** The VM
> installs **only** `insertAdjacentElement` and `insertAdjacentText`
> (`vm/well_known.rs:341-342` → `vm/host/element_insert_adjacent.rs`). It does
> **not** install `insertAdjacentHTML`; that method exists **only** as a
> dom-api handler (`elidex-dom-api/registry.rs:101` `InsertAdjacentHtml`
> registered, body in `element/tree.rs`). So in the elidex-js VM, the
> HTML-parsing `insertAdjacentHTML` is unreachable today. The session-buffer
> `InsertAdjacentHtml` `Mutation` variant (§1.5/§2.2) is therefore a boa-path /
> dom-api-only producer, not a VM-native one.

### 1.4a Range mutations — direct, no `record_mutation` (8YcW)

The 3 mutating `Range` methods are script-visible **tree** (and, for
`deleteContents`/`extractContents`, **characterData**) mutations that must enter
the seam under the §4 direction. Today they bypass it entirely — each clones the
registered `Range`, runs the engine-indep mutation through `host.dom()` (a raw
`&mut EcsDom`), and commits boundary state back to the live-range registry. **No
`record_mutation`.**

| Site | Method | Path | Classification |
|---|---|---|---|
| `range_proto_mutation.rs:73`/`:97` (`native_range_delete_contents`) | `Range.deleteContents` | **direct** → `range.delete_contents(host.dom())` | tree (removed nodes) + characterData (partially-selected text split) |
| `range_proto_mutation.rs:102`/`:120` (`native_range_extract_contents`) | `Range.extractContents` | **direct** → `range.extract_contents(host.dom())` | tree + characterData |
| `range_proto_mutation.rs:125`/`:160` (`native_range_insert_node`) | `Range.insertNode` | **direct** → `transient.insert_node(host.dom())` | tree (added nodes) |

> Underlying `EcsDom` tree ops *do* fire Mechanism-A `Insert`/`Remove` events
> (so live-range / CE reconcile runs), but **no `MutationRecord`** is produced —
> these mutations are MutationObserver-silent. B1's test matrix (§3) must include
> `observe(parent,{childList:true})` + a `Range` mutation, which §3's earlier
> table omitted.

### 1.4b CharacterData splice + `normalize` — direct/bridge, characterData gap (8YcW / 8YcL)

| Site | Method | Path | Classification |
|---|---|---|---|
| `character_data_proto.rs:189`/`:223`/`:236`/`:250`/`:264` | `data=` / `appendData` / `insertData` / `deleteData` / `replaceData` | **bridge** → `invoke_dom_api` → `char_data/char_data_handlers.rs` | characterData |
| `node_methods_extras.rs:270` (`native_node_normalize`) | `Node.normalize` | **bridge** → `invoke_dom_api("normalize", …)` | tree (text-node removal) + characterData (merge) |

> **CommentData characterData hole (8YcL — confirmed by Read).**
> `set_char_data` (`elidex-dom-api/char_data/char_data_handlers.rs:45`) splits:
> the **Text / CDATASection** branch routes through `EcsDom::set_text_data`
> (`elidex-ecs/dom/mod.rs:332`), which bumps `rev_version` **and fires
> `MutationEvent::TextChange` via `dispatch_event`** (`:340-344`). The **Comment**
> branch (`char_data_handlers.rs:59-73`) writes `CommentData.0` in place and bumps
> `rev_version` **only — it calls no `dispatch_event`** (verified at HEAD
> `76c1677f`). So a `data=`/`appendData`/… on a **Comment** node fires **no**
> mutation event at all, and `observe(comment,{characterData:true})` is silent
> even on the dispatcher path. The §4 seam direction fixes this uniformly (the
> `record_mutation` is emitted at the handler/seam, independent of whether the
> EcsDom primitive dispatches), but B1 must call this out explicitly: Comment
> characterData currently has **no** notification of either kind, whereas Text has
> `TextChange`. §3's characterData row must split Text vs Comment.

### 1.5 The bridge does NOT mean the session buffer (critical refinement)

The audit (and a naive read of "bridge = aligned") assumes bridge-routed
mutations land in the session mutation buffer and thus produce
MutationObserver records. **They do not, except for innerHTML-class ops.** The
`elidex-dom-api` handlers themselves split:

- **`SetAttribute` / `RemoveAttribute`** (`element/props.rs:43`/`:82`) call
  `EcsDom::set_attribute` / `remove_attribute` **directly** ("Lesson #181:
  route through the canonical `EcsDom::set_attribute` chokepoint",
  `props.rs:61`/`:107`). **No `record_mutation`.**
- **Tree handlers** `AppendChild` / `InsertBefore` / `RemoveChild` /
  `ReplaceChild` (`element/tree.rs:35`/`:75`/`:90`/`:125`/`:177`) call
  `EcsDom::append_child` / `insert_before` / `remove_child` /
  `replace_child` **directly**. **No `record_mutation`.**
- **ChildNode/ParentNode mixins** (`before`/`after`/`remove`/`replaceWith`/
  `prepend`/`append`/`replaceChildren`) do the same — and say so:
  `child_node/mutations.rs:4-9` *"These handlers perform direct DOM operations
  … rather than going through `session.record_mutation()`. As a result, Custom
  Element lifecycle callbacks … are not automatically enqueued … Tracked for a
  future milestone."* (This CE-reaction note is now **partly stale** — CE
  reactions are driven by the `ConsumerDispatcher`'s
  `CustomElementReactionConsumer`, which fires off `MutationEvent::Insert`;
  but the underlying observation that these bypass `record_mutation` is exactly
  the MutationObserver gap of §3.)
- **Only `SetInnerHtml` / `InsertAdjacentHtml`** (`element/tree.rs:416`/`:476`)
  call `session.record_mutation(...)`, so only they populate the session buffer
  and thus reach MutationObserver via flush.

So **bridge ≠ observable**. The dispatch seam (bridge vs direct) is orthogonal
to MutationObserver coverage; both bottom out at the `EcsDom` chokepoint, which
notifies the `ConsumerDispatcher` but not the observer registry.

---

## 2. The Two Notification Mechanisms

There are exactly two, and they are disjoint in what they drive. **Read them as
answering two different questions, not as two competing canonical write paths:**
Mechanism A is **engine-internal derived-state reconciliation** (none of its
consumers is script-observable); Mechanism B is the **script-visibility** path
(`MutationObserver`). The §4 canonical-path decision is *not* "which mechanism
wins" — it is "route every script-visible mutation into Mechanism B (the
ScriptSession seam), and keep Mechanism A as the EcsDom-internal reconcile detail
*below* the seam".

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint) — **engine-internal reconcile, VM-only today**

> **Scope (mandate-relevant).** Every one of the 7 consumers below is
> **engine-internal derived-state reconciliation** — live-range adjustment,
> NodeIterator pre-removal, `<base href>` resolution, form-control state,
> event-handler-attr compilation, canvas reset, CE reactions. **None is
> script-observable** in the WHATWG-DOM sense (`MutationObserver` is *not* among
> them — see the last bullet). So this mechanism is **not** the canonical path
> for script-visible mutation; it is the EcsDom-internal detail *below* the
> ScriptSession seam. Keeping it is correct (§4); making it *also* own MO records
> (the rejected Option 1) is what violates the mandate.

- **Trigger:** every `EcsDom::set_attribute` / `remove_attribute` (via
  `dispatch_event`, `attribute.rs:118`/`:294`) and every tree mutation
  (`tree/mutation.rs:292`/`:356` fire `MutationEvent::Insert`/`Remove`).
- **Plumbing:** `EcsDom::dispatch_event` (`dom/mod.rs:191`) drives the single
  installed `Box<dyn MutationDispatcher>`. **That dispatcher is installed only by
  `Vm::bind` (`vm/vm_api.rs:279` → `ConsumerDispatcher`,
  `vm/consumer_dispatcher.rs:38`).** There is **no** `set_mutation_dispatcher`
  call in `elidex-js-boa` or `elidex-shell`, and the production pipeline runs
  boa (`pipeline.rs:9`/`:68`). So **in production today this dispatcher is
  absent** and `dispatch_event` fires into a no-op sink; the consumer fan-out
  below runs **only under the elidex-js VM**. (The inline
  `reconcile_attribute_derived_components` + `rev_version` — §2.1's last
  bullet — are baked into the `EcsDom` primitives, not consumers, so they run
  in *both* runtimes.) When the VM dispatcher is present it is synchronous,
  take-and-restore borrow, `debug_assert!(dispatch_depth == 0)` re-entry guard.
- **Consumers driven (field order = dispatch order,
  `consumer_dispatcher.rs:141-147`):**
  1. `LiveRangeBridge` — Range live-tracking (DOM §5.5).
  2. `NodeIteratorAdjuster` — NodeIterator pre-removing-steps (DOM §6.1).
  3. `BaseUrlMaintainer` — `<base href>` → `DocumentBaseUrl` (HTML §2.4.3).
  4. `FormControlReconciler` — `FormControlState` re-derivation (HTML §4.10.18.3).
  5. `EventHandlerAttributeConsumer` — `onclick="…"` → `EventListeners`
     (HTML §8.1.8.1).
  6. `CanvasReconciler` — `<canvas>` width/height bitmap reset (HTML §4.12.5).
  7. `CustomElementReactionConsumer` — CE v1 reactions
     (`connectedCallback`/`disconnectedCallback`/`attributeChangedCallback`,
     HTML §4.13.6).
- **Plus, baked into the chokepoint itself (not consumers):**
  `reconcile_attribute_derived_components` (inline derived components / style)
  and `rev_version` (live `HTMLCollection`/`NodeList` invalidation via the
  `inclusive_descendants_version` counter).
- **Shadow-root suppression (a record-source constraint, see §4).** The tree
  fire sites `fire_after_insert` / `fire_after_remove`
  (`tree/mutation.rs:289` and `:343-344`) **suppress** the
  `MutationEvent::Insert`/`Remove` entirely when *either* the node or its
  parent is a shadow root (light-tree-only contract). So any consumer driven
  off these events observes **no** event for `ShadowRoot` childList mutations.
  (This is why an event-driven MO source — the rejected §4 Option 1 — would also
  miss shadow-root mutations; the seam-driven source must record them from the
  `record_mutation` site, not from these suppressed events. See §4.2a C3.)
- **Crucially: `MutationObserver` is NOT a consumer here, and must not become
  one** — it belongs on the ScriptSession seam (§4), not in this
  engine-internal-reconcile fan-out.

### 2.2 Mechanism B — `SessionCore` mutation buffer + `flush`

- **API:** `SessionCore::record_mutation(Mutation)` buffers into `pending`;
  `SessionCore::flush(dom)` applies each via `apply_mutation` and returns
  `Vec<Option<MutationRecord>>` (`session.rs:79`/`:88`).
- **Production writers of `pending`:** the innerHTML-class `Mutation`
  variants — `elidex-dom-api/element/tree.rs:416` (`SetInnerHtml`), `:476`
  (`InsertAdjacentHtml`) — **plus the boa iframe attribute writes**
  (`elidex-js-boa/globals/iframe.rs:99`/`:105`/`:206` record
  `Mutation::SetAttribute`/`RemoveAttribute`). The boa iframe attribute path
  matters for §4: `apply_set_attribute`/`apply_remove_attribute`
  (`mutation/mod.rs:288-332`) write `Attributes` directly + reconcile +
  `rev_version` and **self-generate** the `MutationRecord` — they never enter
  `EcsDom::set_attribute` and never fire a `MutationEvent`. So attribute writes
  are **not** fully funneled through the `set_attribute` chokepoint; this
  buffered iframe write would sit *outside* a dispatcher-consumer SoT (§4.3
  constraint). **No elidex-js VM attribute or tree native records a `Mutation`.**
- **Flush drivers:** the shell — `re_render` (`elidex-shell/src/lib.rs:616`,
  with a bounded CE-stabilization re-flush loop `:634-660`) and
  `pipeline.rs:31`. **But not every flush delivers to MutationObserver.** The
  per-frame `re_render` path hands the returned records to
  `deliver_mutation_records` (`content/mod.rs:258`) — that is the **only** MO
  delivery site. The pipeline's `flush_with_ce_reactions`
  (`pipeline.rs:25-34`), used for initial-script execution + lifecycle
  finalization (`pipeline.rs:89`/`:93`/`:156`/`:205`/`:231`/`:245`), flushes
  records into **CE reactions only** and does **not** call
  `deliver_mutation_records`. So MutationObservers registered during page load
  miss mutations performed before the first per-frame re-render. B1's
  delivery wiring + tests must cover this flush path, not only `content/mod.rs`.
- **Consumer driven:** `MutationObserver` only —
  `Vm::deliver_mutation_records` (`vm_api.rs:867`) →
  `VmInner::deliver_mutation_records` (`mutation_observer.rs:418`) →
  `MutationObserverRegistry::notify` (per-record inclusive-ancestor walk over
  `MutationObservedBy`, DOM §4.3.2) → observer callbacks.
- **There is NO existing flush→MO microtask drain hook (8YcR).** The
  `Microtask::NotifyMutationObservers` enum variant
  (`natives_promise.rs:51-59`) exists, but its drain arm (`:333-344`) dispatches
  **only the `slotchange` half** (`dispatch_pending_slotchange_signals`,
  `:342`); the `MutationObserver`-callback half is **not** wired there — it is
  embedder-driven via `Vm::deliver_mutation_records`, which only the per-frame
  `content/mod.rs:258` site calls. So the corrected §4 direction still needs
  **new** wiring: `session.flush` (which *does* exist and returns the records)
  must drive MO delivery — either by extending `NotifyMutationObservers`'s drain
  to deliver buffered records at the §4.3.2 microtask checkpoint, or by a
  flush-tail delivery call. Either way the drain *point* (`flush`) exists; the
  *hook* from flush to MO does not. B1 wires it.

### 2.3 Overlap

The two mechanisms intersect **only at innerHTML/insertAdjacentHTML**:
`apply_mutation(SetInnerHtml)` ultimately drives `EcsDom` tree ops (Mechanism A
consumers fire) *and* yields a `MutationRecord` (Mechanism B → observer). For
all other writes, Mechanism A fires and Mechanism B is empty.

---

## 3. The MutationObserver Coverage Gap (exact)

The JS-level `MutationObserver` (WHATWG DOM §4.3) observes a mutation **iff a
`MutationRecord` is produced and delivered**, i.e. iff the mutation went through
`SessionCore::record_mutation` (or a direct `dom_inner_html.rs` deliver call).

**Records ARE produced for (in the elidex-js VM):**

- `innerHTML =` setter (`native_*set_inner_html`; explicit
  `deliver_mutation_records` at `dom_inner_html.rs:148`).
- **`outerHTML =` setter** (`native_element_set_outer_html`; explicit deliver at
  `dom_inner_html.rs:362` via `apply_set_outer_html`). **Correction:** the
  earlier draft labelled `:362` as `insertAdjacentHTML` — it is the **outerHTML
  setter**. The VM does **not** install `insertAdjacentHTML` at all (only
  `insertAdjacentElement`/`insertAdjacentText`, `well_known.rs:341-342`); the
  HTML-parsing `insertAdjacentHTML` lives only as the dom-api `InsertAdjacentHtml`
  handler (`registry.rs:101`).
- `InsertAdjacentHtml` / `SetInnerHtml` `Mutation` variants on the dom-api /
  boa path (delivered via `SessionCore::flush` → `deliver_mutation_records`,
  per-frame site only).
- `DOMParser`/fragment innerHTML on the boa path (boa-only).

**NO record is produced for (the gap):**

| Mutation kind | Example JS | Why no record |
|---|---|---|
| Attribute set | `el.setAttribute('x','1')` | `SetAttribute` handler → `EcsDom::set_attribute` direct; no `record_mutation` |
| Attribute remove | `el.removeAttribute('x')` | VM `attr_remove` → `EcsDom::remove_attribute` direct |
| Reflected IDL setter | `input.value`, `a.href`, `form.method`, … | direct `EcsDom::set_attribute` in `vm/host/*_proto.rs` |
| `appendChild` / `insertBefore` / `removeChild` / `replaceChild` | `p.appendChild(c)` | bridge handler → `EcsDom::append_child` direct; no `record_mutation` |
| ChildNode/ParentNode mixins | `el.remove()`, `el.before(x)`, `el.append(x)` | `child_node/mutations.rs` direct ops (self-documented `:4-9`) |
| **`Range` mutations (8YcW)** | `r.deleteContents()`, `r.extractContents()`, `r.insertNode(n)` | direct `range.{delete,extract}_contents`/`insert_node(host.dom())` (`range_proto_mutation.rs:73`/`:102`/`:125`); no `record_mutation` |
| **`Node.normalize` (8YcW)** | `el.normalize()` | bridge → `invoke_dom_api("normalize", …)` (`node_methods_extras.rs:270`); handler does direct EcsDom text removal/merge, no `record_mutation` |
| CharacterData on **Text** | `t.data = 'x'` | direct `EcsDom::set_text_data`; `ConsumerDispatcher` `TextChange` fires (engine-internal), but **no observer record** |
| CharacterData on **Comment (8YcL)** | `c.data = 'x'` | `set_char_data` Comment branch (`char_data_handlers.rs:59-73`) writes `CommentData.0` + `rev_version` **only — no `dispatch_event`**; so **neither** a `TextChange` event **nor** an observer record (worse than Text) |

So `new MutationObserver(cb).observe(el, {attributes:true, childList:true,
characterData:true})` in the elidex-js VM fires `cb` **only** when the subtree
is touched via `innerHTML`/`insertAdjacentHTML`. Every direct DOM API mutation
is silent. The gap is **uniform across the bridge and direct paths** — it is
*not* a bridge-vs-direct distinction (correcting the audit's framing). The
existing MutationObserver delivery tests construct `SessionRecord`s by hand and
call `deliver_mutation_records` directly; **none asserts a JS-level mutation
yields a record**, which is why the gap was not caught by tests.

### 3.1 Record-shape correctness still owed (for B1)

Even where records *are* produced, B1 must ensure full §4.3.3 shape across the
newly-covered kinds: `oldValue` (attributes + characterData), `attributeName`,
`attributeFilter` gating, `addedNodes`/`removedNodes`/`previousSibling`/
`nextSibling` (childList). `attributeNamespace` is already explicitly deferred
to `#11-mutation-observer-extras` (`mutation_event.rs:295-298`).

---

## 4. Recommendation — Canonical Path (for B1/B2)

> Written as **recommendation + trade-offs**, not a settled fix. The B1/B2
> plan-memos run `/elidex-plan-review`; this section is their starting premise.

### 4.1 The decision

**The ScriptSession seam owns `MutationObserver` record production.** Every
script-visible mutation — attribute set/remove, reflected IDL setters, tree
mutation (`appendChild`/`insertBefore`/`removeChild`/`replaceChild` + ChildNode/
ParentNode mixins), characterData (Text **and Comment**), `Range`
delete/extract/insert, `Node.normalize`, and innerHTML/outerHTML — enters the
seam via `SessionCore::record_mutation` (or is already buffered there), and
`MutationObserver` drains at `flush` (the §4.3.2 microtask checkpoint). This is
**Option 2 done correctly** (§4.3). The earlier draft's **Option 1**
(MutationObserver as a `ConsumerDispatcher` consumer) is **rejected** (§4.2).

**The root mandate makes the direction unique — this is not a balance of
trade-offs.** `CLAUDE.md` L16: *"ScriptSession as the sole Script↔ECS boundary …
書き込みは session mutation と flush point に集約し、SameObject・**MutationObserver**・
atomic script-task visibility を同一機構で守る."* MutationObserver visibility is,
by mandate, a ScriptSession-seam responsibility. Option 1 inverts the mandate by
placing a script-observable responsibility in the engine-internal `EcsDom` layer.
Two further reasons, developed below, independently rule Option 1 out:

1. **Record-shape destruction (8YcO).** innerHTML/outerHTML produce **one
   coalesced** childList record (`apply_set_inner_html`,
   `html_fragment.rs:85-89`, returns a single `ChildList` record with both
   `added_nodes` and `removed_nodes`; delivered as a 1-element slice at
   `dom_inner_html.rs:148`/`:362`). The underlying op fires **N+M** per-node
   `Insert`/`Remove` events at the dispatcher. An event-driven Option-1 consumer
   would emit **N+M** records — diverging from DOM §4.3.2's single-record shape.
   The seam's `apply_*` already produces the spec-correct bulk record **by
   construction**; this is a *positive* argument for the seam direction (§4.3a).
2. **Mandate / SSoT alignment.** Design §12 (writes via `record_mutation`, flush
   generates MutationRecords) and ADR #17 (seam provides "consistent
   MutationObserver records" in a single mechanism) **match** the mandate. They
   are **not stale** — the *code* drifted by routing innerHTML through an explicit
   `deliver_mutation_records` rather than letting the seam own all script-visible
   mutation. The correction brings the code back to the design, not the reverse
   (§4.4 SSoT).

**The `ConsumerDispatcher` is kept**, honestly scoped: it is the
engine-internal derived-state reconciliation mechanism (§2.1, 7 consumers, none
script-observable). It sits **below** the ScriptSession seam as an EcsDom-internal
detail. The "One issue, one way" convergence is therefore: **ScriptSession is the
single write entry for script mutation; it delegates engine-internal reconcile to
the `EcsDom` chokepoint (which fans out via `ConsumerDispatcher`) and owns the MO
record at flush.** No second script-visible write path.

> **Lesson #181 is *not* in tension with this direction** (correcting the earlier
> draft, which treated #181 as forcing Option 1). #181 consolidated *attribute
> writes* onto the `EcsDom::set_attribute` chokepoint so **derived components /
> live ranges / form state reconcile synchronously at write time**. That is an
> *engine-internal-reconcile* concern (Mechanism A) and stays exactly as-is under
> this direction — `record_mutation`'s `apply_*` step still bottoms out at the
> same `EcsDom::set_attribute` chokepoint (e.g. `props.rs:43` `SetAttribute`
> handler), so the dispatcher fan-out is unchanged. What changes is only that the
> **MO record** is emitted at the seam, not derived from dispatcher events. #181
> governs *where reconcile happens*; the mandate governs *where MO visibility
> happens*. They are orthogonal and both satisfied.

### 4.2a Record-source constraints the canonical (seam) path must satisfy

The seam-owned MO source must reproduce these, each verified at HEAD `76c1677f`.
Note how the seam direction handles each **more cleanly** than an event-driven
consumer would:

- **C1 — replaceChild coalescing.** `EcsDom::replace_child`
  (`tree/mutation.rs:185-205`) fires **two** events: `fire_after_remove(old)`
  then `fire_after_insert(new)`. The spec single childList record for a replace
  carries `addedNodes` *and* `removedNodes` in **one** record — and the seam's
  `apply_replace_child` (`mutation/mod.rs:268-285`) **already does exactly that
  by construction**. *This is why the seam direction is cleaner:* the record is
  built from the `Mutation::ReplaceChild` intent, not reconstructed from two
  dispatcher events that a consumer would have to coalesce. B1 must ensure the
  `record_mutation` for the bridge/direct `replaceChild` natives carries the
  replace intent (not two separate add/remove records).
- **C2 — non-dispatching attribute writes.** `set_attribute_without_dispatch`
  (`attribute.rs:146`) fires **no** `MutationEvent` (used inside consumers where
  re-entry forbids dispatch). Form value-mode/type-change calls it to move the
  live value into the `value` content attribute (`elidex-form/value_mode.rs:222`).
  An event-driven Option-1 consumer would **never see** this — a hard hole. The
  **seam direction does not depend on the event firing at all**: if the spec
  requires this generated `value` write to be observable, the form algorithm
  records a `Mutation` at the seam (it already runs through `elidex-form`, an
  engine-indep crate that can call `record_mutation`). B1 decides per-spec whether
  this internal write is script-observable (it generally is **not** — it is an
  internal reflection, not a script-initiated `setAttribute`); document the
  decision either way.
- **C3 — shadow-root suppression.** `fire_after_insert`/`fire_after_remove`
  (`tree/mutation.rs:289`, `:343-344`) suppress Insert/Remove when node/parent is
  a ShadowRoot. An event-driven consumer would silently miss shadow-root
  childList mutations. The **seam direction records at the `record_mutation`
  site**, *upstream* of the dispatcher suppression, so shadow-root childList
  mutations are recordable without touching the light-tree-only suppression
  contract. B1 records them at the seam and lets the existing §4.3.2
  inclusive-ancestor walk gate delivery (a MO must explicitly observe inside the
  shadow tree).
- **C4 — boa buffered iframe writes.** `iframe.rs:99`/`:105`/`:206` already
  record `Mutation::SetAttribute`/`RemoveAttribute`, applied by
  `apply_set_attribute`/`apply_remove_attribute` (`mutation/mod.rs:288-332`)
  which self-generate the record. Under the seam direction these are **already on
  the canonical path** — they are exactly the shape every other attribute write
  should converge to. *This argues for the seam direction:* the residual buffered
  iframe path is not an exception to route around (as it was under Option 1) but
  the model the VM attribute natives should match. B1 converges VM attribute
  writes onto `record_mutation` and these iframe writes need no special-casing.
- **C5 — innerHTML/outerHTML bulk-coalescing (8YcO).** `apply_set_inner_html` /
  `apply_set_outer_html` (`html_fragment.rs:55`/`:116`) emit **one** coalesced
  `ChildList` record (`added_nodes` + `removed_nodes`) for a whole-subtree
  replace, even though the underlying op does N `remove_child` + M `append_child`
  (each firing a per-node dispatcher event). The seam path **preserves this bulk
  shape by construction** (the record is built from the `SetInnerHtml` intent);
  an event-driven Option-1 source would shatter it into N+M records. B1 must
  retire the *explicit* `deliver_mutation_records` at `dom_inner_html.rs:148`/
  `:362` and route innerHTML/outerHTML through the same seam-flush delivery as
  every other mutation, so there is exactly **one** delivery path (no
  double-delivery, no per-node shattering).

### 4.2 Option 1 — MutationObserver as a `ConsumerDispatcher` consumer (REJECTED)

Add a `MutationObserverConsumer` as a new typed field on `ConsumerDispatcher`,
translating each `MutationEvent` into a `MutationRecord` routed to the observer
registry. **Rejected.** It was attractive only because it rides the *current
code shape* (the dispatcher already fans out at the chokepoint) — a
reactive-to-current-code anchor, not a mandate-derived one. The three reasons it
fails:

- **Mandate inversion (decisive).** It places `MutationObserver` — a
  *script-observable* responsibility — inside the engine-internal `EcsDom`
  dispatcher, **bypassing the ScriptSession seam** that `CLAUDE.md` L16 makes the
  *sole* Script↔ECS boundary. The mandate names MutationObserver explicitly as a
  seam-and-flush responsibility. This alone rules Option 1 out, independent of
  the mechanics below.
- **Record-shape destruction (C5 / 8YcO).** Driving records off per-node
  `Insert`/`Remove` events shatters innerHTML/outerHTML's single coalesced
  childList record into N+M records, and forces ad-hoc coalescing for
  replaceChild (C1). The seam path keeps the correct shape by construction.
- **Coverage holes by construction (C2/C3/C4).** It is blind to the
  non-dispatching `value` write (C2), the shadow-root-suppressed events (C3), and
  would have to special-case the buffered iframe path (C4) — each a place where
  *the dispatcher does not fire or is suppressed*, so an event-driven source
  cannot see it. The seam records *upstream* of all three.

The only superficial appeal of Option 1 ("no per-site edits") is also weaker than
it looks: the §1 write sites do **not** all reach the dispatcher (C2/C4), so
Option 1 still needs per-site work for the holes — while *also* violating the
mandate and breaking record shape. There is no version of Option 1 that is both
mandate-compliant and shape-correct.

### 4.3 Option 2 — ScriptSession seam owns MO record production (RECOMMENDED)

Every script-visible mutation enters the seam via `SessionCore::record_mutation`
(routing through `elidex-dom-api` / `DomApiHandler` so the algorithm stays in
engine-independent crates per the Layering mandate), and `MutationObserver`
drains at `flush`. The `apply_*` step still bottoms out at the `EcsDom`
chokepoint, so the `ConsumerDispatcher` fan-out (engine-internal reconcile,
lesson #181) is **unchanged** — only the MO record now originates at the seam.

- **Why this is the canonical "one way":**
  - **Mandate-compliant.** MutationObserver visibility lives on the ScriptSession
    seam, exactly as `CLAUDE.md` L16 and ADR #17 require — "SameObject,
    MutationObserver, atomic script-task visibility 守られる by the same
    mechanism" (the seam's identity map + mutation buffer + flush).
  - **Record shape correct by construction.** innerHTML/outerHTML (C5) and
    replaceChild (C1) already produce the spec-correct coalesced records on this
    path (`apply_set_inner_html`, `apply_replace_child`). No event-coalescing
    logic to get wrong.
  - **No coverage holes from dispatcher suppression.** Recording at the
    `record_mutation` site is upstream of shadow-root suppression (C3) and of the
    non-dispatching attribute write (C2); the buffered iframe path (C4) is
    *already* on this path. The seam is the natural superset.
  - **One write path.** ScriptSession is the single script-mutation entry; it
    delegates engine-internal reconcile to the `EcsDom` chokepoint and owns the
    MO record. No "new seam + N legacy" strangler state — the legacy state is the
    *current* split (innerHTML via explicit deliver, everything else silent), and
    this collapses it.
- **Work / risks (for plan-review):**
  - **Blast radius.** The §1 write sites (and their `elidex-dom-api` handlers)
    must record a `Mutation` (or the handler must, keeping `vm/host/`
    marshalling-only). This is larger than "one consumer", but it is the *correct*
    blast radius — it is what bringing every script-visible mutation onto the seam
    requires, and §4.2a shows the event-driven shortcut does not actually avoid
    per-site work. Sequence per §4.5; B2 (§4.6) folds the bridge/direct +
    reflected-setter convergence into the same pass.
  - **Flush→MO delivery hook is new (8YcR).** §2.2 confirmed no such hook exists
    today (`Microtask::NotifyMutationObservers` wires only `slotchange`). B1 wires
    `flush` → MO delivery at the §4.3.2 microtask checkpoint, covering **both** the
    per-frame `re_render` flush and the `flush_with_ce_reactions` page-load flush
    (§2.2) so observers registered during load are not missed.
  - **Retire the explicit innerHTML/outerHTML deliver (C5).** Remove the
    `deliver_mutation_records` calls at `dom_inner_html.rs:148`/`:362` once the
    seam-flush path delivers, so there is exactly one delivery path (no
    double-delivery).
  - **CommentData notification (8YcL).** Comment characterData fires no event
    today; the seam records it at the `set_char_data`/handler site regardless,
    closing the hole uniformly with Text.
  - **`oldValue` threading.** `characterDataOldValue` / attribute `oldValue` need
    the pre-write value captured at the `record_mutation` site (the handler has it
    in hand before calling `EcsDom`); `attributeNamespace` stays deferred to
    `#11-mutation-observer-extras`.
  - **Dual-runtime.** Both VM and boa flush through `SessionCore`, so the seam
    direction is runtime-uniform; only the (new) flush→MO hook must exist in the
    production (boa) flush path until S5 removes boa. This is *less* S5-coupled
    than Option 1 (which needed the VM-only dispatcher), not more.

### 4.3a Why innerHTML record-shape is a positive argument for the seam (8YcO)

The clearest single discriminator: `apply_set_inner_html`
(`html_fragment.rs:85-89`) does N `remove_child` + M `append_child` internally
(N+M dispatcher events) but returns **one** `ChildList` `MutationRecord` carrying
both `added_nodes` and `removed_nodes`. DOM §4.3.2 specifies exactly this single
coalesced record. The seam produces it from the *intent* (`SetInnerHtml`); an
event-driven consumer (Option 1) would have to reconstruct it by coalescing N+M
events on the same parent within a dispatch — fragile and not what the dispatcher
events even carry (they are per-node, with no "this is part of a bulk replace"
marker). The seam direction is correct *because* the record shape is owned where
the bulk intent lives. The same argument applies to replaceChild (C1).

### 4.5 B2 — bridge/direct + setAttribute/removeAttribute + reflected-setter convergence

§1 surfaced a uniformity gap that folds naturally into the seam convergence
(§4.3): every script-visible attribute write should reach the seam **through a
`DomApiHandler`**, with `vm/host/` staying marshalling-only (Layering mandate).
B2 collapses the decision surface in the same direction as B1:

- `setAttribute` routes through `invoke_dom_api` (`element_attrs.rs:218`) but
  `removeAttribute` uses the file-local `attr_remove` helper
  (`element_attrs.rs:155`) even though a `"removeAttribute"` `DomApiHandler` is
  registered (`element/props.rs:82`). Routing `removeAttribute` through the
  bridge for symmetry is **not a pure dispatch move**: `attr_remove`
  (`element_attrs.rs:180-187`) does **VM-local work the bridge handler does
  not** — it snapshot-freezes any JS-held `Attr` wrapper's `detached_value` and
  calls `invalidate_attr_cache_entry` so a later `el.setAttribute(name, v2)`
  cannot make a previously-held `Attr` appear to track `v2` (Chrome/Firefox both
  return the removal-time snapshot). The bridge `RemoveAttribute` handler
  (`element/props.rs:108-122`) invalidates only the ECS `AttrEntityCache`. So
  **B2 must carry the VM-local Attr-wrapper detach forward after the bridge
  move** (e.g. a VM-side post-step or having the handler signal the VM to freeze
  + invalidate), else held `Attr` objects regress across removal/re-add. This is
  a precondition on the symmetry fix, not an afterthought.
- **Reflected IDL setters route through the seam, not a blessed exception
  (8YcT — direction corrected).** Today `input.value`, `a.href`, `form.method`,
  etc. call `EcsDom::set_attribute` **directly from `vm/host/`**
  (`html_input_proto.rs:460`/`:544`/…, `html_input_value.rs:129`/`:182`/…,
  `html_button_proto.rs`, `html_textarea_proto.rs` — verified at HEAD
  `76c1677f`). The earlier draft offered "bless it as a documented direct-write
  marshalling exception" as one acceptable outcome. **That option is removed.**
  Under the seam direction these writes are exactly the ones that must become
  observable, and the Layering mandate keeps algorithm out of `vm/host/`: the
  layering-correct convergence is to route reflected setters through the **same
  `DomApiHandler` / `record_mutation` seam** as the `setAttribute` API (a shared
  `set_attribute` handler the reflected setter calls with the resolved content
  attribute name + value). Blessing the direct write would freeze a
  script-observable mutation outside the seam — the precise mandate violation B
  exists to fix. So: **one form — reflected writes through the seam** — not an
  exception.
- **`removeAttribute` symmetry carries the VM-local Attr detach (unchanged).**
  Routing VM `removeAttribute` through the bridge for symmetry with `setAttribute`
  must carry the `attr_remove` VM-local work forward — snapshot-freeze the JS-held
  `Attr` wrapper's `detached_value` + `invalidate_attr_cache_entry`
  (`element_attrs.rs:180-187`), which the bridge `RemoveAttribute` handler
  (`element/props.rs:108-122`) does not do (it invalidates only the ECS
  `AttrEntityCache`). This is a precondition on the symmetry fix; with the seam
  convergence it becomes a VM-side post-step the handler signals, not a reason to
  keep the direct path.

### 4.6 Sequencing

B1 (correctness: route every script-visible mutation onto the seam + wire
flush→MO delivery + close C1/C2/C3/C5/8YcL/8YcW) before — or folded with — B2
(the bridge/direct + reflected-setter convergence of §4.5). Because both move in
the **same** direction (everything onto the `DomApiHandler`/`record_mutation`
seam), plan-review may find B2 is simply the write-site half of B1's change
rather than a separate slice. Both are `/elidex-plan-review`-gated per the
umbrella; each is a single invariant-axis-intersection terminal slice under the
approved umbrella. **Dual-runtime note:** the seam path is runtime-uniform (both
VM and boa flush through `SessionCore`); only the new flush→MO hook must exist in
the boa flush path until S5 retires boa — a strictly smaller S5 coupling than the
rejected Option 1 carried.

---

## 5. Spec / Design SSoT Cross-Reference

- **WHATWG DOM §4.3** — MutationObserver; §4.3.2 "queue a mutation record"
  (per-observer queue, microtask delivery, inclusive-ancestor target walk);
  §4.3.3 Interface MutationRecord (record shape).
- **`docs/design/ja/12-dom-cssom.md`** — line 5: read-only `&EcsDom`, "書き込みは
  `session.record_mutation()`経由"; line 28: "MutationObserver … セッションflushが
  バッファされた変更からMutationRecordsを生成。ファーストクラス." **This is NOT a stale
  aspiration — it matches the mandate (8YcV).** The §4 direction is to bring the
  code back to §12, not to rewrite §12 to match drifted code. §12 needs at most a
  clarifying note that the `EcsDom` chokepoint + `ConsumerDispatcher` is the
  *engine-internal-reconcile* layer *below* the seam (so the two are not in
  conflict: writes record at the seam, the seam's `apply_*` delegates reconcile to
  the chokepoint). No SSoT-vs-code conflict remains once the code records at the
  seam.
- **`docs/design/ja/28-adr.md`** — ADR #17 (`ScriptSession` = unified Script↔ECS
  boundary providing Identity Map / **Mutation Buffer** / GC / **consistent
  MutationObserver records** "単一メカニズムで実現") — this is the **SSoT for the
  boundary** and directly mandates the §4 seam direction. ADR #14 ("MutationObserver
  がECS変更検出に自然にマッピング") describes the *implementation substrate* (ECS change
  detection feeds the seam's record production); it does **not** authorize putting
  MO production in the `EcsDom` layer — ADR #17 is the controlling SSoT for *where*
  the boundary sits.
- **`CLAUDE.md`** — L16 "ScriptSession as the sole Script↔ECS boundary … 書き込みは
  session mutation と flush point に集約し、SameObject・MutationObserver・atomic
  script-task visibility を同一機構で守る" (the decisive mandate, §4.1); "One issue,
  one way"; Layering mandate (Axis 1a — algorithm bodies belong in
  engine-independent crates, `vm/host/` is marshalling-only — see §4.5
  reflected-setter convergence).
- **lesson #181** (cited in code: `attribute.rs:5-15`, `element/props.rs:61`)
  — the canonical `EcsDom::set_attribute` write-path consolidation. **Preserved**
  under the §4 seam direction (§4.1 note): #181 governs *engine-internal reconcile
  at write time* (Mechanism A), the mandate governs *MO visibility* (the seam);
  they are orthogonal. The seam's `apply_*` still bottoms out at the #181
  chokepoint, so the consolidation is not unwound.

---

## 6. Re-check Discipline (for B1/B2 plan-memos)

- Re-grep every `file:line` here at PR-open — line numbers will drift.
- Re-confirm the §2 mechanism by direct read of `attribute.rs`
  (`set_attribute`/`dispatch_event`), `tree/mutation.rs`
  (`Insert`/`Remove` fire sites), `consumer_dispatcher.rs` (consumer list),
  and `mutation_observer.rs` (`deliver_mutation_records`). Do not carry this
  reframe forward on trust — Program B's correctness depends on it.
- Re-confirm the §4 seam-direction anchors by direct read: `set_char_data`
  Comment branch (`char_data/char_data_handlers.rs:59-73`, no `dispatch_event` —
  8YcL), `apply_set_inner_html` single-record shape (`html_fragment.rs:85-89` —
  8YcO/C5), the missing flush→MO hook (`natives_promise.rs:333-344` dispatches
  slotchange only — 8YcR), and the reflected-setter direct writes
  (`html_input_value.rs:129`/`:182` etc. — 8YcT).
- Re-check active branches (`git branch -r`) for convergence drift on
  `element_attrs.rs` / `vm/host/` attribute setters (the umbrella flags MED
  collision with JS-side work; low overlap with media Slice 2b today, but B is
  later — Axis 5).
- Slot check: `#11-mutation-observer-extras` (attributeNamespace, primitive
  ToObject for `observe`) must still be open before referencing it.

## Review guidelines (for Codex)

- This is a **doc-only** audit. Verify the `file:line` anchors against
  `main` and challenge any mechanism claim that does not match the code —
  especially §0/§3 (the MutationObserver gap, incl. Range/normalize/Comment
  rows) and §4 (the **ScriptSession-seam-owned MO** canonical-path recommendation
  and the rejection of the dispatcher-consumer alternative).
- The §4 direction is **mandate-derived, not a balance of trade-offs**:
  `CLAUDE.md` L16 makes ScriptSession the sole boundary and names MutationObserver
  as a seam-and-flush responsibility. Flag if any step mis-reads that mandate, if
  the rejected dispatcher-consumer (former Option 1) is dismissed for a reason
  that does not hold, or if a §4.2a constraint (C1–C5) is mis-stated.
- Out of scope: implementing B1/B2; touching `element_attrs.rs`, reflected
  IDL setters, `range_proto_mutation.rs`, `char_data` handlers, or
  `ConsumerDispatcher`.
