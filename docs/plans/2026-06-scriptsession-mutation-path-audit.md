# ScriptSession Mutation-Path Audit (Program B / B0)

Audit date: 2026-06-20 JST
Status: **DOC ONLY — no `.rs` change.** This is the B0 deliverable of the
philosophy-alignment umbrella (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md`,
Program B). It confirms the F3 mechanism end-to-end against current `main`
(HEAD `b09f2ba9`) and recommends the canonical path for B1/B2.
Audience: Claude / maintainers (and Codex via the review guidelines below).

> **Runtime caveat (read first).** The audit below traces the **elidex-js VM**
> (`crates/script/elidex-js`). But the **production shell still runs boa**:
> `crates/shell/elidex-shell/src/pipeline.rs:9`/`:68` constructs
> `elidex_js_boa::JsRuntime`, and `lib.rs:39` re-exports it; S5/boa removal
> (D-26 PR7) is **not yet done**. The `ConsumerDispatcher` is installed **only**
> by `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:279`) — there is **no**
> `set_mutation_dispatcher` install in `elidex-js-boa` or `elidex-shell`. So
> Mechanism A (§2.1) is **VM-only**; today's production (boa) reaches
> MutationObserver exclusively through the session buffer + `deliver_mutation_records`
> path (§2.2). This makes the §4 recommendation **dual-runtime** and couples the
> canonical-path landing to S5/boa removal — see §0, §2, §4.

> **Why B0 before B1/B2.** The original audit (F3 in
> `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`) framed the
> problem as "DOM write paths bypass the `ScriptSession` mutation buffer / its
> observers". The umbrella §2.3 already overturned that framing; this doc
> **re-verifies the reframe by direct code read** (not by trusting the umbrella
> prose) and then sharpens it further — the real gap is wider than §2.3 stated.
> Every claim below carries a `file:line` anchor re-checked at HEAD `b09f2ba9`.
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
- **The two mechanisms are real but not the ones the audit named.** They are:
  (1) `EcsDom`'s `ConsumerDispatcher` (synchronous, the actual canonical path
  for attributes + tree mutations **in the VM**), and (2) `SessionCore`'s
  mutation buffer + `flush` → `deliver_mutation_records` (which feeds
  MutationObserver but is fed only by innerHTML-class ops). They overlap only at
  innerHTML. **In production (boa) Mechanism A's consumer fan-out is not even
  installed** — so today only Mechanism B's flush path reaches MO at all (and
  only for innerHTML-class ops, only at the per-frame flush site).
- **Recommended canonical path (§4): make `MutationObserver` a
  `ConsumerDispatcher` consumer** *for the VM*, *not* re-route VM writes through
  the session buffer — **but this is conditional, not unconditional.** Rationale:
  the implementation already converged on `ConsumerDispatcher` as canonical
  (lesson #181), and re-routing everything through `record_mutation` would unwind
  that and re-introduce a second write path. **The hard constraint** is that
  Option 1 only covers production **after** S5/boa removal (the dispatcher is
  VM-only); until then the boa runtime must keep its session-record delivery
  path (or grow a dispatcher install). Option 1 must additionally satisfy the
  record-source constraints §4 enumerates (replaceChild coalescing,
  non-dispatching attribute writes, shadow-root suppression, boa buffered iframe
  writes). The design-doc §12 picture (writes via `session.record_mutation`) is
  the **stale aspiration**; the code chose the dispatcher. This is a "One issue,
  one way" decision to make the design doc follow the code, not the reverse —
  but the convergence is **coupled to S5** and gated on the constraint set.

---

## 1. VM `vm/host/` DOM Write-Site Map

Seeded from umbrella Appendix A, **re-verified at HEAD `b09f2ba9`**. Each site is
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

There are exactly two, and they are disjoint in what they drive.

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint) — **VM-only today**

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
  off these events — including a future MutationObserver consumer (§4
  Option 1) — observes **no** event for `ShadowRoot` childList mutations.
- **Crucially: `MutationObserver` is NOT a consumer here.**

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
| `textContent` / CharacterData on text nodes | `t.data = 'x'` | direct `EcsDom::set_text_data`; `ConsumerDispatcher` `TextChange` fires, but no observer record |

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

### 4.1 The decision (conditional)

**Make `MutationObserver` a `ConsumerDispatcher` consumer** (Option 1 below) **for
the elidex-js VM**, and converge the session buffer's role to the
innerHTML-class ops it already owns — rather than re-routing every VM write
through `SessionCore::record_mutation` (Option 2). This is a **recommendation
under constraints**, not an unconditional fix:

1. **Dual-runtime / S5 coupling.** The `ConsumerDispatcher` is installed only by
   `Vm::bind`; the production shell runs boa (`pipeline.rs:9`/`:68`). So Option 1
   covers **production only after S5/boa removal**. Until then, the boa runtime
   reaches MO solely through the session-record delivery path (§2.2) and that
   path must be **preserved** (or boa must grow its own dispatcher install).
   Treat the canonical-path landing as **coupled to S5**, not independent.
2. **The new consumer must satisfy the record-source constraint set (§4.2a)** —
   replaceChild coalescing, non-dispatching attribute writes, shadow-root
   suppression, boa buffered iframe writes. Any of these silently breaks a
   §4.3.3-correct record stream if not handled.

The two options' trade-offs (§4.2/§4.3) stand; the framing is "Option 1 is the
recommended *target shape*, gated on S5 and on §4.2a", **not** "Option 1 is a
drop-in fix today".

### 4.2a Record-source constraints the canonical path must satisfy

Any canonical MO-record source — whether the §4.2 consumer or the §4.3 buffer —
must reproduce these, each verified at HEAD `b09f2ba9`:

- **C1 — replaceChild coalescing.** `EcsDom::replace_child`
  (`tree/mutation.rs:185-205`) fires **two** events:
  `fire_after_remove(old_child)` then `fire_after_insert(new_child)`. But the
  spec single childList record for a replace carries `addedNodes` *and*
  `removedNodes` in **one** record (the buffered `apply_replace_child`,
  `mutation/mod.rs:268-285`, does exactly that). An event→record 1:1 mapping
  would split replaceChild into **two** records, diverging from the buffer path
  and from DOM §4.3.2. The event source (or the consumer) must carry an explicit
  replace/coalescing shape, or the consumer must coalesce a remove+insert pair on
  the same parent within a dispatch.
- **C2 — non-dispatching attribute writes.** `set_attribute_without_dispatch`
  (`attribute.rs:146`) fires **no** `MutationEvent` (it is used inside consumers
  where re-entry forbids dispatch). Form value-mode/type-change calls it to move
  the live value into the `value` content attribute
  (`elidex-form/value_mode.rs:222`). A consumer-based MO source therefore **never
  sees** that generated `value` attribute mutation — it stays silent even after
  Option 1. Document this as a **known hole**; if observability of the generated
  `value` write is required, it needs an explicit record emission at that site.
- **C3 — shadow-root suppression.** `fire_after_insert`/`fire_after_remove`
  (`tree/mutation.rs:289`, `:343-344`) suppress Insert/Remove when node/parent is
  a ShadowRoot. A MO observing a shadow root's `childList`, or `ShadowRoot.innerHTML`
  after the explicit-record path is retired, gets **no** record unless either the
  light-tree-only suppression is changed or a separate shadow-root record path is
  added.
- **C4 — boa buffered iframe writes.** `iframe.rs:99`/`:105`/`:206` record
  `Mutation::SetAttribute`/`RemoveAttribute`, applied by
  `apply_set_attribute`/`apply_remove_attribute` (`mutation/mod.rs:288-332`)
  which self-generate the record **without** entering `EcsDom::set_attribute` or
  firing a `MutationEvent`. If Option 1 makes the dispatcher consumer the single
  record source, this buffered write lands **outside** that source. Either keep
  the buffer path producing its record for these, or route iframe attribute
  writes through the chokepoint so the consumer sees them.

### 4.2 Option 1 — MutationObserver as a `ConsumerDispatcher` consumer (RECOMMENDED)

Add a `MutationObserverConsumer` (in `elidex-api-observers` /
`elidex-script-session`) as a new typed field on `ConsumerDispatcher`. It
translates each `MutationEvent` into a `MutationRecord` and routes it to the
observer registry (queueing at the §4.3.2 microtask checkpoint, not
synchronously inside dispatch — the consumer enqueues, the VM drains at the
existing checkpoint, mirroring `CustomElementReactionConsumer`).

- **Pros:**
  - Single source of truth **in the VM**. The chokepoint already converged here
    (lesson #181); every attribute write *that goes through
    `EcsDom::set_attribute`*, reflected setter, tree mutation, and characterData
    change flows through the dispatcher. Adding one consumer covers most of the
    §3 gap in one place, with no per-site edits to the ~60 write sites of §1.
    (Exceptions that do **not** reach the dispatcher even in the VM — the
    non-dispatching `set_attribute_without_dispatch` write (C2), shadow-root
    suppression (C3), and the boa buffered iframe path (C4) — are not covered by
    consumer addition alone; see §4.2a.)
  - "One issue, one way": no new seam; consumer-addition is the established
    extension shape (event-as-data + typed composer field,
    `mutation_event.rs:6-24`).
  - No unwinding of lesson #181 / the `EcsDom`-chokepoint canonical write.
  - Ordering with CE reactions is compile-time-visible (field order); the
    `was_connected`/`old_value` data the observer record needs is already
    carried on the `MutationEvent` variants.
- **Cons / risks (for plan-review):**
  - **Production not covered until S5.** The dispatcher is VM-only
    (`vm_api.rs:279`); production runs boa. Option 1 covers production **only
    after** S5/boa removal. Until then either (a) ship Option 1 as VM-only and
    keep boa's session-record delivery path as the production MO source, or
    (b) install a dispatcher in boa too — the first is the natural sequencing
    given D-26 PR7 retires boa. Either way the canonical-path landing is
    **coupled to S5**, and B1 must state which.
  - **Record-source constraint set (§4.2a) — C1–C4.** The consumer must coalesce
    replaceChild (C1), is blind to non-dispatching `value` writes (C2), gets no
    shadow-root records (C3), and does not see boa buffered iframe writes (C4)
    unless those are handled explicitly. None is a blocker, but each is a
    correctness gap if unhandled.
  - **Double-delivery at innerHTML.** Today innerHTML records flow through the
    session buffer *and* would now also flow through the new consumer (the
    underlying `EcsDom` tree ops fire `Insert`/`Remove`). B1 must pick **one**
    path for innerHTML — almost certainly retire the explicit
    `dom_inner_html.rs:148`/`:362` delivers and the `SetInnerHtml`/
    `InsertAdjacentHtml` record production once the consumer covers tree ops,
    so innerHTML is observed via the same consumer (no coexistence).
  - **Record coalescing / batching semantics.** §4.3.2 queues records per
    observer and delivers at the microtask checkpoint; the consumer must
    enqueue (not deliver) to preserve atomic script-task visibility. Needs the
    VM drain point wired (same checkpoint as CE-reaction drain).
  - **characterData** `MutationEvent::TextChange`/`ReplaceData` carry UTF-16
    lengths, not the old string; `oldValue` for `characterDataOldValue` needs
    the pre-write value threaded (small `EcsDom` change — weigh in B1).

### 4.3 Option 2 — Route all VM writes through `SessionCore::record_mutation` (NOT recommended)

Make every VM attribute/tree native (and the dom-api handlers) record a
`Mutation` and rely on `flush` to produce records, matching design §12's
literal prose (`12-dom-cssom.md:24`/`:71-72`).

- **Pros:** matches the design doc's written model; one buffer, atomic flush.
- **Cons:**
  - **Unwinds lesson #181.** The codebase *deliberately* moved attribute writes
    to the `EcsDom::set_attribute` chokepoint (away from buffered mutations) so
    that derived components / live ranges / form state reconcile synchronously
    at write time. Re-buffering would reintroduce the second write path the
    chokepoint consolidated, and either duplicate or bypass the
    `ConsumerDispatcher` consumers.
    - **But the convergence is not total today** (correcting an over-stated
      dismissal): the boa iframe path *still* records buffered
      `Mutation::SetAttribute`/`RemoveAttribute` (C4, §4.2a) which
      `apply_set_attribute`/`apply_remove_attribute` apply **without** entering
      `EcsDom::set_attribute`. So a residual buffered attribute-write path
      already coexists with the chokepoint. This does **not** rescue Option 2
      (it argues for *removing* the residual buffered path, not generalizing
      it), but it means Option 1 cannot assume *all* attribute writes pass the
      chokepoint — the C4 buffered write must be handled explicitly.
  - **Two-writers problem.** Either the dispatcher consumers also have to run
    off `apply_mutation` (re-deriving the whole fan-out on a different seam), or
    they keep firing at the `EcsDom` chokepoint *and* records flow through the
    buffer — i.e. the very coexistence "One issue, one way" forbids.
  - **Bigger blast radius:** edits ~60 write sites + the dom-api handlers, vs.
    one consumer.

**Conclusion:** the design doc §12 (writes via `record_mutation`) describes an
*earlier* intended architecture; the implementation converged on the
`EcsDom` chokepoint + `ConsumerDispatcher` and that convergence is the better
"one way". The right correction is to **make MutationObserver join that
convergence (Option 1) and update design §12 to describe the chosen
mechanism**, not to drag the code back to the buffer.

### 4.4 B2 — bridge/direct + setAttribute/removeAttribute asymmetry

Independently of observer coverage, §1 surfaced a uniformity gap to fold into
B2 (One issue, one way):

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
- Reflected IDL setters write the content attribute directly in `vm/host/`
  rather than through a `DomApiHandler`. Since the attribute-change algorithm
  lives entirely in the `EcsDom` chokepoint, this is borderline-marshalling;
  B2 should decide whether to keep it (documented marshalling exception) or
  route reflected writes through a shared `set_attribute` seam. Either way, pick
  **one** form and document it; do not leave "API path through bridge, reflected
  path direct" as an undocumented split.

### 4.5 Sequencing

B1 (correctness: produce records, Option 1) before B2 (collapse the
bridge/direct + setAttribute/removeAttribute decision surface). B2 may fold into
B1 if plan-review finds the unification is the natural shape of the consumer
change. Both are `/elidex-plan-review`-gated per the umbrella; each is a single
invariant-axis-intersection terminal slice under the approved umbrella.

---

## 5. Spec / Design SSoT Cross-Reference

- **WHATWG DOM §4.3** — MutationObserver; §4.3.2 "queue a mutation record"
  (per-observer queue, microtask delivery, inclusive-ancestor target walk);
  §4.3.3 Interface MutationRecord (record shape).
- **`docs/design/ja/12-dom-cssom.md`** — line 24 / 71-72: "writes go via
  `session.record_mutation()`"; line 47: "session flush generates
  MutationRecords from buffered mutations". **This is the stale aspiration**
  the code diverged from; §4.2's recommendation is to update §12 to describe
  the `ConsumerDispatcher` mechanism the code actually uses.
- **`docs/design/ja/28-adr.md`** — ADR #17 (`ScriptSession` as the unified
  Script↔ECS boundary providing Identity Map / Mutation Buffer / GC /
  consistent MutationObserver records); ADR #14 (DomApiHandler + DomSpecLevel,
  "MutationObserver maps naturally to ECS change detection").
- **`CLAUDE.md`** — "ScriptSession as the sole Script↔ECS boundary"; "One issue,
  one way"; Layering mandate (Axis 1a — algorithm bodies belong in
  engine-independent crates, `vm/host/` is marshalling-only).
- **lesson #181** (cited in code: `attribute.rs:5-15`, `element/props.rs:61`)
  — the canonical `EcsDom::set_attribute` write-path consolidation that Option 2
  would unwind.

---

## 6. Re-check Discipline (for B1/B2 plan-memos)

- Re-grep every `file:line` here at PR-open — line numbers will drift.
- Re-confirm the §2 mechanism by direct read of `attribute.rs`
  (`set_attribute`/`dispatch_event`), `tree/mutation.rs`
  (`Insert`/`Remove` fire sites), `consumer_dispatcher.rs` (consumer list),
  and `mutation_observer.rs` (`deliver_mutation_records`). Do not carry this
  reframe forward on trust — Program B's correctness depends on it.
- Re-check active branches (`git branch -r`) for convergence drift on
  `element_attrs.rs` / `vm/host/` attribute setters (the umbrella flags MED
  collision with JS-side work; low overlap with media Slice 2b today, but B is
  later — Axis 5).
- Slot check: `#11-mutation-observer-extras` (attributeNamespace, primitive
  ToObject for `observe`) must still be open before referencing it.

## Review guidelines (for Codex)

- This is a **doc-only** audit. Verify the `file:line` anchors against
  `main` and challenge any mechanism claim that does not match the code —
  especially §0/§3 (the MutationObserver gap) and §4.2 vs §4.3 (the
  ConsumerDispatcher-vs-session-buffer canonical-path recommendation).
- The recommendation is intentionally a recommendation, not a settled fix;
  B1/B2 are `/elidex-plan-review`-gated. Flag if any §4 trade-off is
  mis-stated or if Option 2 is unfairly dismissed.
- Out of scope: implementing B1/B2; touching `element_attrs.rs`, reflected
  IDL setters, or `ConsumerDispatcher`.
