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

> **§4 status — open design question for B1, not a prescribed fix (this
> revision).** Across R1→R2→R3 the §4 "canonical path" recommendation drew an
> IMP finding three rounds running. That recurrence is itself the diagnosis:
> §4 sits on an **edge-dense coupled-invariant corner** — synchronous
> apply / read-your-writes × `ConsumerDispatcher` fan-out × ScriptSession-MO
> ownership × record-shape coalescing × dual-runtime (boa/VM) — i.e. ≥3
> intersecting invariant axes. Under `CLAUDE.md` "Edge-dense work = multi-PR +
> 実装前 plan-review 必須" + `feedback_coupled-invariant-design-corner`, a B0
> *audit* prescribing a single canonical mechanism here would be a mandate
> violation: that decision belongs to B1's `/elidex-plan-review`, with the
> coupled invariants mapped upfront. The original B0 charter framed §4 as
> "recommendation + trade-off, not a settled fix"; the R2 revision over-committed
> it to a prescriptive single mechanism (ScriptSession-seam-owned MO). **This
> revision withdraws that prescription.** §4 is re-framed as the *constraint set +
> coupled invariants* that B1 must satisfy, presented as B1's input — not as B0's
> answer. The earlier R2 claim that "the seam `apply_*` bottoms out at the lesson
> #181 chokepoint, so #181 is not in tension with seam-owned MO" was **wrong** and
> is corrected in §4/§5: the buffered `apply_set_attribute` does **not** call
> `EcsDom::set_attribute` — it duplicates only the reconcile fragment and fires no
> `MutationEvent` — so the buffered path *does* bypass the #181 chokepoint, and
> the #181-vs-MO-fan-out tension is real. That tension is part of the corner B1
> resolves.

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
> B1 plan-review corner — it bears on where any flush→MO-delivery hook must be
> wired and on the S5/boa-removal coupling of each candidate mechanism; see
> §0, §2, §4.

> **boa-runtime scoping note (read before any boa-specific path claim).** The
> **boa runtime is scheduled for removal in S5 / D-26 PR7** (the production shell
> runs it today, but the canonical Script↔ECS boundary is the elidex-js VM). Per
> `memory/feedback_boa-findings-light-touch`, this audit deliberately describes
> boa-specific mutation / CE-reaction / record-delivery paths only at a
> **known-to-differ** level — it does **not** exhaustively or precisely map them,
> because that map goes moot the moment boa is deleted. Earlier revisions
> (R4/R5/R6) over-invested in boa-specific precision (exact iframe-record /
> binding-direct-CE / fragment-record enumeration); that precision is **withdrawn
> here** and replaced by this scoping note. **The canonical MutationObserver
> design (§4, B1) targets the post-S5 VM runtime**; B1's `/elidex-plan-review`
> resolves the canonical path against the **VM** mechanism, and the exhaustive
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
> the original framing — §0/§3 quantify it. B0's job is to establish that factual
> map (§1–§3) and to *enumerate the coupled invariants* the canonical fix must
> satisfy (§4), **not** to pick the mechanism — that is B1's `/elidex-plan-review`
> (see the §4 status callout above). Every claim below carries a `file:line`
> anchor re-checked at HEAD `26d00c5a`.
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
    `EcsDom::set_attribute`, so they never fire a `MutationEvent` either. **This
    is invariant #2 below (8kG9):** the buffered path duplicates only
    `attrs.set` + `reconcile_attribute_derived_components` + `rev_version` and
    **does not fan out via `ConsumerDispatcher`** — so a write routed through it
    silently loses the base-url / form-control / event-handler / canvas / CE
    consumers that `EcsDom::set_attribute`'s `dispatch_event` drives. Any
    candidate "route all writes through `record_mutation`" mechanism must
    reckon with this loss (§4 invariant #2).
- **The real gap is the JS-level `MutationObserver`, and it is broader than the
  original F3 framing stated.** `MutationObserver` is *not* a `ConsumerDispatcher`
  consumer. It is fed by a `deliver_mutation_records` call — but **two distinct
  wirings** deliver, and B1 must not conflate them (corrected, 8ykO):
  - **VM-direct delivery** — the elidex-js VM `Vm::deliver_mutation_records`
    (`vm_api.rs:867`, single-arg `&[records]`) is called **synchronously inside
    two VM natives**: the innerHTML setter (`dom_inner_html.rs:148`) and the
    **outerHTML** setter (`dom_inner_html.rs:362` —
    `native_element_set_outer_html`, **not** insertAdjacentHTML). No `flush`, no
    shell, no session buffer — the native delivers its own record.
  - **boa per-frame delivery** — the shell's per-frame flush
    (`content/mod.rs:258` ← `re_render` ← `SessionCore::flush`) calls the **boa**
    `JsRuntime::deliver_mutation_records` (`elidex-js-boa/runtime/observers.rs:20`,
    four-arg: records + session + dom + document), **not** `Vm::…` — the shell
    imports `elidex_js_boa::JsRuntime` (`pipeline.rs:9`, `lib.rs:39`). This is
    the **only** flush-driven MO delivery, and it is on the **boa** runtime,
    distinct from the VM-direct path above. **B1 must therefore not look for a
    `Vm` flush-hook on the shell path** — the shell drives boa today; a
    seam→MO flush hook (8YcR) must be wired into the runtime the shell actually
    holds. The session buffer
  (`SessionCore::pending`) is populated in production by the
  `SetInnerHtml` / `InsertAdjacentHtml` `Mutation` variants
  (`elidex-dom-api/element/tree.rs:416`/`:476`) **and by the boa `<iframe>`
  attribute setters** (`elidex-js-boa/globals/iframe.rs:99`/`:105`/`:206` record
  `Mutation::SetAttribute`/`RemoveAttribute`). That iframe path is the **one
  existing attribute write that bypasses `EcsDom::set_attribute`** (it
  self-generates the record via `apply_set_attribute`/`apply_remove_attribute`,
  `mutation/mod.rs:288-332`, dropping the dispatcher fan-out — §2.2/§4.2a C4) and
  is therefore a write B1 must **reconcile**, not a clean precedent. With that one
  exception, every *other* JS DOM write — `setAttribute`, `removeAttribute`,
  every reflected IDL setter, **and
  `appendChild`/`removeChild`/`insertBefore`/`replaceChild` even through the
  bridge** — produces **no `MutationRecord`** and is therefore unobservable by
  `new MutationObserver(...)` (correcting an earlier "every other JS DOM write is
  silent" over-claim that omitted the boa iframe producer — 8ykE). Note the shell's *initial-script /
  finalization* flush (`pipeline.rs:25-34` `flush_with_ce_reactions`) feeds
  flush records to **CE reactions only** and does **not** call
  `deliver_mutation_records`, so even innerHTML mutations done during page
  load are not delivered to MO via that path — only the per-frame
  `content/mod.rs:258` site delivers (§2.2).
- **There are two mechanisms, and they answer two *different* questions** — but
  the consumer/mechanism map is **not** the clean "dispatcher = non-observable /
  seam = MutationObserver" dichotomy an earlier draft drew (corrected, 8ykJ/8ykQ).
  They are: (1) `EcsDom`'s `ConsumerDispatcher` — **mostly engine-internal
  derived-state reconciliation**, synchronous at the chokepoint, driving 7
  consumers (live-range / node-iterator / base-url / form-control /
  event-handler-attr / canvas / custom-element). **Six are derived-state
  reconcilers** (several feeding script-readable state); the seventh,
  `CustomElementReactionConsumer`, is **script-visible** — it enqueues
  `connected`/`disconnected`/`attributeChangedCallback` reactions drained by
  `flush_ce_reactions` (§2.1), firing user JS. So the dispatcher is **not** purely
  non-observable: `MutationObserver` is not among its consumers, but **CE
  reactions are**. (2) `SessionCore`'s mutation buffer + `flush` →
  `deliver_mutation_records` — the path that feeds `MutationObserver` (fed today
  only by innerHTML-class ops) **and also feeds CE reactions**: the shell drains
  flush records into `enqueue_ce_reactions_from_mutations`
  (`pipeline.rs:29-34` `flush_with_ce_reactions`, `lib.rs:618-628` per-frame),
  so CE reactions are driven from **both** mechanisms (8ykQ). They overlap at
  innerHTML (both mechanisms) and at CE reactions (both mechanisms). Read as a
  *factual map*: the dispatcher is a *mostly-reconcile + CE-tap* mechanism, the
  seam is the *script-visibility + flush-side-CE* mechanism — and the gap is that
  script-visible **MutationObserver** mutations do not all reach the seam. **How
  to close that gap while keeping the dispatcher fan-out (invariant 2) and
  synchronous read-your-writes (invariant 1) is the coupled design question §4
  hands to B1** — not settled here as "one mechanism wins".
- **Dual-runtime risk is broader than "MO is fine because it is separate"
  (8ykJ).** In production (boa) the `ConsumerDispatcher` is **not installed** at
  all (§2.1). It is **not** harmless to say "those consumers are not
  script-observable, so fine": the dispatcher's CE-reaction enqueue *is*
  script-visible, so a boa runtime that lacks the dispatcher lacks that
  script-visible effect *via the dispatcher path* — boa instead drives CE
  reactions through a **separate** wiring (`pipeline.rs:29-34` /
  `ce.rs:137-145`), and B1 must show that wiring covers the same reactions the VM
  dispatcher would. The script-visibility gap is **not** confined to Mechanism B;
  any candidate that drops dispatcher fan-out must independently re-establish the
  CE-reaction (and base-url / form-control / event-handler / canvas) effects, not
  just the MutationObserver records.
- **Canonical-path decision is deferred to B1's `/elidex-plan-review` (§4).**
  B0 does **not** prescribe the mechanism. §4 sits on an edge-dense
  coupled-invariant corner (≥3 intersecting axes — see the §4 status callout),
  and `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須" reserves
  that design judgment for B1. What B0 *does* fix is the **constraint set** B1
  must satisfy. The hardest three, which any candidate mechanism must reconcile
  *simultaneously*, are coupled:
  1. **Synchronous apply / read-your-writes.** `el.setAttribute('x','1');
     el.getAttribute('x')` must observe `'1'` without a flush, but
     `record_mutation` (`session.rs:78-90`) only *buffers* and applies at flush —
     deferred. So DOM-write *application* and MO-record *buffering* cannot be the
     same deferred step; they must be separated.
  2. **`ConsumerDispatcher` fan-out preservation.** A buffered
     `apply_set_attribute` (`mutation/mod.rs:288-313`) writes `Attributes`
     directly + `reconcile_attribute_derived_components` + `rev_version` and
     **does not call `dispatch_event`** ("instead of entering
     `EcsDom::set_attribute`", `:299-300`). Routing every write through that
     buffered path would **lose** the base-url / form-control / event-handler /
     canvas / CE consumer fan-out that `EcsDom::set_attribute` provides.
  3. **ScriptSession mandate.** MutationObserver visibility is, per `CLAUDE.md`
     "ScriptSession as the sole Script↔ECS boundary … session mutation と flush
     point に集約 … MutationObserver … を同一機構で守る", a seam-and-flush
     responsibility.
  These pull in different directions: "route all writes through `record_mutation`"
  breaks invariants 1 and 2; "make MutationObserver a `ConsumerDispatcher`
  consumer" breaks invariant 3 (and shatters innerHTML's single coalesced
  childList record, 8YcO). **No single naive routing satisfies all three** — that
  is exactly why the choice is a B1 plan-review design judgment, not a B0 verdict.
  §4 presents these three invariants plus the C1–C8 + non-dispatching /
  shadow-root / Range / `normalize` / CommentData record-source constraints as
  B1's *input*. (Design §12's "writes via `session.record_mutation`" and ADR #17's
  "consistent MutationObserver records in a single mechanism" are the design
  *aspiration* B1 reconciles against these invariants; B0 does not declare them
  satisfied or stale.)

---

## 1. VM `vm/host/` DOM Write-Site Map

> **Write-site invariant + B1 methodology (read first — this map is
> representative, not a hand-maintained registry).** The R1→R5 review history
> (R1 Range/normalize, R4 CE/outerHTML, R5 boa-CE/textContent/splitText) showed
> that *hand-enumerating* write sites loses sites round after round. So this
> section is framed as **invariant + representative known-set + B1 methodology**,
> not an open-ended reactive list:
> - **Invariant (the load-bearing statement — corrected, 9LCP).** *Any
>   script-reachable mutation that reaches an `EcsDom` / component mutator
>   (`set_attribute` / `remove_attribute` / `append_child` / `remove_child` /
>   `insert_before` / `replace_child` / `set_text_data` / `Attributes::set` /
>   `CommentData` direct write, etc.) **without** going through
>   `SessionCore::record_mutation` is **MutationObserver-silent — EXCEPT a
>   direct-delivery path that calls `Vm::deliver_mutation_records` with a
>   self-generated record.*** The VM `innerHTML`/`outerHTML` setters are exactly
>   that exception: they call `apply_set_inner_html`/`apply_set_outer_html` via
>   `with_session_and_dom` (`dom_inner_html.rs:146`/`:359`) — bypassing
>   `SessionCore::record_mutation` — and then **synchronously deliver** the
>   returned record with `ctx.vm.deliver_mutation_records(&[rec])`
>   (`dom_inner_html.rs:148`/`:362`). So they are **observable** without going
>   through `record_mutation`. The precise invariant is therefore: *a mutator is
>   MutationObserver-silent iff it neither goes through `record_mutation` (→ flush
>   → deliver) **nor** explicitly drives `deliver_mutation_records` with its own
>   record.* This is the single property that defines the §3 gap; it holds
>   regardless of whether the site is enumerated below. (The `EcsDom` chokepoint
>   still drives Mechanism A — §2.1 — but Mechanism A is not a `MutationObserver`
>   source; §2/§3.)
> - **The tables below are a *representative known-set*, not exhaustive.** They
>   pin the sites confirmed by direct read (incl. the R5 additions in §1.6), but a
>   site's *absence* here does **not** mean it records a `Mutation` — apply the
>   invariant.
> - **Verified-exhaustive enumeration is a B1 plan-review deliverable.** B1 must
>   produce the exhaustive write-site list by **grep-diff**, not by extending this
>   table reactively. The *covered* side of the diff is **not** just
>   `record_mutation` call-sites — it is the **union** of (a) every
>   `SessionCore::record_mutation` call-site **whose flush actually reaches
>   `deliver_mutation_records`** **and** (b) every **direct
>   `deliver_mutation_records` producer** (a mutator that self-generates a record
>   and delivers it synchronously, e.g. the VM `innerHTML`/`outerHTML` setters at
>   `dom_inner_html.rs:148`/`:362` — see the corrected invariant above).
>   **The flush-reaches-delivery qualifier on (a) is load-bearing, not pedantry:**
>   `record_mutation` only *buffers*; whether that buffer becomes an MO record
>   depends on *which flush* drains it. The per-frame `re_render` flush delivers
>   (`content/mod.rs:258`), but `flush_with_ce_reactions`
>   (`crates/shell/elidex-shell/src/pipeline.rs:25-34`) flushes the buffer into
>   **CE reactions only** and never calls `deliver_mutation_records` (§2.2 / R4
>   8ykQ/8ykJ). So a `record_mutation` call whose buffer is only ever drained by
>   `flush_with_ce_reactions` is **MO-silent despite recording** — (a) covers a
>   call-site only when its flush path is the delivering one. **The grep-diff must
>   therefore enumerate flush/delivery sites alongside recording call-sites and
>   judge "covered" by recording-call ∧ delivering-flush, not by the recording call
>   alone.** Enumerate that union, then enumerate every direct
>   `EcsDom`/component-mutator call across the four layers —
>   `crates/script/elidex-js/src/vm/host/`, `crates/dom/elidex-dom-api/`,
>   `crates/script/elidex-js-boa/`, and the `elidex-ecs` mutators themselves — and
>   take the difference (mutators in neither covered set = the gap set). **Diffing
>   against `record_mutation` alone would false-positive-flag VM
>   `innerHTML`/`outerHTML` as a gap** even though they are observable via the
>   direct-delivery path — and would **false-negative** a record-but-never-delivered
>   site. That mechanical sweep — covered = (`record_mutation` ∧ delivering flush)
>   ∪ direct `deliver_mutation_records` producers — not this hand list, is the SoT
>   for completeness.

Seeded from the original F3 write-site survey and **re-verified at HEAD
`26d00c5a`** (the seed is not load-bearing — every row is re-checked by direct
read). Each site is
classified and tagged **bridge** (routes through
`dom_bridge::invoke_dom_api` → an `elidex-dom-api` `DomApiHandler`) or
**direct** (calls `EcsDom::*` straight from `vm/host/`). Note: "bridge" here
means *dispatch* routing — it does **not** imply the session buffer; see §1.5.

### 1.1 Attribute API (`element_attrs.rs` + Attr/NamedNodeMap)

| Site | Method | Path | Notes |
|---|---|---|---|
| `element_attrs.rs:106` (`attr_set`) | helper for reflected setters & toggleAttribute | **direct** → `EcsDom::set_attribute` (`:112`) | thin shim, marshalling-only |
| `element_attrs.rs:155` (`attr_remove`) | helper for `removeAttribute` etc. | **direct** → `EcsDom::remove_attribute` (`:177`) | **does VM-local work the bridge handler does not**: snapshot-freezes any JS-held `Attr` wrapper's `detached_value` + `invalidate_attr_cache_entry` (`element_attrs.rs:180-187`); the bridge `RemoveAttribute` handler (`element/props.rs:108-122`) invalidates only the ECS `AttrEntityCache`, no VM-local detach. See §4.5 (B2 constraint). |
| `element_attrs.rs:205` (`native_element_set_attribute`) | `Element.setAttribute` | **bridge** → `invoke_dom_api("setAttribute", …)` (`:218`) | dom-api `SetAttribute` handler bottoms out at `EcsDom::set_attribute` (props.rs:70) |
| `element_attrs.rs:191` (`native_element_get_attribute`) | `Element.getAttribute` | **bridge** → `invoke_dom_api("getAttribute", …)` (`:202`) | read |
| `element_attrs.rs:226` (`native_element_remove_attribute`) | `Element.removeAttribute` | **direct** → `attr_remove` (`:235`) | **asymmetry: a `"removeAttribute"` handler is registered, but the VM bypasses it** (see §3) |
| `attr_proto.rs:416` | `Attr.value =` setter | **direct** → `EcsDom::set_attribute` | reflected-attr value write |
| `named_node_map.rs:345`/`:431` | `NamedNodeMap.setNamedItem`/`removeNamedItem` | **direct** → `EcsDom::set_attribute`/`remove_attribute` | |
| `element_attrs.rs:414`/`:535` | `setAttributeNode`/`removeAttributeNode` | **direct** → `EcsDom::set_attribute`/`remove_attribute` | |

### 1.2 Reflected IDL setters (direct `EcsDom::set_attribute` / `remove_attribute`)

All **direct**. These are content-attribute reflections (HTML §2.6.1
"Reflecting content attributes in IDL attributes",
`#reflecting-content-attributes-in-idl-attributes`, verified via webref at
HEAD `26d00c5a`); each writes the backing content attribute through the
`EcsDom` chokepoint. Per that section the reflected IDL setter's contract is
"set the content attribute" (set one content attribute via the
`EcsDom::set_attribute` chokepoint) — which is why these are pure
content-attribute reflections. The `HTMLInputElement.value` exception (8kHF,
§1.2 caveat below) does **not** keep this reflection contract: its setter is a
value-mode dispatch, not "set the content attribute", so in text-like mode it
writes `FormControlState` live value and the content attribute is untouched.

- `html_input_proto.rs:460`/`:544`/`:687`/`:853`; `html_input_value.rs:129`/`:182`/`:253`/`:501`/`:535`
  - **Caveat (8kHF):** `html_input_value.rs:129` is **not** an unconditional
    reflection — it is the `SetContentAttr` arm of the `HTMLInputElement.value`
    value-mode dispatch (`html_input_value.rs:120-129`), reached **only** in
    default/default-on value mode. In text-like mode the same setter takes
    `ValueSetAction::SetLiveValue` and writes `FormControlState` live value
    (`Attributes` untouched, no `set_attribute`). So `input.value` is a
    live-state write in the common case, not a content-attribute mutation; do not
    treat it as a plain reflected setter (§3 gap table, 8kHF).
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
> the `setAttribute` API uses — a uniformity gap B2 should weigh (§4.5).

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
`deleteContents`/`extractContents`, **characterData**) mutations that must become
MutationObserver-visible under whichever mechanism §4 settles on. Today they
produce **no `MutationRecord`** at all — each clones the registered `Range`, runs
the engine-indep mutation through `host.dom()` (a raw `&mut EcsDom`), and commits
boundary state back to the live-range registry. **No `record_mutation`.**

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
> `26d00c5a`). So a `data=`/`appendData`/… on a **Comment** node fires **no**
> mutation event at all, and `observe(comment,{characterData:true})` is silent
> even on the dispatcher path. A handler/seam-emitted `record_mutation` (Pole B,
> §4.2) would close this uniformly — independent of whether the EcsDom primitive
> dispatches — but the mechanism is B1's to choose; either way B1 must call this
> out explicitly: Comment
> characterData currently has **no** notification of either kind, whereas Text has
> `TextChange`. §3's characterData row must split Text vs Comment.
>
> **Coupled invariant for B1 (9WUB).** Comment implements CharacterData, and the
> canonical fix is more than "emit the missing record": WHATWG DOM §4.10 Interface
> CharacterData "replace data" (`#concept-cd-replace`) **queues the characterData
> record (step 4) *and* adjusts live ranges whose boundary points fall in the
> spliced node (steps 8–11**, over §5.5 Interface Range's "live range",
> `#concept-live-range`). The current Comment path adjusts **neither** (no event →
> `LiveRangeBridge` never runs; no record). So B1 must keep MO-record production
> and live-range boundary adjustment **coupled at one source** for all
> character-data splices — see §4.3's "CommentData notification + live-range
> coupling" bullet, where this is named as a §4 coupled invariant.

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

### 1.6 Additional representative no-record sites (R5 grep-diff catches)

These three are added to the representative known-set because the R5 review found
them missing — each is a script-reachable mutation that hits an `EcsDom` /
component mutator with **no `record_mutation`**, so it is MutationObserver-silent
per the §1 invariant. They are *examples* the B1 grep-diff must also surface, not
a new exhaustive boundary.

- **`textContent` / `nodeValue` setters (886F)** —
  `node_methods/text_content.rs` `SetTextContentNodeKind` /
  `SetNodeValue`: Text/CData branch → `dom.set_text_data` (`:93`/`:145`),
  Comment branch → `CommentData.0` direct write + `rev_version`
  (`:99-102`/`:148-151`), element branch → `remove_child` + `append_child`
  (`:107-113`). **None calls `record_mutation`** — so `el.textContent = 'x'` and
  `text.nodeValue = 'x'` are MO-silent (§3 gap rows).
- **`Text.prototype.splitText` (886H)** — VM
  `vm/host/text_proto.rs:119` (`native_text_split_text`) calls
  `split_text_at_offset` (`elidex-dom-api/char_data/split_text.rs:99`), which
  inserts the new sibling (`append_child`/`insert_before`, firing `Insert`),
  fires `fire_split_text` + the `set_text_data` `TextChange` (`:171`/`:179`), but
  **records no `Mutation`**. So `text.splitText(n)` is MO-silent under
  `observe(parent,{childList,subtree,characterData})` (§3 gap row).

These confirm the invariant's reach across `node_methods/` and `char_data/`, both
outside the §1.1–§1.4b tables — exactly the kind of site hand-enumeration kept
missing.

---

## 2. The Two Notification Mechanisms

There are exactly two. **Read them as answering two different questions, not as
two competing canonical write paths.** The detail sections own the precise
characterization (this intro does not restate it, to avoid the summary drift the
R3/R4/R5 review flagged): §2.1 establishes that Mechanism A is *mostly*
engine-internal reconcile **plus a script-visible CE-reaction tap** (so its
consumers are **not** all non-observable), and §2.2 establishes that Mechanism B
(the `SessionCore` buffer + flush) drives **both** `MutationObserver` *and* CE
reactions. The §4 canonical-path decision — how a script-visible mutation reaches
MutationObserver while preserving the §4.1 coupled invariants — is **deferred to
B1's `/elidex-plan-review`** (§4); §2 does **not** prescribe a mechanism, and in
particular does **not** assert "route every write into Mechanism B".

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint) — **engine-internal reconcile, VM-only today**

> **Scope (mandate-relevant) — corrected (8ykJ).** An earlier draft called all
> 7 consumers "engine-internal derived-state reconciliation" and "**none is
> script-observable**". That dichotomy was **too clean**. The consumers split:
> - **Derived-state reconcilers** (live-range adjustment, NodeIterator
>   pre-removal, `<base href>` resolution, form-control state,
>   event-handler-attr compilation, canvas reset) feed engine-internal derived
>   ECS components; these are not *directly* a `MutationObserver` source, but
>   several drive **script-visible** state (Range boundary points,
>   NodeIterator reference node, compiled `onclick` listeners, canvas bitmap,
>   form-control value) that JS can read back — so "non-observable" overstated it.
> - **`CustomElementReactionConsumer`** (`consumer_dispatcher.rs:84`, doc block
>   `:75-84`) is **directly script-visible**: on the matching
>   `MutationEvent::Insert`/`Remove`/`AttributeChange` it **enqueues**
>   `connectedCallback` / `disconnectedCallback` / `attributeChangedCallback`
>   reactions, drained by `VmInner::flush_ce_reactions`
>   (`vm/host/custom_elements/flush.rs:40`, called from `interpreter.rs:54` /
>   `natives_timer.rs:281`) — i.e. it **fires user JS lifecycle callbacks**. So
>   the dispatcher is **not** a purely engine-internal mechanism: CE reactions
>   are a *second* script-visible consumer alongside `MutationObserver`. The
>   accurate statement is: `MutationObserver` is *not* a dispatcher consumer, but
>   CE-reaction enqueue *is* — and it is script-visible.
> So this mechanism is **mostly** EcsDom-internal reconcile **plus a
> script-visible CE-reaction tap**; whether `MutationObserver` records should be
> derived from its events (Pole A) or emitted at the ScriptSession seam (Pole B)
> is the §4 open question. Note the mandate (invariant 3) names MO as a seam
> responsibility — a factor against Pole A — but invariants 1+2 cut the other
> way; B1 weighs them. **Dual-runtime consequence (§0/§2):** because CE reaction
> *enqueue* is a dispatcher consumer, a runtime that lacks the dispatcher
> fan-out lacks this script-visible effect *via this path* — see the boa caveat
> in §2.1's Plumbing bullet (boa drives CE reactions through a separate wiring,
> `pipeline.rs:29-34` / `ce.rs:137-145`, which must be shown to cover the same
> reactions).

- **Trigger:** the `MutationEvent` enum
  (`crates/core/elidex-ecs/src/dom/mutation_event.rs`) carries **seven** variants,
  all dispatched through the same `dispatch_event` sink:
  - `AttributeChange` (`:306`) — every `EcsDom::set_attribute` / `remove_attribute`
    (via `dispatch_event`, `attribute.rs:118`/`:294`).
  - `Insert` / `Remove` (`:131`/`:172`) — every tree mutation
    (`tree/mutation.rs:292`/`:356`).
  - **Character-data variants** (the input list B1 needs to evaluate
    `data=`/`appendData`/`splitText`/`normalize` coverage): `TextChange`
    (`:191`) — fired AFTER a Text/CData entity's `set_text_data`
    (`dom/mod.rs:340-344`); `ReplaceData` (`:209`) — fired AFTER
    `appendData`/`insertData`/`deleteData`/`replaceData` on Text/CData;
    `SplitText` (`:237`) — fired by `EcsDom::fire_split_text` AFTER
    `Text.splitText`; `NormalizeMerge` (`:273`) — fired by
    `EcsDom::fire_normalize_merge` during `Node.normalize`.
  Note these character-data events are **Text/CData-only**: the **Comment** branch
  of `set_char_data` (`char_data/char_data_handlers.rs:59-73`) and the Comment
  branches of `textContent`/`nodeValue` fire **no** `MutationEvent` at all (§1.4b
  8YcL), so `data=`/`appendData`/… on a Comment is silent on the dispatcher path
  too — a coverage hole B1 must read off this list.
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
  (Consequently an event-driven MO source — Pole A, §4.2 — would also miss
  shadow-root mutations, whereas a record emitted upstream of the dispatcher would
  not; this is constraint C3, §4.2a — a factor B1 weighs, not a settled verdict.)
- **Note: `MutationObserver` is NOT a consumer here today.** Whether it should
  become one (Pole A) or be fed at the ScriptSession seam (Pole B) is the §4 open
  question; the mandate (invariant 3) leans toward the seam, invariants 1+2 toward
  the chokepoint.

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
  constraint). (The boa iframe path is **boa-specific** — per the scoping note,
  it is known-to-differ, not a precise model B1 converges onto; it goes moot at
  S5. It is retained here only because it is the one *existing* example of an
  attribute write recording to Mechanism B without a `MutationEvent`, which §4
  weighs as a constraint, not as a contract.) **No elidex-js VM attribute or tree
  native records a `Mutation`.**
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
- **Consumers driven — `MutationObserver` *and* CE reactions (corrected, 8ykQ).**
  An earlier draft said "MutationObserver only"; the flush records actually feed
  **two** script-visible consumers:
  1. **`MutationObserver`** — `Vm::deliver_mutation_records` (`vm_api.rs:867`) →
     `VmInner::deliver_mutation_records` (`mutation_observer.rs:418`) →
     `MutationObserverRegistry::notify` (per-record inclusive-ancestor walk over
     `MutationObservedBy`, DOM §4.3.2) → observer callbacks. (In production/boa,
     the boa `JsRuntime::deliver_mutation_records`, `runtime/observers.rs:20`,
     called from `content/mod.rs:258` — see §0/8ykO for the VM-vs-boa split.)
  2. **Custom-element reactions** — the shell hands the same flush records to
     `enqueue_ce_reactions_from_mutations` (`elidex-js-boa/runtime/ce.rs:137-145`,
     a CE-lifecycle source) before observer delivery:
     `flush_with_ce_reactions` (`pipeline.rs:29-34`) and the per-frame `re_render`
     (`lib.rs:618-628`) both do `session.flush(dom)` →
     `enqueue_ce_reactions_from_mutations(&records, dom)` →
     `drain_custom_element_reactions_public(...)`. So **Mechanism B is not
     MutationObserver-only** — buffered records also drive CE
     `connected`/`disconnected`/`attributeChangedCallback`. Consequently CE
     reactions are driven from **both** mechanisms (dispatcher tap §2.1 *and*
     flush-side here), and **B1's record production/delivery changes must preserve
     CE-reaction semantics**, not treat the session buffer as MO-only (invariant
     for §4.x / 8ykQ).
     - **boa CE-reaction sources are *two* systems, not one (886B / 9LCX) —
       boa-specific, see scoping note.** Per the boa-runtime scoping note (top of
       doc), this is described at *known-to-differ* level only; boa is removed in
       S5/D-26 PR7, so its precise CE-producer set is not load-bearing for B1
       (whose canonical design targets the VM). At that level: besides the
       flush-record scan (`enqueue_ce_reactions_from_mutations`, above), boa also
       enqueues CE reactions **directly from the JS binding**
       (`elidex-js-boa/globals/element/core.rs`) — `appendChild`/`removeChild`
       call `enqueue_ce_reactions_for_subtree` (`:152-176`, def `:323`) for
       connected/disconnected, and `setAttribute`/`removeAttribute` enqueue
       `CustomElementReaction::AttributeChanged` (`:219`/`:292`). **boa registers
       only `appendChild` and `removeChild`** (`core.rs:106`/`:116`); the
       `"insertBefore"` arm in `dom_child_operation`'s match (`:152`) is
       **unreachable** because no `insertBefore` method is bound, so `insertBefore`
       is **not** a binding-direct CE producer today (a prior revision wrongly
       listed it — if boa later exposed `insertBefore` it would need wiring, but
       that is moot post-S5). The B1-relevant takeaway is just the *shape*: a CE
       reaction can originate from either the flush-record scan **or** a
       binding-direct enqueue, so a record-production change must not
       double-enqueue or miss CE reactions. The exact boa producer enumeration is
       out of B1's scope (scoping note).
- **There is NO existing flush→MO microtask drain hook (8YcR).** The
  `Microtask::NotifyMutationObservers` enum variant
  (`natives_promise.rs:51-59`) exists, but its drain arm (`:333-344`) dispatches
  **only the `slotchange` half** (`dispatch_pending_slotchange_signals`,
  `:342`); the `MutationObserver`-callback half is **not** wired there — it is
  embedder-driven via `Vm::deliver_mutation_records`, which only the per-frame
  `content/mod.rs:258` site calls. So **any** seam-fed mechanism (Pole B, §4.2)
  needs **new** wiring: `session.flush` (which *does* exist and returns the
  records) would have to drive MO delivery — e.g. by extending
  `NotifyMutationObservers`'s drain to deliver buffered records at the §4.3.2
  microtask checkpoint, or by a flush-tail delivery call. The drain *point*
  (`flush`) exists; the *hook* from flush to MO does not. Which hook (and whether
  Pole B is taken at all) is B1's to decide.

### 2.3 Overlap

The two mechanisms intersect at the **HTML-fragment write family —
innerHTML, outerHTML, and insertAdjacentHTML** (corrected, 8ykL; the earlier
"only innerHTML/insertAdjacentHTML" omitted the VM `outerHTML` setter):
`apply_mutation(SetInnerHtml)` / `apply_set_outer_html` ultimately drive `EcsDom`
tree ops (Mechanism A consumers fire) *and* yield a `MutationRecord` that reaches
the observer.

> **The VM innerHTML/outerHTML record path is *direct-delivery*, NOT Mechanism B
> buffer-overlap (corrected, 9dTQ).** An earlier draft classified the VM
> `innerHTML`/`outerHTML` record path as "Mechanism B → observer" — i.e. as if the
> record were buffered into `SessionCore::pending` and delivered at `flush`. **It is
> not.** The VM natives call `apply_set_inner_html` / `apply_set_outer_html`
> *directly* through `with_session_and_dom` with the `_session` argument **unused**
> (`dom_inner_html.rs:146`/`:359`), and then **synchronously hand the returned
> record to `ctx.vm.deliver_mutation_records(&[rec])`** (`dom_inner_html.rs:148`/
> `:362`) — never entering `SessionCore::record_mutation` and never going through
> `flush`. So the correct map of the §1-invariant "covered" set has **three**
> categories, not two: Mechanism A (dispatcher events), Mechanism B (`SessionCore`
> buffer → `flush` → `deliver_mutation_records`, the production/boa MO path and the
> boa iframe attr writes, §2.2), and a **VM direct-delivery** category (the VM
> innerHTML/outerHTML setters self-generate a record and deliver it synchronously,
> bypassing the buffer). The overlap at the HTML-fragment family is therefore
> **Mechanism A (tree-op dispatcher events fire) ∩ VM direct-delivery** in the VM,
> and Mechanism A ∩ Mechanism B on the boa/dom-api `SetInnerHtml`/`InsertAdjacentHtml`
> path — *not* a single uniform "Mechanism B" overlap. This is the same
> three-way split §0/§1's covered-set definition (covered = `record_mutation` ∧
> delivering flush **∪** direct `deliver_mutation_records` producers) and §6's
> grep-diff methodology already use; §2.3 is corrected here to match it.
> `insertAdjacentHTML` is **not** a VM native (`well_known.rs:341-342` installs only
> `insertAdjacentElement`/`insertAdjacentText`), so its `SetInnerHtml`-class record
> is a dom-api/boa Mechanism-B producer only, not a VM direct-delivery one (§1.4).

The VM `outerHTML` setter is concretely both Mechanism A and VM direct-delivery:
`native_element_set_outer_html` → `apply_set_outer_html` (`html_fragment.rs:116`)
runs the replace through `EcsDom` tree ops (Mechanism A `Insert`/`Remove` fire per
node) **and** emits the coalesced record delivered synchronously at
`dom_inner_html.rs:362` (VM direct-delivery, **not** buffered Mechanism B) — so
outerHTML is on the overlap with innerHTML, consistent with §3's "records ARE
produced" list and §4.3's no-double-delivery / C5 coalescing caveat.

For most other writes, Mechanism A fires and Mechanism B is empty — but **not
all** (9LCT). The blanket "every non-fragment write fires Mechanism A" is
**false** for **two** non-fragment write classes, each of which drives a path
*other than* Mechanism A:

1. **Comment character-data — drives *neither* mechanism (9LCT/8YcL).**
   `set_char_data`'s Comment branch (`char_data/char_data_handlers.rs:59-73`)
   writes `CommentData.0` + bumps `rev_version` **only** — it calls no
   `dispatch_event`, so **neither** a Mechanism-A `MutationEvent` **nor** a
   Mechanism-B record fires (§1.4b/§3 8YcL). So a `comment.data = 'x'` (or
   `appendData`/… on a Comment) drives **neither** mechanism. (The Text/CData
   branch *does* fire `TextChange` via Mechanism A; only the Comment branch is the
   hole.)
2. **boa buffered `<iframe>` attribute writes — Mechanism B, *not* Mechanism A
   (9dTR).** `iframe.rs:99`/`:105`/`:206` record
   `Mutation::SetAttribute`/`RemoveAttribute`, applied by
   `apply_set_attribute`/`apply_remove_attribute` (`mutation/mod.rs:288-332`) which
   write `Attributes` directly + reconcile + `rev_version` and **self-generate** the
   record — they never enter `EcsDom::set_attribute` and so fire **no**
   `MutationEvent`. So this is a non-fragment write that records into Mechanism B
   while driving **no** Mechanism-A event (and bypassing the `EcsDom::set_attribute`
   chokepoint / `MutationEvent`, §2.2/§4.2a C4). (boa-specific — known-to-differ per
   the scoping note, moot at S5; retained because it is the one *existing*
   non-fragment attribute write that records to Mechanism B without a
   `MutationEvent`.)

So the "all non-fragment writes fire Mechanism A" statement has **two**
counterexamples, not one: Comment character-data (neither mechanism) and the boa
buffered iframe attr write (Mechanism B, no Mechanism-A event). With those two
exceptions noted, for all remaining writes Mechanism A fires and Mechanism B is
empty.

---

## 3. The MutationObserver Coverage Gap (exact)

The JS-level `MutationObserver` (WHATWG DOM §4.3) observes a mutation **iff a
`MutationRecord` is produced *and a delivering path actually hands it to*
`deliver_mutation_records`** (corrected, 9dTT — "produced and delivered", with
*delivered* spelled out below). Concretely, the mutation must be in **one** of:
**(a)** it went through `SessionCore::record_mutation` **and** the flush that
drains its buffer is the *delivering* flush — the per-frame `re_render` →
`content/mod.rs:258`, **not** `flush_with_ce_reactions` (`pipeline.rs:25-34`), which
feeds the buffered records to **CE reactions only** and **never calls
`deliver_mutation_records`**; **or (b)** it is a **direct-delivery** producer that
self-generates a record and calls `deliver_mutation_records` synchronously without
the buffer (the VM `innerHTML`/`outerHTML` setters, `dom_inner_html.rs:148`/`:362`).
**`record_mutation` is therefore NOT, by itself, equivalent to observation:** a
write that records a `Mutation` whose buffer is only ever drained by
`flush_with_ce_reactions` (page-load / lifecycle finalization) is **MO-silent
despite recording** — it is delivered to CE reactions but not to MutationObservers
(§1 covered-set qualifier / §2.2 / §6 grep-diff). This is the §1 invariant
("covered = (`record_mutation` ∧ delivering flush) ∪ direct
`deliver_mutation_records` producers") restated for the observer axis — **not**
"recorded ⇒ observed". The gap rows below are a **representative** list (incl. the
R5 textContent/nodeValue/splitText additions), and the **verified-exhaustive** gap
set is the §1 B1 grep-diff deliverable, not this table.

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

> **boa `DOMParser`/`outerHTML` are NOT counted as record-producing coverage
> (retracted, 9LCV — boa-specific, see scoping note).** A prior revision listed
> "`DOMParser`/fragment innerHTML / outerHTML on the boa path" as record-producing
> coverage. That is **withdrawn**: per the boa-runtime scoping note (top of doc),
> boa-specific delivery is described only at known-to-differ level and is moot
> post-S5. At that level the boa fragment/outerHTML paths do **not** reliably
> surface a `MutationRecord` to MO — the boa binding's own `session.flush` can
> discard the fragment record (only the shell's per-frame `re_render` records are
> delivered), and a boa `outerHTML` replace bottoms out in direct
> `insert_before`/`remove_child` with no record. So they are **not** a clean
> "records ARE produced" precedent; they live under the scoping note as
> known-to-differ, not as B1 coverage to converge onto. B1's canonical design
> targets the VM, where innerHTML/outerHTML records *are* produced and delivered
> (the two bullets above).

**NO record is produced for (the gap):**

| Mutation kind | Example JS | Why no record |
|---|---|---|
| Attribute set | `el.setAttribute('x','1')` | `SetAttribute` handler → `EcsDom::set_attribute` direct; no `record_mutation` |
| Attribute remove | `el.removeAttribute('x')` | VM `attr_remove` → `EcsDom::remove_attribute` direct |
| Reflected IDL setter (true reflections) | `a.href`, `form.method`, `input.type`, … | direct `EcsDom::set_attribute` in `vm/host/*_proto.rs` |
| `HTMLInputElement.value` (8kHF — **not** a reflection) | `input.value = 'x'` | value-mode dispatch (`html_input_value.rs:120-129`): text-like mode = `ValueSetAction::SetLiveValue` → updates `FormControlState` live value (**`Attributes` untouched** — this is a live-state write, *not* an attribute mutation); only default/default-on mode = `SetContentAttr` → `set_attribute(entity,"value",…)`. So `input.value` is **not** in the same class as content-attribute reflections, and B1 must not put it on an attribute/MO seam unconditionally (text-mode writes would emit a spurious attribute record). |
| `appendChild` / `insertBefore` / `removeChild` / `replaceChild` | `p.appendChild(c)` | bridge handler → `EcsDom::append_child` direct; no `record_mutation` |
| ChildNode/ParentNode mixins | `el.remove()`, `el.before(x)`, `el.append(x)` | `child_node/mutations.rs` direct ops (self-documented `:4-9`) |
| **`Range` mutations (8YcW)** | `r.deleteContents()`, `r.extractContents()`, `r.insertNode(n)` | direct `range.{delete,extract}_contents`/`insert_node(host.dom())` (`range_proto_mutation.rs:73`/`:102`/`:125`); no `record_mutation` |
| **`Node.normalize` (8YcW)** | `el.normalize()` | bridge → `invoke_dom_api("normalize", …)` (`node_methods_extras.rs:270`); handler does direct EcsDom text removal/merge, no `record_mutation` |
| **`textContent` / `nodeValue` setters (886F)** | `el.textContent='x'`, `text.nodeValue='x'` | `node_methods/text_content.rs` `SetTextContentNodeKind`/`SetNodeValue`: Text/CData → `set_text_data` (`:93`/`:145`), Comment → `CommentData` direct write + `rev_version` (`:99-102`/`:148-151`), element → `remove_child`+`append_child` (`:107-113`); **no `record_mutation`** on any branch (childList for element, characterData for Text/Comment all silent) |
| **`Text.prototype.splitText` (886H)** | `text.splitText(3)` | VM `text_proto.rs:119` → `split_text_at_offset` (`char_data/split_text.rs:99`): inserts new sibling (`append_child`/`insert_before`) + `fire_split_text` + truncates via `set_text_data`, **no `record_mutation`** — silent under `observe(parent,{childList,subtree,characterData})` |
| CharacterData on **Text** | `t.data = 'x'` | direct `EcsDom::set_text_data`; `ConsumerDispatcher` `TextChange` fires (engine-internal), but **no observer record** |
| CharacterData on **Comment (8YcL)** | `c.data = 'x'` | `set_char_data` Comment branch (`char_data_handlers.rs:59-73`) writes `CommentData.0` + `rev_version` **only — no `dispatch_event`**; so **neither** a `TextChange` event **nor** an observer record (worse than Text) |

So `new MutationObserver(cb).observe(el, {attributes:true, childList:true,
characterData:true})` in the elidex-js VM fires `cb` **only** when the subtree
is touched via the `innerHTML` or `outerHTML` setter (the two VM natives with an
explicit `deliver_mutation_records`, `dom_inner_html.rs:148`/`:362`). **Not**
`insertAdjacentHTML` — the VM does not install it (`well_known.rs:341-342` =
`insertAdjacentElement`/`insertAdjacentText` only); `InsertAdjacentHtml` is a
dom-api/boa-path producer, not a VM native (§1.4). Every direct DOM API mutation
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

## 4. Canonical Path — open design question for B1 (constraints + coupled invariants)

> **This section does NOT prescribe a mechanism.** Per the §4 status callout
> (top of doc), the canonical MutationObserver-record path is an **edge-dense
> coupled-invariant corner** and, under `CLAUDE.md` "Edge-dense work = multi-PR +
> 実装前 plan-review 必須", that choice is **B1's `/elidex-plan-review` design
> judgment**, not B0's verdict. What follows is the *input* B1 must satisfy: the
> three coupled invariants (§4.1) and the record-source constraints C1–C8 (§4.2a).
> The B1/B2 plan-memos run `/elidex-plan-review` with this corner mapped upfront.

### 4.1 The three coupled invariants any candidate mechanism must satisfy

The reason there is no obvious "just route everything through X" answer is that
three invariants are coupled — fixing one naively breaks another. B1 must satisfy
**all three simultaneously**:

1. **Synchronous apply / read-your-writes.** `el.setAttribute('x','1');
   el.getAttribute('x')` must read back `'1'` within the same script task — no
   flush in between. But `SessionCore::record_mutation` (`session.rs:78-90`)
   **only buffers into `pending`** and applies at `flush` — it is **deferred**.
   So MO-record *buffering* and DOM-write *application* cannot be the same
   deferred step: a "route all writes through `record_mutation`, apply at flush"
   design would make every reflected/attribute read after a write observe the
   pre-write value. The synchronous write and the buffered MO record must be
   decoupled.
2. **`ConsumerDispatcher` fan-out preservation.** The buffered applier
   `apply_set_attribute` (`mutation/mod.rs:288-313`) writes `attrs.set` directly +
   `reconcile_attribute_derived_components` + `rev_version` and — by its own
   comment, "instead of entering `EcsDom::set_attribute`" (`:299-300`) — **does
   not call `dispatch_event`**, so it **does not fan out via `ConsumerDispatcher`**.
   A design that funnels all writes through this buffered path therefore **loses**
   the base-url / form-control / event-handler-attr / canvas / CE consumer
   fan-out that `EcsDom::set_attribute`'s `dispatch_event` (`attribute.rs:118`)
   drives. (8kG9.)
3. **ScriptSession boundary mandate.** Per `CLAUDE.md` "ScriptSession as the sole
   Script↔ECS boundary … 書き込みは session mutation と flush point に集約し、
   SameObject・**MutationObserver**・atomic script-task visibility を同一機構で守る",
   MutationObserver visibility is a ScriptSession-seam-and-flush responsibility —
   so a design that makes `MutationObserver` a `ConsumerDispatcher` consumer
   inside the engine-internal `EcsDom` layer inverts the mandate (and additionally
   shatters innerHTML/outerHTML's single coalesced childList record into N+M
   per-node records, 8YcO — §4.2a C5).

**Why these are genuinely coupled (not separable):** the simplest seam-only
design — "every write becomes a buffered `Mutation`, MO drains at flush" —
satisfies invariant 3 but breaks invariants 1 (deferred apply) **and** 2 (no
dispatcher fan-out). The simplest dispatcher-based design — "MutationObserver is a
`ConsumerDispatcher` consumer" — satisfies invariant 1 (synchronous) but breaks
invariant 3 (mandate inversion) and the record-shape constraint. **Neither pole is
correct as-is.** A satisfying mechanism likely has to (a) keep the synchronous
write at the `EcsDom::set_attribute` chokepoint (invariants 1 + 2) **while** (b)
producing the MO record at the ScriptSession seam (invariant 3) — i.e. separate
*where the write applies* from *where the MO record originates*. **How to wire
that — and whether it requires a seam-side record emitted at the same call as the
synchronous chokepoint write, a flush-coalescing layer, or another structure — is
the design B1 owns.** §4.2a's C1–C8 are the additional record-source constraints
that mechanism must also reproduce.

> **Lesson #181 *does* tension with the naive seam-only routing (correcting an
> earlier draft's claim).** A prior revision asserted that "`record_mutation`'s
> `apply_*` step still bottoms out at the same `EcsDom::set_attribute` chokepoint,
> so #181 is not in tension with seam-owned MO." **That is wrong.** The buffered
> applier `apply_set_attribute` (`mutation/mod.rs:288-313`) does **not** call
> `EcsDom::set_attribute` — it duplicates only the `reconcile_*` + `rev_version`
> fragment and explicitly bypasses the chokepoint ("instead of entering
> `EcsDom::set_attribute`"). #181 consolidated attribute writes onto the
> `EcsDom::set_attribute` chokepoint precisely so derived-component / live-range /
> form-state reconcile **and** the `ConsumerDispatcher` fan-out happen at write
> time; the buffered seam path **bypasses** that chokepoint, so routing writes
> through it is in **direct tension** with #181 (it re-introduces the very
> attribute-write fork #181 collapsed). The immediate dom-api `SetAttribute`
> *handler* (`element/props.rs:43`) *does* call `EcsDom::set_attribute` (that path
> honours #181) — but it does **not** record a `Mutation`, so it is not the MO
> path. This non-equivalence (immediate-chokepoint-but-no-record vs.
> buffered-record-but-no-chokepoint) is the heart of the coupled corner, and a
> reason invariant 1 + invariant 2 cannot be waved off. B1 resolves it.

### 4.2a Record-source constraints the canonical (seam) path must satisfy

Whatever mechanism B1 chooses, its MO-record source must reproduce these, each
verified at HEAD `26d00c5a`. They are stated as constraints, not as an argument
for a particular mechanism; where a constraint discriminates between recording
*at a dispatcher event* vs. *upstream of the dispatcher*, that is noted as a
factor for B1 to weigh (it does **not** settle the choice — invariants 1 + 2 of
§4.1 pull the other way):

- **C1 — replaceChild coalescing.** `EcsDom::replace_child`
  (`tree/mutation.rs:185-205`) fires **two** events: `fire_after_remove(old)`
  then `fire_after_insert(new)`. The spec single childList record for a replace
  carries `addedNodes` *and* `removedNodes` in **one** record, which the
  intent-driven `apply_replace_child` (`mutation/mod.rs:268-285`) produces by
  construction, whereas an event-driven source must coalesce the two dispatcher
  events. *Favors recording from intent* (Pole B), but B1 must still ensure the
  replaceChild record carries the replace intent (not two separate add/remove
  records).
- **C2 — non-dispatching attribute writes (spec-observable, must be closed).**
  `set_attribute_without_dispatch` (`attribute.rs:146`) fires **no**
  `MutationEvent` (used inside consumers where re-entry forbids dispatch). Form
  value-mode/type-change calls it via `apply_type_change_value_migration` to move
  the live value into the `value` **content attribute** when the live value is
  non-empty (`elidex-form/value_mode.rs:222`). **This is a real content-attribute
  write, and per WHATWG DOM §4.9 Interface Element it is spec-observable, not
  per-plan optional:** appending/changing/removing a content attribute runs "change
  an attribute" → "handle attribute changes" (`#handle-attribute-changes`), whose
  **step 1 queues an `"attributes"` mutation record** for the element. So a
  `new MutationObserver(cb).observe(input, {attributes:true,
  attributeFilter:['value']})` is **owed** a record when a type-change migrates the
  live value into the `value` attribute. The earlier "generally not
  script-observable / internal reflection" framing was **wrong** and is corrected
  here: B1 **must** close this silent write (it is exactly an `attributes`
  mutation the spec requires observers to see). An event-driven (Pole A) source
  would **never see** it (no `MutationEvent`) — a hard hole that makes Pole A
  insufficient by itself; a record emitted independent of the event (Pole B) can
  capture it. Note this is the content-attribute migration step only — it does
  **not** include the text-like-mode live-value write (which leaves `Attributes`
  untouched and must stay record-free, 8kHF).
- **C3 — shadow-root suppression.** `fire_after_insert`/`fire_after_remove`
  (`tree/mutation.rs:289`, `:343-344`) suppress Insert/Remove when node/parent is
  a ShadowRoot. An event-driven (Pole A) source would silently miss shadow-root
  childList mutations; a record emitted *upstream* of the dispatcher (Pole B) can
  capture them without touching the light-tree-only suppression contract. *Favors
  recording upstream of the dispatcher.* Whichever mechanism is chosen, the
  existing §4.3.2 inclusive-ancestor walk still gates delivery (a MO must
  explicitly observe inside the shadow tree).
- **C4 — boa buffered iframe writes (with an invariant-2 caveat).**
  `iframe.rs:99`/`:105`/`:206` already record
  `Mutation::SetAttribute`/`RemoveAttribute`, applied by
  `apply_set_attribute`/`apply_remove_attribute` (`mutation/mod.rs:288-332`)
  which self-generate the record. These are a *precedent* for a seam-recorded
  attribute write — but **not yet a clean model to converge onto**, because that
  buffered applier **bypasses `EcsDom::set_attribute` and so does not fan out via
  `ConsumerDispatcher`** (invariant 2 / 8kG9). So if B1 converges other attribute
  writes onto this exact buffered path, they would lose the consumer fan-out;
  conversely if B1 routes them through the chokepoint, the iframe path is the one
  that would need reconciling. B1 must resolve which way the convergence runs —
  this is a constraint, not a solved special case.
- **C5 — innerHTML/outerHTML bulk-coalescing (8YcO).** `apply_set_inner_html` /
  `apply_set_outer_html` (`html_fragment.rs:55`/`:116`) emit **one** coalesced
  `ChildList` record (`added_nodes` + `removed_nodes`) for a whole-subtree
  replace, even though the underlying op does N `remove_child` + M `append_child`
  (each firing a per-node dispatcher event). An intent-driven record (Pole B)
  preserves this bulk shape by construction (built from the `SetInnerHtml`
  intent); an event-driven source (Pole A) would shatter it into N+M records.
  *Favors recording from intent.* Whichever way B1 goes, it must reconcile the
  *explicit* `deliver_mutation_records` at `dom_inner_html.rs:148`/`:362` so a
  given innerHTML/outerHTML mutation is delivered exactly once (no double-delivery,
  no per-node shattering).
- **C6 — "replace all" coalescing for `replaceChildren` / element `textContent`
  (9dTW).** Two more script-reachable whole-subtree replaces are implemented as
  **remove-all-then-insert** loops of per-node `EcsDom` ops, *not* as a single
  intent — so they carry the **same** N+M-shatter risk as innerHTML (C5) but have
  **no** record at all today (§3 gap rows):
  - **`replaceChildren`** (`child_node/mutations.rs:275-283`) removes every existing
    child via a `remove_child` loop, then inserts the new node(s).
  - **element `textContent =`** (`node_methods/text_content.rs:106-113`, the element
    branch) removes every child via a `remove_child` loop, then optionally appends a
    single text node.
  Per **WHATWG DOM §4.2.3 Mutation algorithms**, "replace all" (`#concept-node-replace-all`)
  removes all children **with `suppressObservers` true** and inserts the new node
  **with `suppressObservers` true**, then (step 7) **queues a single tree mutation
  record** for the parent carrying `addedNodes` *and* `removedNodes` together —
  i.e. **one** aggregate `ChildList` record, not N+M per-node records. (Both
  `replaceChildren`, DOM §4.2.6 Mixin ParentNode (`#dom-parentnode-replacechildren`),
  and the `textContent` setter, DOM §4.4 Interface Node (`#dom-node-textcontent`),
  are spec-defined in terms of "replace all".) So C6
  is the same intent-vs-event discriminator as C5: a seam/intent-driven source
  yields the single aggregate record by construction, while an event-driven source
  (Pole A) must coalesce the per-node `remove_child`/`append_child` dispatcher events
  back into one record. *Favors recording from intent.* B1 must produce **one**
  coalesced record for `replaceChildren` and element `textContent`, matching the C5
  innerHTML/outerHTML shape (§4.3 record-shape correctness).
- **C7 — CE-reaction *order* preservation across coalesced records (9dTU — a named
  coupled invariant).** A coalesced childList record (C1 replaceChild / C5
  innerHTML / C6 replace-all) is **not** order-free: the spec/CE consumer cares
  about added-vs-removed ordering, and the current sources fix a specific order that
  a naive "node-set match" record loses:
  - The flush-side CE scan `enqueue_ce_reactions_from_mutations`
    (`elidex-js-boa/runtime/ce.rs:145`) iterates **added nodes first** (connected,
    `ce.rs:24` region) **then removed nodes** (disconnected) within a single record.
  - `EcsDom::replace_child` (`tree/mutation.rs:185-205`) dispatches
    **`fire_after_remove(old)` (`:189`) then `fire_after_insert(new)` (`:200`)** —
    Remove **before** Insert.
  - `apply_set_inner_html` (and outerHTML) likewise **remove-old then append-new**.
  So if B1's record/coalesce step or its flush-side CE scan reorders added vs.
  removed (or treats a coalesced record as an unordered node-set), the CE
  `disconnected`-then-`connected` (or `connected`-then-`disconnected`) **callback
  firing order can invert** relative to today. **B1 must treat
  added/removed-node ordering inside a coalesced record as a load-bearing
  invariant** — record production, coalescing, *and* the flush-side CE scan must
  agree on one order — alongside C1–C6, not as an afterthought. (This couples
  record-shape coalescing with CE-reaction semantics — §4.3's CE-reaction
  preservation bullet — at the same source.)
- **C8 — characterData `oldValue` capture *timing* (9dTY — a named coupled
  invariant).** `{characterDataOldValue:true}` (DOM §4.3.3) requires the **pre-write**
  character data, but on the Mechanism-A / `ConsumerDispatcher` path the old value is
  **already gone** by the time any consumer runs:
  - `EcsDom::set_text_data` (`elidex-ecs/dom/mod.rs:332`) **overwrites the
    `TextContent` buffer** (`:336`) **before** firing `MutationEvent::TextChange`
    (`:340-344`), and that event carries only `{ node, new_utf16_len }`
    (`mutation_event.rs:191`) — **no old value**.
  - `ReplaceData` (`mutation_event.rs:209`) carries only `{ offset, count }` —
    again no old value, and the splice has already mutated the buffer.
  - The Comment branch of `set_char_data` (`char_data_handlers.rs:59-73`) overwrites
    `CommentData.0` in place with no event at all.
  So an **event-driven (Pole A) characterData-oldValue source is impossible by
  construction** — the dispatcher event neither carries the old value nor fires
  before the overwrite. The old value must be **captured at the seam/handler,
  *before* the `EcsDom` write**, where it is still in hand (§4.3 `oldValue`
  threading bullet). **B1 must treat characterData `oldValue` capture-timing as a
  C1–C7-class constraint**: it is a hard reason an event-driven source cannot fully
  serve `characterDataOldValue`, coupling the record source with capture ordering at
  one upstream point (the same coupling C2/C3 raise for attribute oldValue and
  shadow-root suppression).

### 4.2 Candidate directions B1 weighs (neither pre-decided here)

B0 enumerates the design space without picking; B1's `/elidex-plan-review` decides.
Two poles bound it, and §4.1 already showed **neither pole is correct as-is** —
the answer is likely a structure that separates *where the write applies* from
*where the MO record originates*. The poles, with the trade-off each carries:

- **Pole A — `MutationObserver` as a `ConsumerDispatcher` consumer.** Add a
  consumer translating each `MutationEvent` into a `MutationRecord`. *Satisfies
  invariant 1* (synchronous, at the chokepoint) and *invariant 2* (rides the
  existing fan-out). *Tensions:* (i) **invariant 3** — puts a script-observable
  responsibility in the engine-internal `EcsDom` layer, against the ScriptSession
  mandate; (ii) **record shape (C5/8YcO)** — per-node `Insert`/`Remove` events
  shatter innerHTML/outerHTML's single coalesced childList record into N+M, and
  force ad-hoc replaceChild coalescing (C1); (iii) **coverage holes** — blind to
  the non-dispatching `value` write (C2) and shadow-root-suppressed events (C3),
  and would special-case the buffered iframe path (C4).
- **Pole B — ScriptSession seam owns MO record production.** Every script-visible
  mutation records a `Mutation` (via `elidex-dom-api` / `DomApiHandler`, keeping
  `vm/host/` marshalling-only), MO drains at `flush`. *Satisfies invariant 3*
  (mandate) and produces C5/C1 coalesced shapes by construction, recording
  upstream of C2/C3 suppression. *Tensions:* (i) **invariant 1** — the naive form
  (apply at flush) defers the write, breaking read-your-writes; B1 must keep the
  synchronous apply at the chokepoint while buffering only the *record*; (ii)
  **invariant 2 / lesson #181** — if the record's `apply_*` uses the buffered
  `apply_set_attribute` (which bypasses `EcsDom::set_attribute`), it loses the
  `ConsumerDispatcher` fan-out and re-forks the attribute write #181 collapsed;
  (iii) **blast radius** — the §1 write sites must each record a `Mutation`;
  (iv) **new flush→MO hook** (8YcR) — none exists today
  (`Microtask::NotifyMutationObservers` wires only `slotchange`,
  `natives_promise.rs:333-344`); it must cover **both** the per-frame `re_render`
  flush and the `flush_with_ce_reactions` page-load flush (§2.2).

Neither pole is free. A satisfying mechanism plausibly **records the MO entry at
the ScriptSession seam (invariant 3, correct shapes) while keeping the synchronous
write + dispatcher fan-out at the `EcsDom::set_attribute` chokepoint (invariants 1
+ 2)** — i.e. emit the seam record *at the same call as* the synchronous chokepoint
write, rather than deferring application to flush. **Whether that, a
flush-coalescing layer, or another structure is correct — and how to thread it
through `elidex-dom-api` handlers, the reflected setters (§4.5), the dual runtime,
and the C1–C8 constraints — is the B1 design judgment.** B0 deliberately stops
short of choosing.

### 4.3 Cross-cutting work any direction inherits (for B1's plan)

Independent of which mechanism B1 picks, these must be handled and are listed so
B1's plan-review covers them:

- **Record-shape correctness (C5/C1, 8YcO).** `apply_set_inner_html`
  (`html_fragment.rs:85-89`) does N `remove_child` + M `append_child` internally
  (N+M dispatcher events) but returns **one** `ChildList` record with both
  `added_nodes` and `removed_nodes` — DOM §4.3.2's single coalesced shape. Any
  event-driven source must reconstruct this by coalescing N+M same-parent events
  within a dispatch (the events are per-node, carrying no "part of a bulk replace"
  marker); a seam/intent-driven source gets it by construction. replaceChild (C1)
  is the same shape. This is the sharpest discriminator between the poles.
- **No double-delivery for innerHTML/outerHTML.** Today the explicit
  `deliver_mutation_records` at `dom_inner_html.rs:148`/`:362` is the only VM MO
  delivery. If B1 adds a flush→MO path, it must retire or reconcile these so a
  given innerHTML mutation is delivered exactly once.
- **CE-reaction preservation (8ykQ — Mechanism B is not MO-only).** The session
  buffer feeds **two** script-visible consumers: `MutationObserver` *and*
  custom-element reactions (the shell drains flush records through
  `enqueue_ce_reactions_from_mutations`, `ce.rs:137-145`, in both
  `flush_with_ce_reactions` (`pipeline.rs:29-34`) and per-frame `re_render`
  (`lib.rs:618-628`) — §2.2). So B1 must **not** treat `record_mutation` /
  `flush` as an MO-only channel: any change to record *production*, *coalescing*,
  or *delivery ordering* must preserve the CE-reaction scan (added/removed CE
  nodes → `connected`/`disconnected`, attribute records →
  `attributeChangedCallback`). In particular a Pole-B record-shape change (e.g.
  coalescing or re-ordering childList records) must keep CE
  `enqueue_ce_reactions_from_mutations` seeing the same added/removed node set
  **in the same order** (the added-then-removed / Remove-before-Insert order is a
  named constraint — **C7**, §4.2a, 9dTU), and the flush→CE drain must continue to
  run on the page-load (`flush_with_ce_reactions`) path, not only per-frame.
- **CommentData notification + live-range coupling (8YcL / 9WUB) — a *coupled*
  invariant, not a lone missing record.** Comment characterData fires no event
  today (§1.4b); the current Comment path writes only `CommentData` + `rev_version`
  (`char_data/char_data_handlers.rs:59-73`). But Comment implements
  CharacterData, and the canonical fix is **not** "emit one missing
  `characterData` record": per WHATWG DOM §4.10 Interface CharacterData "replace
  data" (`#concept-cd-replace`), the same algorithm that **queues the
  `"characterData"` mutation record** (step 4) **also adjusts every live range
  whose boundary point is inside the spliced node** (steps 8–11, over the
  §5.5 Interface Range "live range" concept, `#concept-live-range`). So a
  seam-emitted MO record **alone** leaves Range boundary points stale on a
  `comment.data`/`appendData`/`deleteData`/… splice — the record and the
  live-range boundary adjustment are **one coupled invariant** that B1 must satisfy
  together, for **all** character-data splices (Comment *and* Text/CData), not just
  the Comment record hole. (Today Text/CData boundary adjustment rides the
  `LiveRangeBridge` consumer off the dispatcher `TextChange`/`ReplaceData` events;
  Comment fires no event, so it gets **neither** the record **nor** the
  boundary adjustment — the hole is double.) **B1 must therefore design the
  character-data fix so MO-record production and live-range boundary adjustment
  stay coupled at one source** (§4.10/§5.5), rather than wiring a seam record that
  the dispatcher-side `LiveRangeBridge` no longer covers for Comment. This is a
  named §4 coupled invariant handed to B1's `/elidex-plan-review`, not solved here.
- **`oldValue` threading.** `characterDataOldValue` / attribute `oldValue` need
  the pre-write value captured before the `EcsDom` write (the handler has it in
  hand). For **characterData** this is a hard capture-*timing* constraint — **C8**
  (§4.2a, 9dTY): the `EcsDom::set_text_data` / Comment write overwrites the buffer
  **before** any dispatcher event fires, and `TextChange`/`ReplaceData` carry no old
  value, so an event-driven source cannot serve `characterDataOldValue` at all — the
  old value must be captured at the seam/handler upstream of the write. `attributeNamespace`
  stays deferred to `#11-mutation-observer-extras` (`mutation_event.rs:295-298`).
- **Dual-runtime delivery.** Both VM and boa flush through `SessionCore`, so a
  seam-side record is runtime-uniform; a flush→MO hook must exist in the boa flush
  path until S5 removes boa. A dispatcher-consumer (Pole A) is VM-only (the
  dispatcher is installed only by `Vm::bind`, §2.1), a larger S5 coupling — itself
  a factor for B1.

### 4.5 B2 — bridge/direct + setAttribute/removeAttribute + reflected-setter convergence

§1 surfaced a uniformity gap that B1's mechanism choice will shape: today
`setAttribute` routes through a `DomApiHandler` while `removeAttribute` and the
reflected IDL setters do not.

> **B2 summary — corrected from "whether" to "where/how" (9dTS).** An earlier draft
> left B2 *gated on* "**whether** reflected writes carry an MO record at all". That
> framing is **inconsistent with §4.5's own reflected-setter bullet (8YcT) below**,
> which establishes — via webref-verified spec — that a *true* reflected IDL setter
> (`a.href`, `form.method`, `input.type`, …) **is** an observable attribute mutation
> (HTML §2.6.1 reflection contract = "set the content attribute" → WHATWG DOM §4.9
> "change an attribute" → "handle attribute changes" step 1 **queues an
> `"attributes"` mutation record**), so a `{attributes:true,
> attributeFilter:['href']}` observer **is owed** that record — *spec-settled, not a
> per-plan option*. The **sole exception is `input.value`** (8kHF): its setter is a
> value-mode dispatch, **not** "set the content attribute", so a text-like-mode
> `input.value = 'x'` is a non-attribute live-state write (`FormControlState`,
> `Attributes` untouched) that must **not** emit a spurious attribute record. So the
> open B2 question is **not** "observable or not" — that is settled (true reflections:
> record owed; `input.value` text-mode: record-free) — it is **where/how** the
> (spec-owed) MO record is recorded: which layer / mechanism, per §4.1's coupled
> invariants. This propagates §4.5's 8YcT/8kHF conclusion into the B2 summary so the
> summary states *where/how*, not *whether*.

**The target direction depends on B1's §4.1/§4.2
decision** (the recording layer/mechanism for the spec-owed reflected-write record,
per the summary above), so B2 is *gated on* B1 rather than independently prescribed.

> **B2 convergence scope = *all* direct-`EcsDom` attribute-write host APIs (9dTV).**
> The uniformity gap is **not** confined to `removeAttribute` + the reflected
> setters. §1.1 + the code show the full set of `vm/host/` attribute-write APIs that
> reach `EcsDom::set_attribute`/`remove_attribute` **directly** (no `invoke_dom_api`
> seam), every one of which B2's convergence must include alongside `removeAttribute`
> and the reflected setters:
> - **`toggleAttribute`** — via `attr_set` (`element_attrs.rs:577`/`:592`) →
>   `EcsDom::set_attribute` (and the absent-branch removal, `:558` region).
> - **`Attr.value =` setter** — `attr_proto.rs:416` → `EcsDom::set_attribute` direct.
> - **`NamedNodeMap.setNamedItem` / `removeNamedItem`** —
>   `named_node_map.rs:345`/`:431` → `EcsDom::set_attribute`/`remove_attribute` direct.
> - **`setAttributeNode` / `removeAttributeNode`** — `element_attrs.rs:414`/`:535` →
>   `EcsDom::set_attribute`/`remove_attribute` direct.
>
> So B2's "route attribute writes uniformly" convergence scope is **{`removeAttribute`,
> reflected IDL setters, `toggleAttribute`, `Attr.value`, `setNamedItem`,
> `removeNamedItem`, `setAttributeNode`, `removeAttributeNode`}** — the whole direct-
> `EcsDom` attribute-write surface, not the two originally listed. (Each carries the
> same VM-local `Attr`-detach precondition where it removes an attribute, per the
> first bullet below.)

The Layering
mandate (`vm/host/` marshalling-only) and the following per-site facts constrain
whatever B2 does:

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
- **Reflected IDL setters (8YcT) — and the `input.value` non-reflection caveat
  (8kHF).** Today true reflections — `a.href`, `form.method`, `input.type`, etc. —
  call `EcsDom::set_attribute` **directly from `vm/host/`**
  (`html_input_proto.rs:460`/`:544`/…, `html_button_proto.rs`,
  `html_textarea_proto.rs` — verified at HEAD `26d00c5a`). **`HTMLInputElement.value`
  is *not* a plain reflection** (8kHF): `html_input_value.rs:120-129` dispatches by
  value mode — text-like mode writes `FormControlState` live value
  (`ValueSetAction::SetLiveValue`, **no attribute mutation**), only default/default-on
  mode writes the `value` content attribute (`SetContentAttr`). So B1/B2 **must not**
  treat `input.value` as an attribute write to put on an attribute/MO seam
  unconditionally — a text-mode `input.value = 'x'` is a live-state write that must
  not emit a spurious attribute `MutationRecord`. The Layering mandate (algorithm
  out of `vm/host/`) and the question of whether/where reflected writes carry an MO
  record are **inputs to B1's §4.1/§4.2 decision**, not a settled "route everything
  through `record_mutation`". **But the open question is narrower than "are
  reflected writes observable at all" — tightened.** A true reflected IDL setter
  (`a.href`, `form.method`, `input.type`, …) *is* an **observable attribute
  mutation**: per HTML §2.6.1 "Reflecting content attributes in IDL attributes"
  (`#reflecting-content-attributes-in-idl-attributes`, §1.2) its contract is "set
  the content attribute", and it does so through the `EcsDom::set_attribute`
  chokepoint (§2.6.1/§1.2), so the underlying attribute change is a DOM "change an
  attribute" → "handle attribute changes" step (WHATWG DOM §4.9 Interface Element,
  `#handle-attribute-changes`) which **queues an `"attributes"` mutation record**.
  That a `{attributes:true, attributeFilter:['href']}` observer must see
  `a.href = …` is therefore **spec-settled, not a per-plan option**. The open
  question is **not** "observable or not" — it is **where/how the MO record is
  recorded** (which layer / mechanism, per §4.1's coupled invariants). B1 must
  **not** read this section as license for a plan that fixes `setAttribute` but
  leaves `a.href = …` MO-silent. **The sole exception is `input.value` (8kHF):**
  its setter is a value-mode dispatch, **not** "set the content attribute" — in
  text-like mode it is a non-attribute live-state write
  (`FormControlState`, `Attributes` untouched), so it must **not** emit a spurious
  attribute `MutationRecord`. What B0 *does* fix: true reflections are observable
  attribute mutations (record owed); `input.value` value-mode dispatch must be
  preserved (live-state vs. content-attribute split); and `vm/host/` stays
  marshalling-only. The remaining B1 choice is the recording mechanism/location,
  not observability.
- **`removeAttribute` symmetry carries the VM-local Attr detach (unchanged).**
  Routing VM `removeAttribute` through the bridge for symmetry with `setAttribute`
  must carry the `attr_remove` VM-local work forward — snapshot-freeze the JS-held
  `Attr` wrapper's `detached_value` + `invalidate_attr_cache_entry`
  (`element_attrs.rs:180-187`), which the bridge `RemoveAttribute` handler
  (`element/props.rs:108-122`) does not do (it invalidates only the ECS
  `AttrEntityCache`). This is a precondition on any symmetry fix — whichever
  direction B1 picks, the VM-local `Attr`-wrapper detach must be carried forward
  (e.g. a VM-side post-step the handler signals).

### 4.6 Sequencing

**B1's `/elidex-plan-review` resolves the §4.1 coupled-invariant corner and picks
the mechanism** (correctness: close the MutationObserver gap of §3 + the C1–C8
constraints + 8YcL/8YcW), **before** B2 (the bridge/direct + reflected-setter
convergence of §4.5), since B2's target shape depends on B1's mechanism choice.
Both are `/elidex-plan-review`-gated per `CLAUDE.md` "Edge-dense work = multi-PR +
実装前 plan-review 必須"; whether B2 is a separate slice or the write-site half of
B1 is itself a plan-review outcome. **Dual-runtime note:** both VM and boa flush
through `SessionCore`, so a seam-side record is runtime-uniform; a
`ConsumerDispatcher`-consumer mechanism is VM-only (dispatcher installed only by
`Vm::bind`, §2.1), a larger S5 coupling — a factor B1 weighs, not a settled
verdict here.

---

## 5. Spec / Design SSoT Cross-Reference

- **WHATWG DOM §4.3** — MutationObserver; §4.3.2 "queue a mutation record"
  (per-observer queue, microtask delivery, inclusive-ancestor target walk);
  §4.3.3 Interface MutationRecord (record shape).
- **`docs/design/ja/12-dom-cssom.md`** — line 24: read-only `&EcsDom`, "書き込みは
  `session.record_mutation()`経由" (the `DomApiHandler::invoke` `dom: &EcsDom`
  comment, §12.1.1); line 47: "MutationObserver … セッションflushが
  バッファされた変更からMutationRecordsを生成。ファーストクラス" (§12.1.2 core/compat
  table row). (Corrected from an earlier draft's stale "line 5"/"line 28"
  anchors — re-checked by direct read of `12-dom-cssom.md`.) This is the design
  *aspiration* B1 reconciles against the §4.1 invariants. B0 does **not** declare
  it satisfied or stale: §12 describes a seam-recorded MO path, but it does not by
  itself resolve how that coexists with synchronous read-your-writes (invariant 1)
  and the `EcsDom::set_attribute` chokepoint fan-out (invariant 2 / #181). That
  reconciliation is B1's plan-review.
- **`docs/design/ja/28-adr.md`** — ADR #17 (`ScriptSession` = unified Script↔ECS
  boundary providing Identity Map / **Mutation Buffer** / GC / **consistent
  MutationObserver records** "単一メカニズムで実現") — the **SSoT for the boundary's
  existence**: MutationObserver visibility belongs on the ScriptSession seam
  (invariant 3). It establishes *that* the seam owns MO records; it does **not**
  prescribe the *mechanism* by which every write reaches the seam while preserving
  invariants 1+2 — that is the open question §4 hands to B1. ADR #14
  ("MutationObserver がECS変更検出に自然にマッピング") describes the implementation
  substrate (ECS change detection feeds record production), not a license to put MO
  production in the `EcsDom` layer.
- **`CLAUDE.md`** — "ScriptSession as the sole Script↔ECS boundary … 書き込みは
  session mutation と flush point に集約し、SameObject・MutationObserver・atomic
  script-task visibility を同一機構で守る" (invariant 3, §4.1); **"Edge-dense work =
  multi-PR + 実装前 plan-review 必須"** (the rule that makes the §4 mechanism choice
  a B1 plan-review judgment, not a B0 verdict — the normative basis for this
  revision's §4 downgrade); "One issue, one way"; Layering mandate (Axis 1a —
  algorithm bodies belong in engine-independent crates, `vm/host/` is
  marshalling-only — see §4.5).
- **lesson #181** (cited in code: `attribute.rs:5-15`, `element/props.rs:61`)
  — the canonical `EcsDom::set_attribute` write-path consolidation. **In tension
  with a naive seam-only routing** (correcting an earlier draft, §4.1 callout):
  the buffered applier `apply_set_attribute` (`mutation/mod.rs:288-313`)
  **bypasses** the `EcsDom::set_attribute` chokepoint (it does `attrs.set`
  directly, no `dispatch_event`), so routing writes through *that* path re-forks
  the attribute write #181 collapsed and drops the `ConsumerDispatcher` fan-out
  (invariant 2). Keeping #181 intact (synchronous chokepoint write + fan-out)
  while still producing a seam-side MO record (invariant 3) is exactly the corner
  B1 resolves. The immediate dom-api `SetAttribute` *handler* (`element/props.rs:43`)
  honours #181 by calling `EcsDom::set_attribute` — but it records **no**
  `Mutation`, so it is not today's MO path; that non-equivalence is the crux.

---

## 6. Re-check Discipline (for B1/B2 plan-memos)

- Re-grep every `file:line` here at PR-open — line numbers will drift.
- **Produce the exhaustive write-site set by grep-diff, not by extending §1's
  table** (the §1 invariant + methodology). The *covered* side is the **union**
  established in §1, not `record_mutation` call-sites alone: (a) every
  `record_mutation` call-site **whose flush reaches `deliver_mutation_records`**
  (per-frame `re_render` → `content/mod.rs:258`, **not** the CE-only
  `flush_with_ce_reactions`, `pipeline.rs:25-34`) **∪** (b) every **direct
  `deliver_mutation_records` producer** (the VM `innerHTML`/`outerHTML` setters,
  `dom_inner_html.rs:148`/`:362`, which deliver their own record without
  `record_mutation`). Then enumerate every direct `EcsDom`/component-mutator call
  across `vm/host/`, `elidex-dom-api`, `elidex-js-boa`, and `elidex-ecs`, and diff.
  **Diffing against `record_mutation` call-sites alone would both false-positive
  the direct-delivery `innerHTML`/`outerHTML` natives (covered via (b)) and
  false-negative a record-but-only-CE-flushed site** — so the grep-diff must
  include flush/delivery sites, not just recording calls (§1). The R5 misses
  (textContent/nodeValue/splitText, §1.6) came from reactive hand-enumeration; the
  grep-diff (with this union as the covered set) is the SoT for completeness.
- Re-confirm both boa CE-reaction producers (886B): the flush-record scan
  (`enqueue_ce_reactions_from_mutations`, `ce.rs:137-145`) **and** the
  binding-direct enqueue (`globals/element/core.rs:152-176`/`:219`/`:292`), so a
  record-production change does not double-enqueue or miss CE reactions.
- Re-confirm the §2 mechanism by direct read of `attribute.rs`
  (`set_attribute`/`dispatch_event`), `tree/mutation.rs`
  (`Insert`/`Remove` fire sites), `consumer_dispatcher.rs` (consumer list),
  and `mutation_observer.rs` (`deliver_mutation_records`). Do not carry this
  reframe forward on trust — Program B's correctness depends on it.
- Re-confirm the §4 coupled-invariant anchors by direct read: `record_mutation`
  deferred-apply (`session.rs:78-90` — invariant 1), `apply_set_attribute`
  bypassing `EcsDom::set_attribute` / no `dispatch_event`
  (`mutation/mod.rs:288-313` — invariant 2 / 8kG9 / #181 tension), `set_char_data`
  Comment branch (`char_data/char_data_handlers.rs:59-73`, no `dispatch_event` —
  8YcL), `apply_set_inner_html` single-record shape (`html_fragment.rs:85-89` —
  8YcO/C5), the missing flush→MO hook (`natives_promise.rs:333-344` dispatches
  slotchange only — 8YcR), the `input.value` value-mode dispatch
  (`html_input_value.rs:120-129` — 8kHF, *not* a plain reflection), the
  reflected-setter direct writes (`html_input_proto.rs` etc. — 8YcT), the
  **replace-all** remove-all-then-insert loops (`child_node/mutations.rs:275-283`
  `replaceChildren` + element `textContent` `node_methods/text_content.rs:106-113`
  — C6/9dTW, DOM §4.2.3 single tree mutation record), the **CE-reaction order**
  anchors (`enqueue_ce_reactions_from_mutations` added-then-removed `ce.rs:145`/`:24`
  + `replace_child` Remove-before-Insert `tree/mutation.rs:189`/`:200` — C7/9dTU),
  and the **characterData `oldValue` capture-timing** anchors (`set_text_data`
  overwrite-before-dispatch `elidex-ecs/dom/mod.rs:336`/`:340-344`, `TextChange`/
  `ReplaceData` carry no old value `mutation_event.rs:191`/`:209` — C8/9dTY).
- Re-check active branches (`git branch -r`) for convergence drift on
  `element_attrs.rs` / `vm/host/` attribute setters (MED collision risk with
  JS-side work; low overlap with media Slice 2b today, but B is later — Axis 5).
- Slot check: `#11-mutation-observer-extras` (attributeNamespace, primitive
  ToObject for `observe`) must still be open before referencing it.

## Review guidelines (for Codex)

- This is a **doc-only** audit. Verify the `file:line` anchors against
  `main` and challenge any mechanism claim that does not match the code —
  especially §0/§3 (the MutationObserver gap, incl. Range/normalize/Comment
  rows + the `input.value` non-reflection 8kHF + the R5 textContent/nodeValue/
  splitText rows 886F/886H) and §4.
- **§1's write-site map is deliberately a *representative known-set + invariant +
  B1 grep-diff methodology*, not an exhaustive hand registry** (R1→R5 showed
  hand-enumeration keeps losing sites). So do **not** flag §1 for "missing site X"
  as a registry defect — instead check that (a) the §1 invariant
  (record_mutation-bypassing mutator = MO-silent) is correctly stated, and (b) any
  site you find is consistent with it. A genuinely *mis-stated* invariant or a row
  that contradicts the code is still in scope.
- **CE-reaction sources are two systems (886B):** flush-record scan **and**
  binding-direct enqueue (`globals/element/core.rs`). Challenge §2.2 if either
  producer is mis-described.
- **§4 is deliberately *not* a prescribed fix.** This revision downgrades it from
  the R2 "ScriptSession-seam-owned MO is canonical" prescription to an **open
  design question for B1's `/elidex-plan-review`**, because §4 is an edge-dense
  coupled-invariant corner (≥3 axes — §4.1) and `CLAUDE.md` "Edge-dense work =
  multi-PR + 実装前 plan-review 必須" reserves that choice for B1. So: do **not**
  flag §4 for "failing to pick a mechanism" — picking one in a B0 audit would be
  the mandate violation. **Do** flag if (a) any of the three §4.1 coupled
  invariants is mis-stated or mis-attributed, (b) a §4.2a constraint (C1–C8) is
  wrong, (c) the §4.2 Pole-A/Pole-B trade-offs mis-describe the code, or (d) the
  #181 / `apply_set_attribute`-bypass tension (§4.1 callout, §5) is mis-read.
- Out of scope: implementing B1/B2; touching `element_attrs.rs`, reflected
  IDL setters, `range_proto_mutation.rs`, `char_data` handlers, or
  `ConsumerDispatcher`.
