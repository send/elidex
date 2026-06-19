# ScriptSession Mutation-Path Audit (Program B / B0)

Audit date: 2026-06-20 JST
Status: **DOC ONLY — no `.rs` change.** This is the B0 deliverable of the
philosophy-alignment umbrella (`docs/plans/2026-06-elidex-philosophy-alignment-umbrella.md`,
Program B). It confirms the F3 mechanism end-to-end against current `main`
(HEAD `2f4a9d5a`) and recommends the canonical path for B1/B2.
Audience: Claude / maintainers (and Codex via the review guidelines below).

> **Why B0 before B1/B2.** The original audit (F3 in
> `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`) framed the
> problem as "DOM write paths bypass the `ScriptSession` mutation buffer / its
> observers". The umbrella §2.3 already overturned that framing; this doc
> **re-verifies the reframe by direct code read** (not by trusting the umbrella
> prose) and then sharpens it further — the real gap is wider than §2.3 stated.
> Every claim below carries a `file:line` anchor re-checked at HEAD `2f4a9d5a`.
> Re-grep at PR-open; this doc is itself a snapshot.

---

## 0. TL;DR

- **The audit's central inference is wrong.** Direct `EcsDom::set_attribute` /
  `remove_attribute` calls do **not** bypass observers, reconcilers, style
  derivation, or live collections. `EcsDom::set_attribute`
  (`crates/core/elidex-ecs/src/dom/attribute.rs:101`) *is* the canonical
  attribute-write chokepoint: it runs `reconcile_attribute_derived_components`
  + `rev_version` and then `dispatch_event(MutationEvent::AttributeChange)` to
  the single `ConsumerDispatcher`. Tree mutations
  (`append_child`/`remove_child`/`insert_before`,
  `crates/core/elidex-ecs/src/dom/tree/mutation.rs`) fire
  `MutationEvent::Insert`/`Remove` to the same dispatcher.
- **The real gap is the JS-level `MutationObserver`, and it is broader than the
  umbrella §2.3 stated.** `MutationObserver` is *not* a `ConsumerDispatcher`
  consumer. It is fed exclusively by `Vm::deliver_mutation_records`, which in
  the elidex-js VM is reached from **only three production sites**: the
  innerHTML/fragment natives (`dom_inner_html.rs:148`/`:362`) and the shell's
  per-frame flush (`content/mod.rs:258` ← `re_render` ←
  `SessionCore::flush`). The session buffer (`SessionCore::pending`) is
  populated in production by **only the `SetInnerHtml` / `InsertAdjacentHtml`
  `Mutation` variants** (`elidex-dom-api/element/tree.rs:416`/`:476`). Every
  *other* JS DOM write — `setAttribute`, `removeAttribute`, every reflected IDL
  setter, **and `appendChild`/`removeChild`/`insertBefore`/`replaceChild` even
  through the bridge** — produces **no `MutationRecord`** and is therefore
  unobservable by `new MutationObserver(...)`.
- **The two mechanisms are real but not the ones the audit named.** They are:
  (1) `EcsDom`'s `ConsumerDispatcher` (synchronous, the actual canonical path
  for attributes + tree mutations), and (2) `SessionCore`'s mutation buffer +
  `flush` → `deliver_mutation_records` (which feeds MutationObserver but is fed
  only by innerHTML-class ops). They overlap only at innerHTML.
- **Recommended canonical path (§4): make `MutationObserver` a
  `ConsumerDispatcher` consumer**, *not* re-route VM writes through the session
  buffer. Rationale: the implementation already converged on `ConsumerDispatcher`
  as canonical (lesson #181), and re-routing everything through `record_mutation`
  would unwind that and re-introduce a second write path. The design-doc §12
  picture (writes via `session.record_mutation`) is the **stale aspiration**;
  the code chose the dispatcher. This is a "One issue, one way" decision to make
  the design doc follow the code, not the reverse.

---

## 1. VM `vm/host/` DOM Write-Site Map

Seeded from umbrella Appendix A, **re-verified at HEAD `2f4a9d5a`**. Each site is
classified and tagged **bridge** (routes through
`dom_bridge::invoke_dom_api` → an `elidex-dom-api` `DomApiHandler`) or
**direct** (calls `EcsDom::*` straight from `vm/host/`). Note: "bridge" here
means *dispatch* routing — it does **not** imply the session buffer; see §1.5.

### 1.1 Attribute API (`element_attrs.rs` + Attr/NamedNodeMap)

| Site | Method | Path | Notes |
|---|---|---|---|
| `element_attrs.rs:106` (`attr_set`) | helper for reflected setters & toggleAttribute | **direct** → `EcsDom::set_attribute` (`:112`) | thin shim, marshalling-only |
| `element_attrs.rs:155` (`attr_remove`) | helper for `removeAttribute` etc. | **direct** → `EcsDom::remove_attribute` (`:177`) | also snapshots cached Attr wrapper (marshalling) |
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

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint)

- **Trigger:** every `EcsDom::set_attribute` / `remove_attribute` (via
  `dispatch_event`, `attribute.rs:118`/`:294`) and every tree mutation
  (`tree/mutation.rs:292`/`:356` fire `MutationEvent::Insert`/`Remove`).
- **Plumbing:** `EcsDom::dispatch_event` (`dom/mod.rs:191`) drives the single
  installed `Box<dyn MutationDispatcher>`, which in production is
  `ConsumerDispatcher` (`vm/consumer_dispatcher.rs:38`, installed by `Vm::bind`).
  Synchronous, take-and-restore borrow, `debug_assert!(dispatch_depth == 0)`
  re-entry guard.
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
- **Crucially: `MutationObserver` is NOT a consumer here.**

### 2.2 Mechanism B — `SessionCore` mutation buffer + `flush`

- **API:** `SessionCore::record_mutation(Mutation)` buffers into `pending`;
  `SessionCore::flush(dom)` applies each via `apply_mutation` and returns
  `Vec<Option<MutationRecord>>` (`session.rs:79`/`:88`).
- **Production writers of `pending`:** only the innerHTML-class `Mutation`
  variants — `elidex-dom-api/element/tree.rs:416` (`SetInnerHtml`), `:476`
  (`InsertAdjacentHtml`). (boa-only: `elidex-js-boa/globals/iframe.rs:99`/`:105`/`:206`.)
  **No elidex-js VM attribute or tree native records a `Mutation`.**
- **Flush drivers:** the shell — `re_render` (`elidex-shell/src/lib.rs:616`,
  with a bounded CE-stabilization re-flush loop `:634-660`) and
  `pipeline.rs:31`. The returned records are handed to
  `deliver_mutation_records` (`content/mod.rs:258`).
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

**Records ARE produced for:**

- `innerHTML =` setter (`SetInnerHtml`; also an explicit
  `deliver_mutation_records` at `dom_inner_html.rs:148`).
- `insertAdjacentHTML` (`InsertAdjacentHtml`; explicit deliver at `:362`).
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

### 4.1 The decision

**Make `MutationObserver` a `ConsumerDispatcher` consumer** (Option 1 below),
and converge the session buffer's role to the innerHTML-class ops it already
owns — rather than re-routing every VM write through
`SessionCore::record_mutation` (Option 2).

### 4.2 Option 1 — MutationObserver as a `ConsumerDispatcher` consumer (RECOMMENDED)

Add a `MutationObserverConsumer` (in `elidex-api-observers` /
`elidex-script-session`) as a new typed field on `ConsumerDispatcher`. It
translates each `MutationEvent` into a `MutationRecord` and routes it to the
observer registry (queueing at the §4.3.2 microtask checkpoint, not
synchronously inside dispatch — the consumer enqueues, the VM drains at the
existing checkpoint, mirroring `CustomElementReactionConsumer`).

- **Pros:**
  - Single source of truth. The chokepoint already converged here (lesson
    #181); every attribute write, reflected setter, tree mutation, and
    characterData change *already* flows through `ConsumerDispatcher`. Adding
    one consumer covers the **entire** gap of §3 in one place, with no per-site
    edits to the ~60 write sites of §1.
  - "One issue, one way": no new seam; consumer-addition is the established
    extension shape (event-as-data + typed composer field,
    `mutation_event.rs:6-24`).
  - No unwinding of lesson #181 / the `EcsDom`-chokepoint canonical write.
  - Ordering with CE reactions is compile-time-visible (field order); the
    `was_connected`/`old_value` data the observer record needs is already
    carried on the `MutationEvent` variants.
- **Cons / risks (for plan-review):**
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
  (`element_attrs.rs:235`) even though a `"removeAttribute"` `DomApiHandler` is
  registered (`element/props.rs:82`). Route `removeAttribute` through the bridge
  for symmetry.
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
