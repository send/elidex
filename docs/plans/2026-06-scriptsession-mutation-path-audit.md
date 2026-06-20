# ScriptSession Mutation-Path Audit (Program B / B0)

Audit date: 2026-06-20 JST
Status: **DOC ONLY — no `.rs` change.** This is the B0 deliverable of
Program B (ScriptSession mutation-path / F3). Its normative basis is the
`CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須" rule and the
"ScriptSession as the sole Script↔ECS boundary" mandate (both committed SSoT);
the F1–F6 philosophy-alignment umbrella charter is **forthcoming in a separate
PR** and is referenced here only as a parent-program pointer, not as a
verifiable basis. B0 confirms the F3 mechanism end-to-end against current `main`
(HEAD `26d00c5a`) and hands the canonical-path question to B1's
`/elidex-plan-review`.
Audience: Claude / maintainers (and Codex via the review guidelines below).

> **Altitude of this audit (read first).** B0 establishes the **factual map**
> (which mechanisms exist, what the real gap is) and the **invariants** the
> canonical fix must satisfy — it does **not** characterise every write site,
> per-op event sequence, or per-API record shape. That detailed, code-level,
> exact-per-instance characterization is **B1's `/elidex-plan-review` job**: B1
> derives it by **grep-diff** against the live code (the methodology is in §1 / §6).
> An earlier revision pushed that per-op / per-API / per-parent detail into this
> audit and it became a *finding generator* — the op × parent-type × arity ×
> already-parented × light/shadow/boa combinatorial space cannot be precisely
> pinned in B0 prose, and every attempt to do so spawned a correction. So this
> revision states each fact as an **invariant + at most one or two illustrative
> examples**, and defers the exhaustive per-instance derivation to B1. Where you
> see "B1 derives the rest", that is deliberate, not an omission.

> **§4 status — open design question for B1, not a prescribed fix.** §4 sits on an
> **edge-dense coupled-invariant corner** — synchronous apply / read-your-writes ×
> `ConsumerDispatcher` fan-out × ScriptSession-MO ownership × record-shape
> coalescing × dual-runtime (boa/VM) — i.e. ≥3 intersecting invariant axes. Under
> `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須" +
> `feedback_coupled-invariant-design-corner`, a B0 *audit* prescribing a single
> canonical mechanism here would be a mandate violation: that decision belongs to
> B1's `/elidex-plan-review`, with the coupled invariants mapped upfront. §4 is
> therefore the *constraint set + coupled invariants* B1 must satisfy, presented as
> B1's input — not as B0's answer.

> **Runtime caveat (read first).** The audit below traces the **elidex-js VM**
> (`crates/script/elidex-js`). But the **production shell still runs boa**:
> `crates/shell/elidex-shell/src/pipeline.rs:9`/`:68` constructs
> `elidex_js_boa::JsRuntime`, and `lib.rs:39` re-exports it; S5/boa removal
> (D-26 PR7) is **not yet done**. The `ConsumerDispatcher` is installed **only**
> by `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:279`) — there is **no**
> `set_mutation_dispatcher` install in `elidex-js-boa` or `elidex-shell`. So
> Mechanism A (§2.1) is **VM-only**; today's production (boa) reaches
> MutationObserver exclusively through the session buffer + `deliver_mutation_records`
> path (§2.2). This dual-runtime split is one of the coupled axes that make §4 a
> B1 plan-review corner.

> **boa-runtime scoping note (read before any boa-specific path claim).** The
> **boa runtime is scheduled for removal in S5 / D-26 PR7** (the production shell
> runs it today, but the canonical Script↔ECS boundary is the elidex-js VM). Per
> `memory/feedback_boa-findings-light-touch`, this audit deliberately describes
> boa-specific mutation / CE-reaction / record-delivery paths only at a
> **known-to-differ** level — it does **not** exhaustively or precisely map them,
> because that map goes moot the moment boa is deleted. **The canonical
> MutationObserver design (§4, B1) targets the post-S5 VM runtime**; the exhaustive
> boa-specific path enumeration is **out of B1's scope** (it disappears with boa).
> Where a boa path is mentioned below, treat it as "known-to-differ, not
> load-bearing for B1", not as a precise contract to converge onto.

> **Why B0 before B1/B2.** The original audit (F3 in
> `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`) framed the
> problem as "DOM write paths bypass the `ScriptSession` mutation buffer / its
> observers". This doc **re-verifies and corrects that framing by direct code
> read**: direct `EcsDom::set_attribute` is *not* a bypass (it is the canonical
> chokepoint that fans out via `ConsumerDispatcher`), so the real gap is narrower
> on the *attribute-bypass* axis but **wider** on the *MutationObserver* axis than
> the original framing. B0's job is to establish that factual map (§1–§3) and to
> *enumerate the coupled invariants* the canonical fix must satisfy (§4), **not** to
> pick the mechanism — that is B1's `/elidex-plan-review`. Every claim carries a
> `file:line` anchor re-checked at HEAD `26d00c5a`; re-grep at PR-open — this doc is
> a snapshot.

---

## 0. TL;DR

- **The original audit's central inference is wrong.** Direct
  `EcsDom::set_attribute` / `remove_attribute` calls do **not** bypass observers,
  reconcilers, style derivation, or live collections. `EcsDom::set_attribute`
  (`crates/core/elidex-ecs/src/dom/attribute.rs:101`) *is* the canonical
  attribute-write chokepoint: it runs `reconcile_attribute_derived_components`
  + `rev_version` and then `dispatch_event(MutationEvent::AttributeChange)` to the
  installed `MutationDispatcher`. Tree mutations
  (`crates/core/elidex-ecs/src/dom/tree/mutation.rs`) fire
  `MutationEvent::Insert`/`Remove` to the same dispatcher.
  - **Caveat:** that dispatcher is `ConsumerDispatcher` **only in the elidex-js VM**
    (installed by `Vm::bind`, `vm_api.rs:279`). In today's production shell (boa),
    **no dispatcher is installed**, so the consumer fan-out (live ranges, CE
    reactions, etc.) does not run; the *inline* `reconcile_*` + `rev_version` (baked
    into the primitive, not consumers) still run. So "the chokepoint notifies the
    `ConsumerDispatcher`" is a **VM-only / post-S5** statement.
- **The real gap is the JS-level `MutationObserver`, broader than the original F3
  framing.** `MutationObserver` is *not* a `ConsumerDispatcher` consumer. It is fed
  by `deliver_mutation_records` — and **two distinct wirings** deliver (VM-direct
  synchronous delivery inside the HTML-fragment natives; boa per-frame flush
  delivery), which B1 must not conflate. With **one** existing exception — the boa
  `<iframe>` attribute setters self-generate a buffered record — every *other* JS
  DOM write (`setAttribute`, every reflected IDL setter, `appendChild` and the rest
  of the tree/childList family, even through the bridge) produces **no
  `MutationRecord`** and is unobservable by `new MutationObserver(...)`. This is the
  central factual finding (the §1 invariant + §3 gap).
- **There are two mechanisms, and they answer two *different* questions** —
  **not** a clean "dispatcher = non-observable / seam = MutationObserver"
  dichotomy. (1) **Mechanism A** — `EcsDom`'s `ConsumerDispatcher` (synchronous, at
  the chokepoint): *mostly* engine-internal derived-state reconcile, **plus a
  script-visible CE-reaction tap** (`CustomElementReactionConsumer` fires user JS
  lifecycle callbacks). (2) **Mechanism B** — `SessionCore`'s mutation buffer +
  `flush` → `deliver_mutation_records`: feeds `MutationObserver` (fed today only by
  innerHTML-class ops) **and also feeds CE reactions** (the shell drains flush
  records into `enqueue_ce_reactions_from_mutations`). **CE reactions are therefore
  a script-visible consumer driven from *both* mechanisms** — the key cross-mechanism
  fact B1 must preserve. The gap is that script-visible **MutationObserver**
  mutations do not all reach the seam.
- **Dual-runtime risk is broader than "MO is fine because it is separate".** In
  production (boa) the `ConsumerDispatcher` is **not installed** at all. Because the
  dispatcher's CE-reaction enqueue *is* script-visible, a runtime that lacks the
  dispatcher lacks that script-visible effect *via the dispatcher path* — boa
  instead drives CE reactions through a **separate** wiring, and B1 must show that
  wiring covers the same reactions. Any candidate that drops dispatcher fan-out must
  independently re-establish the CE-reaction (and base-url / form-control /
  event-handler / canvas) effects, not just the MutationObserver records.
- **Canonical-path decision is deferred to B1's `/elidex-plan-review` (§4).** B0
  does **not** prescribe the mechanism. §4 is an edge-dense coupled-invariant
  corner, and `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須"
  reserves that judgment for B1. What B0 fixes is the **constraint set** (§4.1 three
  coupled invariants + §4.2 record-source constraints). The three hardest, which any
  candidate must reconcile *simultaneously*:
  1. **Synchronous apply / read-your-writes** — `record_mutation` only buffers and
     applies at flush, so DOM-write *application* and MO-record *buffering* cannot
     be the same deferred step.
  2. **`ConsumerDispatcher` fan-out preservation** — the buffered
     `apply_set_attribute` bypasses `EcsDom::set_attribute`'s `dispatch_event`, so
     routing every write through it loses the consumer fan-out.
  3. **ScriptSession mandate** — MutationObserver visibility is a seam-and-flush
     responsibility per `CLAUDE.md`.
  These pull in different directions; no single naive routing satisfies all three —
  which is exactly why the choice is a B1 plan-review judgment, not a B0 verdict.

---

## 1. VM `vm/host/` DOM Write-Site Map (representative + invariant + B1 methodology)

> **This map is representative, not a hand-maintained registry.** Review history
> (R1→R5) showed that *hand-enumerating* write sites loses sites round after round.
> So this section is **invariant + representative known-set + B1 grep-diff
> methodology**, not an open-ended reactive list.

**Write-site invariant (the load-bearing statement).** *Any script-reachable
mutation that reaches an `EcsDom` / component mutator (`set_attribute` /
`remove_attribute` / `append_child` / `remove_child` / `insert_before` /
`replace_child` / `set_text_data` / `Attributes::set` / `CommentData` direct
write, etc.) is **MutationObserver-silent unless it either (a) goes through
`SessionCore::record_mutation` AND its buffer is drained by a *delivering* flush,
OR (b) self-generates a `MutationRecord` and synchronously delivers it via
`Vm::deliver_mutation_records`.*** The VM `innerHTML`/`outerHTML` setters and
`Element`/`ShadowRoot.setHTMLUnsafe` are the representative case (b) — they call
`apply_set_inner_html`/`apply_set_outer_html` via `with_session_and_dom` then
`ctx.vm.deliver_mutation_records(&[rec])` (`dom_inner_html.rs:148`/`:362`). This
single property defines the §3 gap; it holds regardless of whether a site is
enumerated below. (The `EcsDom` chokepoint still drives Mechanism A — §2.1 — but
Mechanism A is not a `MutationObserver` source.)

> **The "delivering flush" qualifier on (a) is load-bearing.** `record_mutation`
> only *buffers*; whether that buffer becomes an MO record depends on *which flush*
> drains it. The per-frame `re_render` flush delivers (`content/mod.rs:258`), but
> `flush_with_ce_reactions` (`pipeline.rs:25-34`) flushes the buffer into **CE
> reactions only** and never calls `deliver_mutation_records`. So a `record_mutation`
> call whose buffer is only drained by `flush_with_ce_reactions` is **MO-silent
> despite recording**.

**B1 grep-diff methodology (the SoT for completeness, not this table).** B1 must
produce the exhaustive write-site list by grep-diff:

- *Covered set* = (a) every `SessionCore::record_mutation` call-site **whose flush
  reaches `deliver_mutation_records`** ∪ (b) every **direct
  `deliver_mutation_records` producer** (self-generated record + synchronous
  deliver, no `record_mutation`).
- *Mutator set* = every direct `EcsDom`/component-mutator call across the four
  layers: `crates/script/elidex-js/src/vm/host/`, `crates/dom/elidex-dom-api/`,
  `crates/script/elidex-js-boa/`, and the `elidex-ecs` mutators themselves.
- *Gap set* = mutators in neither covered set.

Diffing against `record_mutation` call-sites **alone** would both false-positive
the direct-delivery `innerHTML`/`outerHTML` natives and false-negative a
record-but-only-CE-flushed site — so the grep-diff must enumerate flush/delivery
sites alongside recording call-sites.

**Representative known-set (illustrative; B1 enumerates the rest).** Confirmed by
direct read at HEAD `26d00c5a`; a site's *absence* here does **not** mean it records
a `Mutation` — apply the invariant. Each is classified **bridge** (routes through
`dom_bridge::invoke_dom_api` → an `elidex-dom-api` `DomApiHandler`) or **direct**
(calls `EcsDom::*` straight from `vm/host/`); "bridge" means *dispatch* routing, not
the session buffer (see §1.5).

- **Attribute API** (`element_attrs.rs`, `attr_proto.rs`, `named_node_map.rs`):
  `setAttribute` is **bridge** (`:218` → `EcsDom::set_attribute` at `props.rs:70`);
  `removeAttribute`, `Attr.value=` (attached only), `setNamedItem`/`removeNamedItem`,
  `setAttributeNode`/`removeAttributeNode`, `toggleAttribute` are **direct**.
  - **VM-local detach asymmetry (B2 constraint, §4.5):** `attr_remove`
    (`element_attrs.rs:155-187`) snapshot-freezes a JS-held `Attr` wrapper's
    `detached_value` + `invalidate_attr_cache_entry`; the bridge `RemoveAttribute`
    handler (`element/props.rs:108-122`) invalidates only the ECS `AttrEntityCache`.
- **Reflected IDL setters** — all **direct** `EcsDom::set_attribute` /
  `remove_attribute` (content-attribute reflections, HTML §2.6.1; verified via
  webref). Examples: `html_input_proto.rs`, `html_button_proto.rs`,
  `html_select_proto.rs`, `html_textarea_proto.rs`, `html_form_proto.rs`,
  `html_element_proto.rs`, `html_iframe_proto.rs`, `canvas/mod.rs:780`.
  - **`HTMLInputElement.value` is the exception (8kHF):** its setter is a value-mode
    *dispatch* (`html_input_value.rs:120-129`), **not** "set the content attribute".
    In text-like mode it writes `FormControlState` live value (`Attributes`
    untouched, no `set_attribute`); only default/default-on mode writes the content
    attribute. So it is a live-state write in the common case — B1/B2 must not put it
    on an attribute/MO seam unconditionally (§3 gap table, §4.5).
- **Tree mutations** — `appendChild` is **bridge** (`node_proto.rs:709`);
  `parentnode.rs`, `childnode.rs`, `element_insert_adjacent.rs`,
  `html_select_proto.rs` tree ops are **direct**. The VM installs only
  `insertAdjacentElement`/`insertAdjacentText` (`well_known.rs:341-342`), **not**
  `insertAdjacentHTML` — the HTML-parsing variant lives only as a dom-api handler.
- **Range mutations** (`range_proto_mutation.rs`) — `deleteContents` /
  `extractContents` / `insertNode` run the engine-indep mutation through a raw
  `&mut EcsDom` with **no `record_mutation`**; tree/characterData mutations that are
  MutationObserver-silent today (B1's test matrix must include
  `observe(parent,{childList:true})` + a Range mutation).
- **CharacterData splice + `normalize`** (`character_data_proto.rs`,
  `node_methods_extras.rs`) — `data=`/`appendData`/`insertData`/`deleteData`/
  `replaceData` route **bridge** → `char_data_handlers.rs`; `normalize` is **bridge**.
  See the CommentData hole below.
- **textContent / nodeValue / splitText** (`node_methods/text_content.rs`,
  `text_proto.rs`) — representative R5 grep-diff catches; all hit an `EcsDom`/component
  mutator with **no `record_mutation`** (see invariant detail below).

### 1.5 The bridge does NOT mean the session buffer

Bridge-routing is **orthogonal** to MutationObserver coverage. The `elidex-dom-api`
handlers split: `SetAttribute`/`RemoveAttribute` and the tree/childList handlers
(`AppendChild`/`InsertBefore`/`RemoveChild`/`ReplaceChild`, plus the
ChildNode/ParentNode mixins) call `EcsDom::*` **directly** with **no
`record_mutation`** (and say so — `child_node/mutations.rs:4-9`). **Only**
`SetInnerHtml` / `InsertAdjacentHtml` (`element/tree.rs:416`/`:476`) call
`session.record_mutation(...)`. So **bridge ≠ observable**; both bridge and direct
bottom out at the `EcsDom` chokepoint, which notifies the `ConsumerDispatcher` but
not the observer registry.

### 1.6 Invariant corner cases (illustrative; B1 grep-diff derives the exhaustive set)

Two non-obvious cases the invariant subsumes — kept because they correct real
mis-conflations, not as an exhaustive boundary:

- **`textContent =` vs `nodeValue =` are different mutation classes (do NOT
  conflate).** Both in `node_methods/text_content.rs`. `textContent =`
  (`SetTextContentNodeKind`) is a **childList replace-all** on non-CharacterData
  hosts (element / `DocumentFragment` / `ShadowRoot` → `remove_child` loop +
  `append_child`, `:105-116`), characterData on Text/Comment. `nodeValue =`
  (`SetNodeValue`) is **characterData-only**; its non-CharacterData catch-all is an
  **explicit no-op** (`:153-155`) — per WHATWG DOM §4.4 `#dom-node-nodevalue`,
  `nodeValue` is `null`/setter-no-op on non-CharacterData, so `element.nodeValue='x'`
  is **mutation-free** (silent because it does *nothing*, not a dropped record). B1
  must not gap-row it as a missing childList record.
- **CommentData characterData has no notification of either kind.** `set_char_data`
  (`char_data_handlers.rs:45`) splits: the **Text/CDATASection** branch routes through
  `EcsDom::set_text_data`, which fires `MutationEvent::TextChange`; the **Comment**
  branch (`:59-73`) writes `CommentData.0` + `rev_version` **only — no
  `dispatch_event`**. So `data=`/`appendData`/… on a **Comment** fires no mutation
  event at all and is silent even on the dispatcher path (worse than Text). This
  couples with live-range adjustment — see §4.3 / the characterData coupled invariant.

---

## 2. The Two Notification Mechanisms

There are exactly two. **Read them as answering two different questions, not as two
competing canonical write paths.** §2 establishes the factual map only; it does
**not** prescribe a mechanism, and in particular does **not** assert "route every
write into Mechanism B".

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint; engine-internal reconcile, VM-only today)

**Mechanism-A fan-out invariant (the load-bearing statement).** A Mechanism-A
consumer fans out for a given mutation **iff both** hold:
- **(a) the `EcsDom` primitive actually fires the `MutationEvent`.** Tree mutations
  are **suppressed at the fire site only when the mutated node *or* its parent is
  itself a `ShadowRoot`** — `fire_after_insert`/`fire_after_remove`
  (`tree/mutation.rs:289`/`:343`) return early when `is_shadow_root(node) ||
  is_shadow_root(parent)` (the shadow-host boundary; #367 finding 82). This is **not**
  light-tree-only: a mutation deep inside a shadow tree, whose mutated node/parent is a
  normal element (not the `ShadowRoot` itself), **does fire** — the suppression is the
  host↔shadow-root boundary, not the whole shadow subtree (docstring `:274-276` is
  explicit that deeper shadow-tree mutations are NOT filtered here). Separately,
  `set_char_data`'s Comment branch / the non-CharacterData `nodeValue=` branch fire no
  `MutationEvent` at all (§1.6); **and**
- **(b) a `ConsumerDispatcher` is installed** — installed **only** by `Vm::bind`
  (`vm_api.rs:279`); boa installs none, so under boa `dispatch_event` reaches a
  **no-op sink** and **no** mutation drives Mechanism A regardless of tree position.

So shadow-host-boundary writes (mutated node/parent is the `ShadowRoot` itself —
suppressed events) and boa fragment writes (no dispatcher) **do not drive
Mechanism A**. The per-path overlap classification (§2.3) is **derived from this
invariant**, not a hand registry.

**Consumers (mostly engine-internal reconcile + one script-visible CE tap).** The
dispatcher drives 7 consumers in field/dispatch order
(`consumer_dispatcher.rs:141-147`): `LiveRangeBridge` (DOM §5.5),
`NodeIteratorAdjuster` (§6.1), `BaseUrlMaintainer` (HTML §2.4.3),
`FormControlReconciler` (§4.10.18.3), `EventHandlerAttributeConsumer` (§8.1.8.1),
`CanvasReconciler` (§4.12.5), and `CustomElementReactionConsumer` (§4.13.6). The
first six are **derived-state reconcilers** (several feeding script-readable state —
Range boundaries, NodeIterator reference node, compiled `onclick`, canvas bitmap,
form-control value); the seventh, **`CustomElementReactionConsumer`, is directly
script-visible** — it enqueues `connected`/`disconnected`/`attributeChangedCallback`
reactions drained by `flush_ce_reactions`, firing user JS. So Mechanism A is
**mostly** engine-internal reconcile **plus a script-visible CE-reaction tap**, not
purely non-observable. `MutationObserver` is **not** among its consumers.

**Trigger / plumbing.** The `MutationEvent` enum (`mutation_event.rs`) carries seven
variants (`AttributeChange`; `Insert`/`Remove`; the character-data variants
`TextChange`/`ReplaceData`/`SplitText`/`NormalizeMerge`, all **Text/CData-only** — the
Comment branch fires none, §1.6). `EcsDom::dispatch_event` (`dom/mod.rs:191`) drives
the single installed `Box<dyn MutationDispatcher>`, synchronous with a re-entry guard.
The inline `reconcile_attribute_derived_components` + `rev_version` are baked into the
primitives (run in both runtimes); the consumer fan-out runs **only under the VM**
(boa drives CE reactions through a separate wiring, `pipeline.rs:29-34` /
`ce.rs:137-145`, which B1 must show covers the same reactions).

### 2.2 Mechanism B — `SessionCore` mutation buffer + `flush`

- **API:** `SessionCore::record_mutation(Mutation)` buffers into `pending`;
  `flush(dom)` applies each via `apply_mutation` and returns
  `Vec<Option<MutationRecord>>` (`session.rs:79`/`:88`).
- **Production writers of `pending`:** the innerHTML-class variants `SetInnerHtml` /
  `InsertAdjacentHtml` (`elidex-dom-api/element/tree.rs:416`/`:476`) **plus the boa
  `<iframe>` attribute setters** (`elidex-js-boa/globals/iframe.rs`). The boa iframe
  path is the **one existing attribute write that bypasses `EcsDom::set_attribute`**:
  `apply_set_attribute`/`apply_remove_attribute` (`mutation/mod.rs:288-332`) write
  `Attributes` directly + reconcile + `rev_version` and **self-generate** the record,
  firing **no** `MutationEvent`. So attribute writes are not fully funneled through
  the chokepoint — a constraint §4 weighs (C4). **(boa-specific — known-to-differ per
  the scoping note, moot at S5; retained only because it is the one existing example
  of a Mechanism-B attribute write without a `MutationEvent`.)** **No elidex-js VM
  attribute or tree native records a `Mutation`.**
- **Flush drivers — not every flush delivers to MutationObserver.** The per-frame
  `re_render` path hands records to the **boa `JsRuntime::deliver_mutation_records`**
  (4-arg: records, session, dom, document) at `content/mod.rs:258` — the **only**
  production MO delivery site (distinct from the VM-direct 1-arg
  `Vm::deliver_mutation_records` used inside the innerHTML/outerHTML natives, §1 (b)). `flush_with_ce_reactions` (`pipeline.rs:25-34`),
  used for initial-script + lifecycle finalization, flushes records into **CE
  reactions only** and never calls `deliver_mutation_records`. So MutationObservers
  registered during page load miss mutations performed before the first per-frame
  re-render — B1's delivery wiring + tests must cover this flush path too.
- **Consumers — `MutationObserver` *and* CE reactions.** The flush records feed
  **two** script-visible consumers: (1) `MutationObserver` (`deliver_mutation_records`
  → `MutationObserverRegistry::notify`, per-record inclusive-ancestor walk, DOM
  §4.3.2); (2) **custom-element reactions** — the shell drains the same records into
  `enqueue_ce_reactions_from_mutations` (`ce.rs:137-145`) in both
  `flush_with_ce_reactions` and per-frame `re_render`. So **Mechanism B is not
  MO-only**, and **CE reactions are driven from *both* mechanisms** (dispatcher tap
  §2.1 *and* flush-side here) — B1's record-production/delivery changes must preserve
  CE-reaction semantics.
  - *(boa CE-reaction sourcing is two systems — flush-record scan + binding-direct
    enqueue in `globals/element/core.rs` — described at known-to-differ level only,
    moot at S5. The B1-relevant shape: a CE reaction can originate from either a
    flush-record scan or a binding-direct enqueue, so a record-production change must
    not double-enqueue or miss CE reactions.)*
- **There is NO existing flush→MO microtask drain hook.** The
  `Microtask::NotifyMutationObservers` variant (`natives_promise.rs:51-59`) exists,
  but its drain arm dispatches **only the `slotchange` half** (`:342`); the
  `MutationObserver`-callback half is **embedder-driven by a per-frame delivery call,
  not by any VM microtask**. The only production delivery site is the shell's
  per-frame `re_render`, which calls **boa `JsRuntime::deliver_mutation_records(records,
  session, dom, document)`** (4-arg) at `content/mod.rs:258` — this is **distinct**
  wiring from the VM-direct 1-arg `Vm::deliver_mutation_records` invoked synchronously
  inside the innerHTML/outerHTML natives (§1 case (b)). So any seam-fed mechanism (§4)
  needs **new** flush→MO wiring. The drain *point* (`flush`) exists; the *hook* does not.

### 2.3 Overlap (derived from the §2.1 fan-out invariant)

The two mechanisms touch the **HTML-fragment write family — innerHTML, outerHTML,
setHTMLUnsafe, insertAdjacentHTML** — but **not** as a uniform overlap. The
classification follows directly from the Mechanism-A fan-out invariant; the
representative members:
- **light-tree fragment write** (VM `innerHTML`/`outerHTML`/`setHTMLUnsafe` on a
  light-tree element) = **Mechanism A ∩ VM direct-delivery** — drives `EcsDom` tree
  ops whose fire sites fire `Insert`/`Remove` (consumers fan out once a VM dispatcher
  is bound) **and** synchronously delivers its own record (`dom_inner_html.rs:148`/
  `:362`), not buffered Mechanism B.
- **shadow-root fragment write** (`ShadowRoot.innerHTML`/`setHTMLUnsafe`, same shared
  `set_inner_html_for` helper) = **direct-delivery *only*, NOT Mechanism A** — these
  replace the **direct children of the `ShadowRoot`** (parent == the `ShadowRoot`
  entity), so the `EcsDom` tree ops hit the host-boundary suppression
  (`is_shadow_root(parent)`) and fire no `MutationEvent`, so no consumer runs. (A
  fragment write *deeper* in a shadow tree, where the parent is a normal element, is
  **not** suppressed — that case fires; see the §2.1 invariant correction.)
- **boa fragment write** (`SetInnerHtml`/`InsertAdjacentHtml` on the boa/dom-api
  path) = **Mechanism-B only** — boa installs no dispatcher, so its tree ops reach
  the no-op sink and the record reaches MO purely via `flush`.

So the family is a **three-way representative split** keyed by the fan-out invariant,
not a single uniform overlap. The exact per-path classification is the §1/§6
grep-diff's derivation; this three-member set is **representative**.

For most other writes, Mechanism A fires and Mechanism B is empty — but the blanket
"every non-fragment write fires Mechanism A" is **false** for three non-fragment
classes, illustrating the fan-out invariant's exceptions: (1) **Comment
character-data** drives *neither* mechanism (no `dispatch_event`, no record — §1.6);
(2) the **boa buffered `<iframe>` attribute write** drives Mechanism B but **no**
Mechanism-A event (boa-specific, moot at S5); (3) the **value-mode value-attr
migration** (`set_attribute_without_dispatch`, `value_mode.rs:222`) drives *neither*
mechanism — a direct `Attributes` write with no `MutationEvent` and no
`record_mutation` (engine-side; this is the §4.2 C2 write).

---

## 3. The MutationObserver Coverage Gap

The JS-level `MutationObserver` (WHATWG DOM §4.3) observes a mutation **iff a
`MutationRecord` is produced AND a delivering path hands it to
`deliver_mutation_records`** — i.e. the §1 invariant restated for the observer axis:
**(a)** through `record_mutation` *and* a delivering flush (per-frame `re_render` →
`content/mod.rs:258`, **not** the CE-only `flush_with_ce_reactions`), **or (b)** a
direct-delivery producer. **`record_mutation` is therefore NOT, by itself, equivalent
to observation.**

**Records ARE produced for:** *(elidex-js VM direct-delivery)* the `innerHTML =`
setter (deliver at `dom_inner_html.rs:148`), the `outerHTML =` setter (deliver at
`:362` via `apply_set_outer_html`), `Element`/`ShadowRoot.setHTMLUnsafe` (same shared
`set_inner_html_for` → deliver, `:125`/`:146`/`:148`); *(dom-api/boa path, via
`flush` → deliver, per-frame site only)* the `SetInnerHtml` and `InsertAdjacentHtml`
handler variants. **Note `insertAdjacentHTML` is NOT installed in the elidex-js VM**
(`well_known.rs:341-342` installs only `insertAdjacentElement`/`insertAdjacentText`,
#367); `InsertAdjacentHtml` records only via the dom-api/boa handler path, never via a
VM native. *Representative — the exhaustive direct-delivery producer set is the §1/§6
grep-diff.* (boa `DOMParser`/`outerHTML` are **not** counted as clean record-producing
coverage — boa-specific, known-to-differ, moot at S5.)

**NO record is produced for (the gap — representative; B1 grep-diff derives the
exhaustive set).** Each row's mutation class is *that write path's* actual class,
derived from the dispatch path, not assumed from the API name:

| Mutation kind | Example JS | Why no record |
|---|---|---|
| Attribute set/remove | `el.setAttribute('x','1')`, `el.removeAttribute('x')` | handler/VM → `EcsDom::set_attribute`/`remove_attribute` direct; no `record_mutation` |
| Reflected IDL setter (true reflections) | `a.href`, `form.method`, `input.type`, … | direct `EcsDom::set_attribute` in `vm/host/*_proto.rs` |
| `HTMLInputElement.value` (8kHF — **not** a reflection) | `input.value='x'` | value-mode dispatch; text-like mode = live `FormControlState` write (`Attributes` untouched) — must **not** emit a spurious attribute record |
| Tree / childList family | `p.appendChild(c)`, `el.remove()`, `el.before(x)`, `el.replaceChildren(...)` | bridge/direct `EcsDom` tree ops; no `record_mutation` |
| `Range` mutations | `r.deleteContents()`, `r.extractContents()`, `r.insertNode(n)` | direct `range.*(host.dom())`; no `record_mutation` |
| `Node.normalize` | `el.normalize()` | bridge handler does direct EcsDom text removal/merge; no `record_mutation` |
| `textContent =` (childList replace-all on element/fragment/shadow; characterData on Text/Comment) | `el.textContent='x'` | `SetTextContentNodeKind`; no `record_mutation` on any branch |
| `nodeValue =` (characterData-only; **no-op** on non-CharacterData) | `text.nodeValue='x'` / `el.nodeValue='x'` | `SetNodeValue`; characterData branches record nothing; non-CharacterData = no-op (silent because mutation-free, **not** a dropped record — §1.6) |
| `Text.prototype.splitText` | `text.splitText(3)` | inserts sibling + truncates via `set_text_data`; no `record_mutation` |
| CharacterData on **Text** | `t.data='x'` | direct `EcsDom::set_text_data`; `TextChange` fires (engine-internal) but **no observer record** |
| CharacterData on **Comment** | `c.data='x'` | Comment branch writes `CommentData.0` + `rev_version` **only — no `dispatch_event`**; neither event nor record (worse than Text — §1.6) |

So `new MutationObserver(cb).observe(el, {attributes,childList,characterData})` in
the VM fires `cb` **only** for the `innerHTML`/`outerHTML`/`setHTMLUnsafe` setters;
every direct DOM API mutation is silent. The gap is **uniform across bridge and
direct paths** — *not* a bridge-vs-direct distinction (correcting the original
framing). The existing MutationObserver tests build `SessionRecord`s by hand and call
`deliver_mutation_records` directly; **none asserts a JS-level mutation yields a
record**, which is why the gap was test-invisible.

### 3.1 Record-shape correctness still owed (for B1)

Even where records *are* produced, B1 must ensure full §4.3.3 shape across the
newly-covered kinds: `oldValue` (attributes + characterData), `attributeName`,
`attributeFilter` gating, `addedNodes`/`removedNodes`/`previousSibling`/`nextSibling`
(childList). `attributeNamespace` is already deferred to
`#11-mutation-observer-extras` (`mutation_event.rs:295-298`).

---

## 4. Canonical Path — open design question for B1 (constraints + coupled invariants)

> **This section does NOT prescribe a mechanism.** Per the §4 status callout, the
> canonical MutationObserver-record path is an **edge-dense coupled-invariant
> corner**; under `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須"
> that choice is **B1's `/elidex-plan-review` judgment**. What follows is the
> *input* B1 must satisfy: the three coupled invariants (§4.1) and the record-source
> constraints (§4.2).

### 4.1 The three coupled invariants any candidate mechanism must satisfy

Fixing one naively breaks another; B1 must satisfy **all three simultaneously**:

1. **Synchronous apply / read-your-writes.** `el.setAttribute('x','1');
   el.getAttribute('x')` must read `'1'` within the same script task — no flush
   between. But `record_mutation` (`session.rs:78-90`) **only buffers** and applies
   at `flush` (deferred). So MO-record *buffering* and DOM-write *application* cannot
   be the same deferred step.
2. **`ConsumerDispatcher` fan-out preservation.** The buffered `apply_set_attribute`
   (`mutation/mod.rs:288-313`) writes `attrs.set` + `reconcile_*` + `rev_version` and
   — "instead of entering `EcsDom::set_attribute`" (`:299-300`) — **does not call
   `dispatch_event`**. Routing every write through it **loses** the base-url /
   form-control / event-handler / canvas / CE consumer fan-out.
3. **ScriptSession mandate.** Per `CLAUDE.md` "ScriptSession as the sole Script↔ECS
   boundary … MutationObserver … を同一機構で守る", MutationObserver visibility is a
   seam-and-flush responsibility — so making `MutationObserver` a `ConsumerDispatcher`
   consumer inside the engine-internal `EcsDom` layer inverts the mandate (and
   shatters innerHTML's single coalesced childList record).

**Why these are genuinely coupled.** The simplest seam-only design ("every write
becomes a buffered `Mutation`, MO drains at flush") satisfies invariant 3 but breaks
1 (deferred apply) and 2 (no fan-out). The simplest dispatcher design
("MutationObserver is a `ConsumerDispatcher` consumer") satisfies 1 but breaks 3 and
record shape. **Neither pole is correct as-is.** A satisfying mechanism plausibly
keeps the synchronous write + dispatcher fan-out at the `EcsDom::set_attribute`
chokepoint (invariants 1+2) **while** producing the MO record at the ScriptSession
seam (invariant 3) — i.e. separates *where the write applies* from *where the MO
record originates*. How to wire that is the B1 design judgment.

> **Lesson #181 *does* tension with naive seam-only routing.** The buffered
> `apply_set_attribute` does **not** call `EcsDom::set_attribute` — it duplicates
> only the `reconcile_*` + `rev_version` fragment and explicitly bypasses the
> chokepoint. #181 consolidated attribute writes onto that chokepoint precisely so
> reconcile **and** `ConsumerDispatcher` fan-out happen at write time; the buffered
> seam path re-introduces the very fork #181 collapsed. The immediate dom-api
> `SetAttribute` *handler* (`element/props.rs:43`) honours #181 but records **no**
> `Mutation`. This non-equivalence (immediate-chokepoint-but-no-record vs.
> buffered-record-but-no-chokepoint) is the heart of the coupled corner; B1 resolves
> it.

### 4.2 Record-source constraints the canonical path must satisfy (invariants; B1 derives exact per-instance shape)

Whatever mechanism B1 chooses, its MO-record source must reproduce these invariants.
Where a constraint discriminates between recording *at a dispatcher event* vs.
*upstream of the dispatcher*, that is noted as a factor for B1 to weigh — it does
**not** settle the choice (invariants 1+2 of §4.1 pull the other way). The **exact
per-op event sequence / per-target record breakdown is B1's grep-diff/dispatch-path
derivation**, not a fixed contract here.

- **Coalescing & move-semantics invariant.** A whole-subtree replace (innerHTML /
  outerHTML / setHTMLUnsafe / `replaceChildren` / `textContent=` / `replaceChild`)
  must yield **one coalesced `ChildList` record** carrying `addedNodes` *and*
  `removedNodes` together, **per target parent** — WHATWG DOM §4.2.3 "replace all"
  (`#concept-node-replace-all`) step 7 (and §4.2.3 replace). **Coalescing is
  per-target-parent: only same-`target` add/remove events fold into one record.**
  Two corollaries:
  - **No-op guard:** a replace-all where both `addedNodes` and `removedNodes` are
    empty (`replaceChildren()` / `textContent=''` on an empty node) produces **no**
    record (step 7 queues only "if either is not empty"; webref-verified).
  - **Move (already-parented node):** any op that inserts an **already-parented Node
    passed as a node argument** (`replaceChild`/`replaceChildren`/`appendChild`/
    `insertBefore`/…) fires a leading pre-detach `Remove` **on the moved node's *old*
    parent** (`detach_with_hook` → `fire_after_remove(child, old_parent, …)`,
    `tree/mutation.rs:458`). That move `Remove`'s target is the **old parent — which may
    be the *same* parent as the destination** (e.g. `parent.appendChild(parent.firstChild)`
    / a re-order within one parent: `old_parent == new_parent`) **or a different
    parent** (cross-parent move). Coalescing is decided **purely by target identity**
    (§4.2 per-target-parent rule), independent of whether it is a move: the move
    `Remove` folds into the destination `ChildList` record **iff** `old_parent ==
    destination parent`, and is a separate record otherwise. (`detach_with_hook` skips
    the redundant `rev_version(old_parent)` when `old_parent == new_parent`,
    `:454-456`, but still fires the `Remove`.) (`textContent=` / `setHTMLUnsafe` take
    strings → fresh nodes and are **not** move-capable.)

  *Illustrative:* `apply_set_inner_html` removes-old-then-appends-new;
  `apply_set_outer_html` does the reverse (insert-new before `entity`, then remove
  `entity`); `replaceChild` parentless = old `Remove` → new `Insert` (one record on
  `parent`). **B1 derives each source's exact per-call sequence and per-target record
  span by grep-diff.** Today these replace-all ops are per-node `remove_child`/
  `append_child` loops with **no** record (§3 gap), carrying both the N+M-shatter risk
  and the missing-record gap. A seam/intent-driven source yields the coalesced shape
  by construction; an event-driven source must reconstruct it (and suppress the no-op
  case) — *favors recording from intent*, but does not settle the choice.

- **Ordering invariant (coalesced-record + CE-reaction order).** Within a coalesced
  record, added-vs-removed ordering is **load-bearing**: record production,
  coalescing, **and** the flush-side CE scan must agree on **one** total source order
  — and a **cross-parent** move `Remove` (a separate record, since `old_parent !=
  destination`) must keep its **fire order relative to** the destination record (it
  fires first), so the CE scan sees `disconnected`(old parent) before
  `connected`/`disconnected`(new parent). (A *same-parent* move re-order does not
  disconnect the node, so it queues no `disconnected`/`connected` reaction — this
  ordering concern is cross-parent-only.) The
  flush-side CE scan `enqueue_ce_reactions_from_mutations` (`ce.rs:145`) iterates
  added-then-removed; if a Pole-A reconstruction reorders, the
  `connected`/`disconnected` firing order inverts relative to today. **B1 derives each
  source's exact per-op order from the dispatch path** (it is *not* a fixed contract
  here); the load-bearing statement is only that the order must agree across record
  production, coalescing, and the CE scan, for **all** coalesced/replace-all sources.

- **Non-dispatching attribute writes (spec-observable, must be closed).**
  `set_attribute_without_dispatch` (`attribute.rs:146`) fires **no** `MutationEvent`.
  The value-mode type-change migration (`apply_type_change_value_migration`,
  `value_mode.rs:222`) uses it to move a non-empty live value into the `value`
  **content attribute** — a real content-attribute write that WHATWG DOM §4.9
  "handle attribute changes" (`#handle-attribute-changes`) step 1 **queues an
  `"attributes"` record** for. So a `{attributes:true, attributeFilter:['value']}`
  observer is **owed** that record (spec-settled, webref-verified). An event-driven
  (Pole A) source would **never see** it — a hard hole that makes Pole A insufficient
  alone. (This is the content-attribute migration only; the text-like-mode live-value
  write leaves `Attributes` untouched and must stay record-free — 8kHF.)

- **Shadow-host-boundary suppression.** `fire_after_insert`/`fire_after_remove`
  (`tree/mutation.rs:289`/`:343`) suppress Insert/Remove **only when the mutated node
  *or* its parent is itself a `ShadowRoot`** (the host↔shadow-root boundary) — a
  mutation deeper in a shadow tree (parent is a normal element) still fires. An
  event-driven (Pole A) source therefore silently misses the **boundary** childList
  mutations (e.g. `ShadowRoot.innerHTML`, where parent == the `ShadowRoot`); a record
  emitted *upstream* of the dispatcher (Pole B) captures them. *Favors recording
  upstream.* The §4.3.2 inclusive-ancestor walk still gates delivery.

- **boa buffered iframe writes.** `iframe.rs` already records
  `Mutation::SetAttribute`/`RemoveAttribute` via the buffered applier that
  **bypasses `EcsDom::set_attribute`** (no dispatcher fan-out, invariant 2). A
  *precedent* for a seam-recorded attribute write, but **not a clean model**: B1 must
  resolve which way convergence runs (route others onto this buffered path → lose
  fan-out; or route through the chokepoint → the iframe path needs reconciling).
  (boa-specific, moot at S5.)

- **CharacterData `oldValue` capture *timing* — record source coupled with capture
  ordering.** `{characterDataOldValue:true}` (DOM §4.3.3) needs the **pre-write**
  data, but `EcsDom::set_text_data` **overwrites the buffer before** firing
  `TextChange` (`dom/mod.rs:336`/`:340-344`), and `TextChange`/`ReplaceData` carry no
  old value. So an **event-driven (Pole A) characterData-oldValue source is
  impossible by construction** — the old value must be captured at the seam/handler
  *before* the `EcsDom` write. A hard reason Pole A cannot fully serve
  `characterDataOldValue`.

### 4.3 Cross-cutting work any direction inherits (for B1's plan)

Independent of which mechanism B1 picks:

- **Record-shape correctness.** The coalesced single-record shape for innerHTML /
  outerHTML / replaceChild (above) is the sharpest discriminator between the poles —
  a seam/intent source gets it by construction; an event-driven source must
  reconstruct it.
- **No double-delivery for the whole direct-delivery producer set** (innerHTML /
  outerHTML / `setHTMLUnsafe`; exhaustive set = §1/§6 grep-diff). Each self-generates
  a `MutationRecord` and synchronously delivers it via `Vm::deliver_mutation_records`,
  but **not through one shared helper**: innerHTML/`setHTMLUnsafe` deliver inside
  `set_inner_html_for` (`dom_inner_html.rs:148`, via `apply_set_inner_html`), while
  `outerHTML` delivers inside `native_element_set_outer_html` (`:362`, via
  `apply_set_outer_html`) — separate helpers, same direct-delivery invariant. So if B1
  adds a flush→MO path it must retire/reconcile direct delivery for **every** member
  (across both helpers), or a member would double-deliver. Stated over the producer
  **set as a whole**.
- **CE-reaction preservation (Mechanism B is not MO-only).** The session buffer feeds
  `MutationObserver` *and* CE reactions (§2.2). Any change to record *production*,
  *coalescing*, or *delivery ordering* must preserve the CE-reaction scan (same
  added/removed node set, **same order** — the ordering invariant above is named C7),
  and the flush→CE drain must keep running on the page-load
  (`flush_with_ce_reactions`) path, not only per-frame.
- **CommentData notification + live-range coupling — one coupled invariant, not a
  lone missing record.** Comment characterData fires no event today (§1.6). Per
  WHATWG DOM §4.10 CharacterData "replace data" (`#concept-cd-replace`), the same
  algorithm that **queues the `"characterData"` record** (step 4) **also adjusts
  every live range whose boundary point is inside the spliced node** (steps 8–11,
  over §5.5 "live range"). So a seam-emitted record **alone** leaves Range boundaries
  stale; the record and the live-range boundary adjustment are **one coupled
  invariant** B1 must satisfy together, for **all** character-data splices (Comment
  *and* Text/CData). (Today Text/CData boundary adjustment rides `LiveRangeBridge` off
  the dispatcher event; Comment fires no event, so it gets neither the record nor the
  adjustment — the hole is double.)
- **`oldValue` threading.** `characterDataOldValue` / attribute `oldValue` need the
  pre-write value captured before the `EcsDom` write (the characterData case is the
  hard capture-*timing* constraint above). `attributeNamespace` stays deferred to
  `#11-mutation-observer-extras`.
- **Dual-runtime delivery.** Both VM and boa flush through `SessionCore`, so a
  seam-side record is runtime-uniform; a flush→MO hook must exist in the boa flush
  path until S5. A dispatcher-consumer (Pole A) is VM-only (dispatcher installed only
  by `Vm::bind`), a larger S5 coupling — a factor for B1.

### 4.4 Candidate directions B1 weighs (neither pre-decided here)

B0 enumerates the design space without picking. Two poles bound it; §4.1 already
showed **neither is correct as-is** — the answer is likely a structure separating
*where the write applies* from *where the MO record originates*:

- **Pole A — `MutationObserver` as a `ConsumerDispatcher` consumer.** *Satisfies*
  invariants 1 (synchronous, at the chokepoint) and 2 (rides the fan-out).
  *Tensions:* invariant 3 (puts a script-observable responsibility in the
  engine-internal layer); record shape (per-node events shatter the coalesced
  childList record); coverage holes (blind to the non-dispatching `value` write, the
  characterData-oldValue timing, and shadow-root-suppressed events).
- **Pole B — ScriptSession seam owns MO record production.** Every script-visible
  mutation records a `Mutation` (via `elidex-dom-api`/`DomApiHandler`, keeping
  `vm/host/` marshalling-only), MO drains at `flush`. *Satisfies* invariant 3 and
  produces coalesced shapes by construction, recording upstream of suppression.
  *Tensions:* invariant 1 (the naive "apply at flush" form defers the write — B1 must
  keep synchronous apply at the chokepoint while buffering only the *record*);
  invariant 2 / #181 (if the record's `apply_*` uses the buffered chokepoint-bypassing
  path, it loses fan-out); blast radius (every §1 write site must record); a **new
  flush→MO hook** (none exists today; must cover both flush paths, §2.2).

A satisfying mechanism plausibly **records the MO entry at the seam (invariant 3,
correct shapes) while keeping the synchronous write + dispatcher fan-out at the
chokepoint (invariants 1+2)**. Whether that, a flush-coalescing layer, or another
structure is correct — and how to thread it through the dom-api handlers, the
reflected setters (§4.5), the dual runtime, and the §4.2 constraints — is the B1
design judgment. B0 deliberately stops short of choosing.

### 4.5 B2 — bridge/direct + setAttribute/removeAttribute + reflected-setter convergence

§1 surfaced a uniformity gap B1's mechanism choice will shape: today `setAttribute`
routes through a `DomApiHandler` while `removeAttribute` and the reflected IDL setters
do not.

**The open B2 question is *where/how*, not *whether*.** A *true* reflected IDL setter
(`a.href`, `form.method`, `input.type`, …) **is** an observable attribute mutation:
per HTML §2.6.1 its contract is "set the content attribute" → WHATWG DOM §4.9 "handle
attribute changes" step 1 **queues an `"attributes"` record** (webref-verified). So a
`{attributes:true, attributeFilter:['href']}` observer **is owed** that record —
**spec-settled, not a per-plan option**. The **sole exception is `input.value`**
(8kHF): its setter is a value-mode dispatch, **not** "set the content attribute", so a
text-like-mode `input.value='x'` is a non-attribute live-state write that must **not**
emit a spurious attribute record. So B2 is **gated on B1**: the open question is which
layer/mechanism records the (spec-owed) reflected-write record, per §4.1's invariants.

**B2 convergence scope = the *whole* direct-`EcsDom` attribute-write host surface,**
not just `removeAttribute` + reflected setters: `{removeAttribute`, reflected IDL
setters, `toggleAttribute`, `Attr.value`, `setNamedItem`, `removeNamedItem`,
`setAttributeNode`, `removeAttributeNode}`. **Each has its own *conditional* write
path** (`Attr.value=` writes only when attached; `toggleAttribute` branches on
`force`/presence; `input.value` is value-mode), so B2 must confirm each API's *actual*
write path per-API — a B1/B2 grep-diff/dispatch-path derivation, not assumed uniform.

**VM-local Attr-detach precondition on the symmetry fix.** Routing `removeAttribute`
(and the other attribute-removing APIs) through the bridge is **not a pure dispatch
move**: `attr_remove` (`element_attrs.rs:180-187`) snapshot-freezes a JS-held `Attr`
wrapper's `detached_value` + `invalidate_attr_cache_entry` (so a later re-add cannot
make a previously-held `Attr` track the new value — matching Chrome/Firefox); the
bridge `RemoveAttribute` handler (`element/props.rs:108-122`) invalidates only the ECS
`AttrEntityCache`. So B2 must carry the VM-local Attr-wrapper detach forward after any
symmetry move (e.g. a VM-side post-step the handler signals), else held `Attr` objects
regress across removal/re-add. The Layering mandate (`vm/host/` marshalling-only)
applies throughout.

### 4.6 Sequencing

**B1's `/elidex-plan-review` resolves the §4.1 coupled-invariant corner and picks the
mechanism** (close the §3 MutationObserver gap + the §4.2 constraints + the
Range/normalize/Comment/live-range coupling), **before** B2 (the §4.5 convergence),
since B2's target shape depends on B1's choice. Both are `/elidex-plan-review`-gated
per `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須"; whether B2 is a
separate slice or the write-site half of B1 is itself a plan-review outcome.

---

## 5. Spec / Design SSoT Cross-Reference

- **WHATWG DOM §4.3** — MutationObserver; §4.3.2 "queue a mutation record"; §4.3.3
  Interface MutationRecord (record shape).
- **`docs/design/ja/12-dom-cssom.md`** — line 24: read-only `&EcsDom`, "書き込みは
  `session.record_mutation()`経由" (§12.1.1); line 47: "MutationObserver … セッション
  flushが…MutationRecordsを生成。ファーストクラス" (§12.1.2). This is the design
  *aspiration* B1 reconciles against the §4.1 invariants. B0 does **not** declare it
  satisfied or stale: §12 describes a seam-recorded MO path but does not by itself
  resolve how it coexists with synchronous read-your-writes (invariant 1) and the
  chokepoint fan-out (invariant 2 / #181) — that reconciliation is B1's plan-review.
- **`docs/design/ja/28-adr.md`** — ADR #17 (`ScriptSession` = unified Script↔ECS
  boundary, Mutation Buffer + "consistent MutationObserver records" in 単一メカニズム)
  — the **SSoT for the boundary's existence**: MO visibility belongs on the seam
  (invariant 3). It establishes *that* the seam owns MO records; it does **not**
  prescribe the *mechanism* by which every write reaches the seam while preserving
  invariants 1+2 — the §4 open question. ADR #14 ("MutationObserver がECS変更検出に
  自然にマッピング") describes the implementation substrate, not a license to put MO
  production in the `EcsDom` layer.
- **`CLAUDE.md`** — "ScriptSession as the sole Script↔ECS boundary … MutationObserver
  … を同一機構で守る" (invariant 3); **"Edge-dense work = multi-PR + 実装前
  plan-review 必須"** (the rule making the §4 mechanism choice a B1 judgment); "One
  issue, one way"; Layering mandate (`vm/host/` marshalling-only — §4.5).
- **lesson #181** (`attribute.rs:5-15`, `element/props.rs:61`) — the canonical
  `EcsDom::set_attribute` write-path consolidation, **in tension with naive seam-only
  routing** (§4.1 callout): the buffered `apply_set_attribute` bypasses the
  chokepoint, so routing writes through it re-forks what #181 collapsed and drops the
  `ConsumerDispatcher` fan-out (invariant 2). Keeping #181 intact while still
  producing a seam-side MO record (invariant 3) is the corner B1 resolves.

---

## 6. Re-check Discipline (for B1/B2 plan-memos)

- Re-grep every `file:line` here at PR-open — line numbers will drift.
- **Produce the exhaustive write-site set by grep-diff, not by extending §1's
  table.** Covered set = (a) every `record_mutation` call-site **whose flush reaches
  `deliver_mutation_records`** (per-frame `re_render` → `content/mod.rs:258`, **not**
  the CE-only `flush_with_ce_reactions`) ∪ (b) every **direct
  `deliver_mutation_records` producer** (representative: the VM innerHTML/outerHTML
  setters + `Element`/`ShadowRoot.setHTMLUnsafe` via `set_inner_html_for` —
  *representative, itself a grep-diff deliverable*). Then enumerate every direct
  `EcsDom`/component-mutator call across `vm/host/`, `elidex-dom-api`,
  `elidex-js-boa`, and `elidex-ecs`, and diff. Diffing against `record_mutation`
  alone false-positives the direct-delivery natives and false-negatives a
  record-but-only-CE-flushed site.
- Re-confirm the §2 mechanism by direct read of `attribute.rs`
  (`set_attribute`/`dispatch_event`), `tree/mutation.rs` (`Insert`/`Remove` fire
  sites + shadow-root suppression `return`), `consumer_dispatcher.rs` (consumer list),
  `mutation_observer.rs` (`deliver_mutation_records`), and the dispatcher-install
  asymmetry (only `Vm::bind` `vm_api.rs:279`, no boa install). This is what makes the
  §2.3 overlap a three-way split. Do not carry this reframe forward on trust.
- Re-confirm the two boa CE-reaction producers (flush-record scan
  `enqueue_ce_reactions_from_mutations` + binding-direct enqueue in
  `globals/element/core.rs`), so a record-production change does not double-enqueue or
  miss CE reactions.
- Re-confirm the §1.6 corners: `textContent=` childList replace-all
  (`text_content.rs:105-116`) vs `nodeValue=` non-CharacterData no-op (`:153-155`,
  mutation-free per DOM §4.4 `#dom-node-nodevalue` — do not gap-row); and the Comment
  `set_char_data` branch with no `dispatch_event` (`char_data_handlers.rs:59-73`).
- Re-confirm the §4 coupled-invariant anchors by direct read: `record_mutation`
  deferred-apply (`session.rs:78-90`); `apply_set_attribute` bypassing the chokepoint
  / no `dispatch_event` (`mutation/mod.rs:288-313` — invariant 2 / #181 tension);
  `apply_set_inner_html` single-record shape (`html_fragment.rs:85-89`); the missing
  flush→MO hook (`natives_promise.rs:333-344` dispatches slotchange only); the
  `input.value` value-mode dispatch (`html_input_value.rs:120-129` — 8kHF); the
  reflected-setter direct writes; the replace-all remove-all-then-insert loops
  (representative: `parentnode.rs:181-249` `replaceChildren` [parent-kind gate `:75` =
  Element/Document/DocumentFragment] + `text_content.rs:105-116` `textContent` — DOM
  §4.2.3 "replace all" `#concept-node-replace-all` **step 7 no-op guard**; exhaustive
  site list = B1 grep-diff); the class-level **move `Remove`** on a moved
  already-parented node (`detach_with_hook` → `fire_after_remove(child, old_parent, …)`,
  `tree/mutation.rs:458` — target = *old* parent, which is the **same** parent as the
  destination for a same-parent re-order [`old_parent == new_parent`, `:454-456` skips
  the redundant `rev_version`] and coalesces into the destination record, but a
  **different** parent for a cross-parent move where it is a separate uncoalesced record;
  `textContent`/`setHTMLUnsafe` string→fresh-node, NOT move-capable); the CE-reaction
  order anchors (`enqueue_ce_reactions_from_mutations` added-then-removed `ce.rs:145`
  + `replace_child` Remove-before-Insert `tree/mutation.rs:189`/`:201`); and the
  characterData `oldValue` capture-timing (`set_text_data` overwrite-before-dispatch
  `dom/mod.rs:336`/`:340-344`, `TextChange`/`ReplaceData` carry no old value). **Exact
  per-op sequences = B1 grep-diff, not this list.**
- Re-check active branches (`git branch -r`) for convergence drift on
  `element_attrs.rs` / `vm/host/` attribute setters (MED collision risk with JS-side
  work; B is later — Axis 5).
- Slot check: `#11-mutation-observer-extras` (attributeNamespace, primitive ToObject
  for `observe`) must still be open before referencing it.

## Review guidelines (for Codex)

- This is a **doc-only, B0-altitude** audit. Verify the `file:line` anchors against
  `main` and challenge any **mechanism claim or invariant** that does not match the
  code — especially §0/§3 (the MutationObserver gap, incl. the `input.value`
  non-reflection 8kHF and the Comment/Range/normalize/textContent/nodeValue/splitText
  rows) and §4 (the three coupled invariants + the §4.2 record-source invariants).
- **§1's write-site map and §4.2's record-source constraints are deliberately
  *invariants + representative examples + B1 grep-diff methodology*, NOT exhaustive
  hand registries or per-op/per-API/per-instance contracts** (review history showed
  per-instance characterization in a B0 audit is a finding generator — that detail is
  B1's plan-review grep-diff job). So do **not** flag for "missing site/op/API X" or
  "incomplete per-op event sequence" as a registry defect — instead check that (a) the
  governing invariant is correctly stated, and (b) any instance you find is consistent
  with it. A genuinely *mis-stated* invariant or an example that contradicts the code
  is still in scope.
- **§4 is deliberately *not* a prescribed fix** (edge-dense corner; `CLAUDE.md`
  reserves the mechanism choice for B1's `/elidex-plan-review`). Do **not** flag §4
  for "failing to pick a mechanism". **Do** flag if (a) any of the three §4.1 coupled
  invariants is mis-stated/mis-attributed, (b) a §4.2 record-source invariant is wrong,
  (c) the §4.4 Pole-A/Pole-B trade-offs mis-describe the code, or (d) the #181 /
  `apply_set_attribute`-bypass tension (§4.1 callout, §5) is mis-read.
- Out of scope: implementing B1/B2; touching `element_attrs.rs`, reflected IDL
  setters, `range_proto_mutation.rs`, `char_data` handlers, or `ConsumerDispatcher`.
