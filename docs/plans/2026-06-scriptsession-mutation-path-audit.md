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
> (which mechanisms exist, what the real gap is) and the **named invariants** the
> canonical fix must satisfy — it does **not** characterise behavior: no per-write
> site, per-op event sequence, per-API record shape, per-path delivery, or
> per-case (same/cross-parent, light/shadow, move/insert) behavior. That detailed,
> code-level, exact-per-instance characterization is **B1's `/elidex-plan-review`
> job**: B1 derives it by **grep-diff** against the live code (the methodology is
> in §1 / §6). An earlier revision pushed per-op / per-API / per-parent / per-path
> behavioral detail into this audit and it became a *finding generator* — each
> behavioral claim carries its own edge-case combinatorial space that cannot be
> precisely pinned in B0 prose, and every attempt to do so spawned a correction.
> So this revision states each fact as a **named invariant** with at most a *site
> pointer* (never a behavioral description), and defers all behavioral derivation
> to B1. Where you see "B1 derives the behavior", that is deliberate, not an
> omission.

> **§4 status — open design question for B1, not a prescribed fix.** §4 sits on an
> **edge-dense coupled-invariant corner** (≥3 intersecting invariant axes:
> synchronous-apply / `ConsumerDispatcher` fan-out / ScriptSession-MO ownership /
> record-shape coalescing / dual-runtime). Under `CLAUDE.md` "Edge-dense work =
> multi-PR + 実装前 plan-review 必須" + `feedback_coupled-invariant-design-corner`,
> a B0 *audit* prescribing a single canonical mechanism here would be a mandate
> violation: that decision belongs to B1's `/elidex-plan-review`, with the coupled
> invariants mapped upfront. §4 is therefore the *named constraint set + coupled
> invariants* B1 must satisfy, presented as B1's input — not as B0's answer.

> **Runtime caveat (read first).** The audit below concerns the **elidex-js VM**
> (`crates/script/elidex-js`) as the canonical Script↔ECS boundary. The
> **production shell still runs boa**: `crates/shell/elidex-shell/src/pipeline.rs`
> constructs `elidex_js_boa::JsRuntime` and `lib.rs` re-exports it; S5/boa removal
> (D-26 PR7) is **not yet done**. The `ConsumerDispatcher` is installed **only** by
> `Vm::bind` (`crates/script/elidex-js/src/vm/vm_api.rs:279`); there is no
> dispatcher install in `elidex-js-boa` or `elidex-shell`. This **dual-runtime
> split** is one of the coupled axes that make §4 a B1 plan-review corner (named in
> §4.1).

> **boa-runtime scoping note (read before any boa-specific claim).** The **boa
> runtime is scheduled for removal in S5 / D-26 PR7** (the production shell runs it
> today, but the canonical Script↔ECS boundary is the elidex-js VM). Per
> `memory/feedback_boa-findings-light-touch`, this audit names boa-specific paths
> only as **known-to-differ**, and does **not** characterize them — that map goes
> moot the moment boa is deleted. **The canonical MutationObserver design (§4, B1)
> targets the post-S5 VM runtime**; the boa-specific path enumeration is **out of
> B1's scope**.

> **Why B0 before B1/B2.** The original audit (F3 in
> `docs/audits/2026-06-elidex-philosophy-implementation-audit.md`) framed the
> problem as "DOM write paths bypass the `ScriptSession` mutation buffer / its
> observers". This doc **re-verifies and corrects that framing by direct code
> read**: direct `EcsDom::set_attribute` is *not* a bypass (it is the canonical
> chokepoint), so the real gap is the JS `MutationObserver` coverage axis, not an
> attribute-bypass axis. B0's job is to establish that factual map (§1–§3) and to
> *name the coupled invariants* the canonical fix must satisfy (§4), **not** to
> pick the mechanism — that is B1's `/elidex-plan-review`. Every site pointer
> carries a `file:line` anchor re-checked at HEAD `26d00c5a`; re-grep at PR-open —
> this doc is a snapshot, and behavior at each anchor is B1's to derive.

---

## 0. TL;DR (findings — conclusions only, behavior is B1's to derive)

- **The original audit's central inference is wrong (F3 reframe).** Direct
  `EcsDom::set_attribute` / `remove_attribute` is **not** an observer/reconciler
  bypass. `EcsDom::set_attribute` (`crates/core/elidex-ecs/src/dom/attribute.rs:101`)
  is the canonical attribute-write chokepoint; tree mutations
  (`crates/core/elidex-ecs/src/dom/tree/mutation.rs`) bottom out at the same
  chokepoint family. **The real gap is the JS-level `MutationObserver` coverage**,
  not an attribute bypass. (Finding conclusion only; the per-API/per-op behavior is
  B1's grep-diff.)
- **Two mechanisms answer two different questions — named, not characterized.**
  - **Mechanism A** = `EcsDom`'s `ConsumerDispatcher` (synchronous, at the
    chokepoint). Role: engine-internal derived-state reconcile **plus** a
    script-visible custom-element-reaction tap. Installed **only** by `Vm::bind`
    (`vm_api.rs:279`) — VM-only / post-S5.
  - **Mechanism B** = `SessionCore`'s mutation buffer + `flush`
    (`session.rs:79`/`:88`). Role: feeds `MutationObserver` **and** CE reactions.
  - `MutationObserver` is **not** a `ConsumerDispatcher` consumer; it is fed via
    `deliver_mutation_records`. **CE reactions are driven from *both* mechanisms** —
    the key cross-mechanism fact B1 must preserve.
- **MutationObserver delivery has two named routes** (both reach
  `deliver_mutation_records`): **buffer-flush delivery** (a Mechanism-B record
  drained by a delivering flush) and **direct-delivery** (a self-generated record
  synchronously delivered). Direct-delivery is a *delivery route*, **not** a third
  mechanism. Which sites take which route, and the per-route conditions, are B1's
  grep-diff (§1 / §6).
- **Canonical-path decision is deferred to B1's `/elidex-plan-review` (§4).** B0
  does **not** prescribe the mechanism. §4 names the coupled invariants (§4.1) and
  record-source constraints (§4.2); it does not characterize their behavior.

---

## 1. VM `vm/host/` DOM Write-Site Map (named invariant + B1 grep-diff methodology)

> **This map is a named invariant + a grep-diff methodology, not a hand-maintained
> registry.** Review history (R1→R5) showed that *hand-enumerating* write sites,
> or characterizing each site's record/event behavior, loses or mis-states sites
> round after round. So this section names the governing invariant and the B1
> grep-diff that derives the exhaustive set and per-site behavior.

**Write-site invariant (named — `non-dispatching-write = MO-silent`).** *A
script-reachable mutation that reaches an `EcsDom` / component mutator is
**MutationObserver-silent** unless it is covered by one of the two named delivery
routes (buffer-flush delivery, or direct-delivery — §0).* This single named
property defines the §3 gap; it holds regardless of whether a site is listed
below. (The `EcsDom` chokepoint independently drives Mechanism A — §2.1 — but
Mechanism A is not a `MutationObserver` source.) The **per-site classification —
which route, if any, covers a given site** — is B1's grep-diff, not a B0 claim.

**B1 grep-diff methodology (the SoT for completeness *and* per-site behavior).**
B1 derives the exhaustive write-site list and each site's coverage by grep-diff:

- *Covered set* = (a) every `SessionCore::record_mutation` call-site **whose flush
  reaches `deliver_mutation_records`** ∪ (b) every **direct
  `deliver_mutation_records` producer**.
- *Mutator set* = every direct `EcsDom`/component-mutator call across the four
  layers: `crates/script/elidex-js/src/vm/host/`, `crates/dom/elidex-dom-api/`,
  `crates/script/elidex-js-boa/`, and the `elidex-ecs` mutators themselves.
- *Gap set* = mutators in neither covered set.

Diffing against `record_mutation` call-sites **alone** mis-classifies both the
direct-delivery producers and the record-but-only-CE-flushed sites — so the
grep-diff must enumerate flush/delivery sites alongside recording call-sites.

**Site pointers (where to look; B1 derives the behavior).** The following are
*locations* a B1 grep-diff must visit. A site's presence here asserts only that it
is a script-reachable DOM-mutating surface; its record/event/route behavior is
**not** stated here — apply the invariant and grep-diff. Each is tagged **bridge**
(dispatch routes through `dom_bridge::invoke_dom_api` → an `elidex-dom-api`
`DomApiHandler`) or **direct** (calls `EcsDom::*` from `vm/host/`); the tag is a
*dispatch-routing* fact only, orthogonal to MO coverage (see §1.5).

- **Attribute API** — `element_attrs.rs`, `attr_proto.rs`, `named_node_map.rs`:
  `setAttribute` **bridge** (`:218`); `removeAttribute`, `Attr.value=`,
  `setNamedItem`/`removeNamedItem`, `setAttributeNode`/`removeAttributeNode`,
  `toggleAttribute` **direct**.
  - **VM-local detach asymmetry (B2 constraint, §4.5):** `attr_remove`
    (`element_attrs.rs:155-187`) and the bridge `RemoveAttribute` handler
    (`element/props.rs:108-122`) differ in what they invalidate (B1/B2 derives the
    exact difference and its consequence).
- **Reflected IDL setters** — `html_input_proto.rs`, `html_button_proto.rs`,
  `html_select_proto.rs`, `html_textarea_proto.rs`, `html_form_proto.rs`,
  `html_element_proto.rs`, `html_iframe_proto.rs`, `canvas/mod.rs:780`
  (content-attribute reflections, HTML §2.6.1; verified via webref).
  - **`HTMLInputElement.value` is the 8kHF exception:** its setter
    (`html_input_value.rs:120-129`) is a value-mode dispatch, **not** a content-
    attribute reflection. B1/B2 must derive its per-mode write path and must not
    place it on an attribute/MO seam unconditionally (§3, §4.5).
- **Tree mutations** — `appendChild` **bridge** (`node_proto.rs:709`);
  `parentnode.rs`, `childnode.rs`, `element_insert_adjacent.rs`,
  `html_select_proto.rs` tree ops **direct**. The VM installs only
  `insertAdjacentElement`/`insertAdjacentText` (`well_known.rs:341-342`), **not**
  `insertAdjacentHTML` (HTML-parsing variant lives only as a dom-api handler).
- **Range mutations** — `range_proto_mutation.rs` (`deleteContents` /
  `extractContents` / `insertNode`; B1's test matrix must include
  `observe(parent,{childList:true})` + a Range mutation).
- **CharacterData splice + `normalize`** — `character_data_proto.rs`,
  `node_methods_extras.rs` (`data=`/`appendData`/`insertData`/`deleteData`/
  `replaceData` route **bridge** → `char_data_handlers.rs`; `normalize` **bridge**).
- **textContent / nodeValue / splitText** — `node_methods/text_content.rs`,
  `text_proto.rs` (R5 grep-diff catches; B1 derives each branch's class — see the
  §1.6 named corners).

### 1.5 The bridge does NOT mean the session buffer

Bridge-routing is **orthogonal** to MutationObserver coverage: it is a dispatch-
routing fact, not a record/buffer fact. **The bridge handler write path — whether a
given handler collapses to the `EcsDom::set_attribute` chokepoint, or instead
buffers a `Mutation` record (a Mechanism-B producer) — is B1's grep-diff**; B0 does
not blanket-assert that all bridge handlers collapse to the chokepoint (some are
buffered-record producers). Likewise which handlers record a `Mutation` and which do
not is B1's grep-diff — `bridge ≠ observable` and `direct ≠ unobservable`. (The
`elidex-dom-api` handlers themselves annotate their recording behavior, e.g.
`child_node/mutations.rs`; B1 reads those annotations directly.)

### 1.6 Named invariant corners (B1 derives the behavior)

Two named corners the write-site invariant subsumes — kept because they correct
real mis-conflations, *named only*, not characterized:

- **`textContent =` vs `nodeValue =` are different mutation classes (named:
  `textContent/nodeValue mutation-class distinction`).** Both in
  `node_methods/text_content.rs` (`SetTextContentNodeKind` `:105-116` /
  `SetNodeValue` `:153-155`). Per WHATWG DOM §4.4 (`#dom-node-nodevalue`) and
  §4.9, the two have different mutation classes by host kind; B1 derives each
  branch's class from the spec + code and must not conflate them.
- **CommentData characterData notification corner (named: `comment-characterData
  notification`).** `set_char_data` (`char_data_handlers.rs:45`, Comment branch
  `:59-73`) and the Text/CDATASection branch differ in notification; B1 derives the
  per-branch behavior and its coupling with live-range adjustment (§4.3 / the
  characterData coupled invariant).

---

## 2. The Two Notification Mechanisms (named roles; behavior is B1's)

There are exactly two **mechanisms** — **Mechanism A** (`EcsDom`
`ConsumerDispatcher`) and **Mechanism B** (`SessionCore` buffer + `flush`). **Read
them as answering two different questions, not as two competing canonical write
paths.** §2 names the mechanisms and their roles; it does **not** prescribe a
mechanism, and does **not** characterize per-op / per-path behavior — that is B1's
grep-diff (§6).

> **Mechanism count vs MO delivery routes — two distinct axes.** "Two mechanisms"
> is the count of write/notification **machines**. `MutationObserver` *delivery*,
> separately, has **two named routes** (§0): buffer-flush delivery and
> direct-delivery, both reaching `deliver_mutation_records`. Direct-delivery is a
> *delivery route*, not a third mechanism. The §2.3 overlap labels combine a
> mechanism with a delivery route; they do not assert a third mechanism.

### 2.1 Mechanism A — `EcsDom` `ConsumerDispatcher` (synchronous, at the chokepoint; VM-only today)

**Mechanism-A fan-out invariant (named).** A Mechanism-A consumer fans out for a
given mutation **iff both** hold: **(a)** the `EcsDom` primitive fires the
`MutationEvent` for that mutation, **and (b)** a `ConsumerDispatcher` is installed.
Whether (a) holds for a given mutation (anchors: tree-mutation shadow-root
fire-site gate `tree/mutation.rs:289`/`:343`; the `set_char_data` / `nodeValue=`
branch corners, §1.6) and whether (b) holds (installed **only** by `Vm::bind`,
`vm_api.rs:279`; boa installs none) are **conditions B1 evaluates per case** — B0
names the invariant, not its per-case truth value. The §2.3 overlap is *derived
from this invariant*, not a hand registry.

**Consumers (named roles only).** The dispatcher drives 7 consumers in
field/dispatch order (`consumer_dispatcher.rs:141-147`): `LiveRangeBridge`
(DOM §5.5), `NodeIteratorAdjuster` (§6.1), `BaseUrlMaintainer` (HTML §2.4.3),
`FormControlReconciler` (§4.10.18.3), `EventHandlerAttributeConsumer` (§8.1.8.1),
`CanvasReconciler` (§4.12.5), `CustomElementReactionConsumer` (§4.13.6). The first
six are **derived-state reconcilers** (engine-internal, several feeding
script-readable state); the seventh, `CustomElementReactionConsumer`, is a
**script-visible CE-reaction tap**. So Mechanism A is **mostly** engine-internal
reconcile **plus** one script-visible tap. `MutationObserver` is **not** a
consumer.

**Trigger / plumbing (site pointers only).** The `MutationEvent` enum
(`mutation_event.rs`) carries seven variants; `EcsDom::dispatch_event`
(`dom/mod.rs:191`) drives the single installed `Box<dyn MutationDispatcher>`
synchronously with a re-entry guard. The inline reconcile + `rev_version` are baked
into the primitives (both runtimes); the consumer fan-out runs only under the VM.
boa drives CE reactions through a **separate** wiring (`pipeline.rs` / `ce.rs`)
that B1 must show covers the same reactions (known-to-differ, moot at S5).

### 2.2 Mechanism B — `SessionCore` mutation buffer + `flush`

- **API (site pointers):** `SessionCore::record_mutation(Mutation)` buffers into
  `pending`; `flush(dom)` applies via `apply_mutation` and returns
  `Vec<Option<MutationRecord>>` (`session.rs:79`/`:88`).
- **Writers of `pending` (site pointers):** the innerHTML-class handlers
  (`elidex-dom-api/element/tree.rs:416`/`:476`) **plus the boa `<iframe>` attribute
  setters** (`elidex-js-boa/globals/iframe.rs`, via `apply_set_attribute`/
  `apply_remove_attribute`, `mutation/mod.rs:288-332`). The boa iframe path is the
  **one existing attribute write that does not go through `EcsDom::set_attribute`**
  — a named constraint §4 weighs (C4). (boa-specific, known-to-differ, moot at S5;
  retained only as the one existing example of a buffered attribute write.)
- **Flush drivers (named: `not every flush delivers to MO`).** The two flush paths
  differ in whether they deliver to `MutationObserver`: the per-frame `re_render`
  path (`content/mod.rs:258`) vs `flush_with_ce_reactions` (`pipeline.rs:25-34`).
  B1 derives which path delivers and must cover both in its delivery wiring + tests
  (a record drained only by the non-delivering path is MO-silent despite
  recording).
- **Consumers — `MutationObserver` *and* CE reactions (named: `Mechanism B is not
  MO-only`).** Flush records feed `MutationObserver`
  (`deliver_mutation_records` → `MutationObserverRegistry::notify`, DOM §4.3.2) and
  **custom-element reactions** (`enqueue_ce_reactions_from_mutations`,
  `ce.rs:137-145`). So CE reactions are driven from *both* mechanisms; B1's
  record-production/delivery changes must preserve CE-reaction semantics.
  - *(boa CE-reaction sourcing involves two systems — flush-record scan +
    binding-direct enqueue in `globals/element/core.rs` — named at known-to-differ
    level only, moot at S5. B1-relevant shape: a record-production change must not
    double-enqueue or miss CE reactions.)*
- **No existing flush→MO microtask drain hook (named: `missing flush→MO hook`).**
  The `Microtask::NotifyMutationObservers` variant (`natives_promise.rs:51-59`)
  exists, but the `MutationObserver`-callback delivery is embedder-driven, not a VM
  microtask (drain arm `:342`). So any seam-fed mechanism (§4) needs **new**
  flush→MO wiring; the drain *point* (`flush`) exists, the *hook* does not.

### 2.3 Overlap (derived from the §2.1 fan-out invariant)

The two mechanisms intersect on the **HTML-fragment write family**. **Which
fragment-write surfaces are members — and the per-member overlap classification**
(which mechanism + which MO-delivery route applies to a light-tree fragment write
vs a shadow-root fragment write vs a boa fragment write) **is B1's grep-diff
derivation** — B0 names the family and the governing §2.1 fan-out invariant, not
its members' behavior (e.g. the fan-out classification of a fragment-write surface
not installed in the VM is moot — §3 `insertAdjacentHTML` install note). Likewise, the named corners where
the blanket "every non-fragment write fires Mechanism A" does **not** hold (the
Comment characterData corner §1.6; the boa buffered `<iframe>` write; the
value-mode value-attr migration `value_mode.rs:222`) are *named* here as
exceptions B1 must evaluate — their record/event behavior is B1's to derive.

---

## 3. The MutationObserver Coverage Gap (named finding; per-API behavior is B1's)

The JS-level `MutationObserver` (WHATWG DOM §4.3) observes a mutation **iff** it is
covered by one of the two named delivery routes (§0) — the §1
`non-dispatching-write = MO-silent` invariant restated for the observer axis.
**`record_mutation` is NOT, by itself, equivalent to observation** (the
delivering-flush qualifier, §2.2).

**Gap finding (conclusion only).** With the present wiring, the JS DOM-mutation
surface enumerated in §1 is **largely MutationObserver-silent**: only a subset
(the HTML-fragment direct-delivery producers + the flush-delivered handler
variants) reaches `deliver_mutation_records`. **Which exact sites are covered vs
gapped — and each site's mutation class — is B1's grep-diff** (the §1/§6
methodology); B0 does not enumerate a per-API gap table, because per-API
characterization in a B0 audit is a finding generator (review history R1→R5). The
gap is **uniform across bridge and direct paths** — *not* a bridge-vs-direct
distinction (correcting the original framing). The existing MutationObserver tests
build `SessionRecord`s by hand and call `deliver_mutation_records` directly; **none
asserts a JS-level mutation yields a record**, which is why the gap was
test-invisible.

> **`insertAdjacentHTML` VM-install note (site fact, not behavior).**
> `insertAdjacentHTML` is **not** installed in the elidex-js VM
> (`well_known.rs:341-342` installs only
> `insertAdjacentElement`/`insertAdjacentText`, #367); it exists only as a dom-api
> handler. (Site fact; its record behavior is B1's grep-diff.)

### 3.1 Record-shape correctness still owed (named, for B1)

Even where records *are* produced, B1 must ensure full DOM §4.3.3 record shape
across the newly-covered kinds (named: `record-shape & coalescing` invariant) —
`oldValue`, `attributeName`, `attributeFilter` gating,
`addedNodes`/`removedNodes`/`previousSibling`/`nextSibling`. `attributeNamespace`
is deferred to `#11-mutation-observer-extras` (`mutation_event.rs:295-298`).

---

## 4. Canonical Path — open design question for B1 (named constraints + coupled invariants)

> **This section does NOT prescribe a mechanism and does NOT characterize
> behavior.** Per the §4 status callout, the canonical MutationObserver-record path
> is an **edge-dense coupled-invariant corner**; under `CLAUDE.md` "Edge-dense work
> = multi-PR + 実装前 plan-review 必須" that choice is **B1's `/elidex-plan-review`
> judgment**. What follows is the *named input* B1 must satisfy: the three coupled
> invariants (§4.1) and the record-source constraints (§4.2). The exact per-op
> event sequence, per-target record breakdown, and per-case behavior are B1's
> grep-diff / dispatch-path derivation, **not** stated here.

### 4.1 The three coupled invariants any candidate mechanism must satisfy (named)

Satisfying one naively tensions another; B1 must satisfy **all three
simultaneously**:

1. **`synchronous-apply` (read-your-writes).** A DOM write must be readable within
   the same script task, but `record_mutation` (`session.rs:78-90`) only buffers
   and applies at `flush`. So MO-record buffering and DOM-write application cannot
   be the same deferred step.
2. **`ConsumerDispatcher` fan-out preservation.** The buffered `apply_set_attribute`
   (`mutation/mod.rs:288-313`) does **not** call `dispatch_event` (it explicitly
   bypasses `EcsDom::set_attribute`, `:299-300`). Routing every write through it
   loses the consumer fan-out (base-url / form-control / event-handler / canvas /
   CE).
3. **`ScriptSession-seam-ownership` mandate.** Per `CLAUDE.md` "ScriptSession as the
   sole Script↔ECS boundary … MutationObserver … を同一機構で守る", MutationObserver
   visibility is a seam-and-flush responsibility — making `MutationObserver` a
   `ConsumerDispatcher` consumer inside the engine-internal `EcsDom` layer inverts
   the mandate.

**Why these are genuinely coupled (named tension, not characterized).** A
seam-only design tensions invariants 1+2; a dispatcher-consumer design tensions
invariant 3 + record shape. **Neither pole is correct as-is.** The resolving
structure plausibly separates *where the write applies* (chokepoint, invariants
1+2) from *where the MO record originates* (seam, invariant 3) — but which
structure, and its exact behavior, is the B1 design judgment.

> **#181 tension with naive seam-only routing (named).** The buffered
> `apply_set_attribute` bypasses the `EcsDom::set_attribute` chokepoint that lesson
> #181 consolidated writes onto (invariant 2). The non-equivalence between the
> chokepoint-honouring immediate `SetAttribute` handler (`element/props.rs:43`) and
> the buffered seam path is the heart of the coupled corner; B1 derives each path's
> record/fan-out behavior from the code and resolves the corner.

### 4.2 Record-source constraints the canonical path must satisfy (named invariants; B1 derives behavior)

Whatever mechanism B1 chooses, its MO-record source must satisfy these **named**
invariants. Each is named with its spec/code resolution pointer; the **exact
per-op event sequence / per-target record breakdown / per-case behavior is B1's
grep-diff / dispatch-path derivation** — B0 does not characterize it.

- **`record-shape & coalescing` invariant.** A remove-all-then-insert–shaped
  operation forms a single coalesced "replace all" record governed by WHATWG DOM
  §4.2.3 "replace all" (`#concept-node-replace-all`). **The exact set of ops that
  bottom out in replace-all is B1's grep-diff** — B0 does not name which APIs do;
  membership is API/node-kind/branch dependent. B1 derives the coalesced-record
  shape, the per-target-parent folding, and the no-op-empty case from the spec +
  the `mutation/mod.rs` apply_* record builders. (Code anchors a B1 grep-diff
  visits: the replace-all loops, e.g. `parentnode.rs` + `text_content.rs:105-116`.)
- **`move-record` / `CE-reaction timing-ordering` / `shadow-root boundary` —
  coupled invariants (behavior intentionally not characterized).** The MO record
  shape for node *moves* (already-parented insertions), the CE-reaction
  timing/ordering/state across moves, and the effect of shadow-root boundaries on
  both MO delivery and CE callbacks are tightly-coupled, algorithm- and
  node-position-dependent invariants. **B0 deliberately does not characterize their
  behavior** — B1's `/elidex-plan-review` derives the exact per-case semantics from
  WHATWG DOM §4.2.3 (insert / `suppressObservers`), HTML §4.13.6 (custom element
  reactions), and the code (`mutation/mod.rs` apply_* builders, `consumer.rs` CE
  handlers, `tree/mutation.rs` shadow-root fire-site gate). Treating any as a
  settled B0 rule is out of scope.
- **`non-dispatching attribute write` invariant.** `set_attribute_without_dispatch`
  (`attribute.rs:146`), used by the value-mode type-change migration
  (`apply_type_change_value_migration`, `value_mode.rs:222`), is a real content-
  attribute write that WHATWG DOM §4.9 "handle attribute changes"
  (`#handle-attribute-changes`) governs (webref-verified). B1 derives which
  observers are owed a record from this write, and the boundary against the
  text-like-mode live-value write (8kHF) that must stay record-free.
- **`shadow-root boundary` (record source vs dispatcher gate) — named.** The
  `tree/mutation.rs` shadow-root fire-site gate couples with the choice of record
  source (event-driven vs upstream-of-dispatcher); B1 derives the per-case behavior
  and how the §4.3.2 inclusive-ancestor walk gates delivery.
- **`boa buffered iframe write` (named precedent, not a model).** `iframe.rs`
  records `Mutation::SetAttribute`/`RemoveAttribute` via the buffered applier that
  bypasses `EcsDom::set_attribute` (invariant 2). B1 resolves which way convergence
  runs. (boa-specific, moot at S5.)
- **`characterData oldValue capture-timing` invariant.** `{characterDataOldValue:
  true}` (DOM §4.3.3) needs the pre-write data, but `EcsDom::set_text_data`
  overwrites before notifying (`dom/mod.rs:336`/`:340-344`). B1 derives where the
  old value must be captured relative to the `EcsDom` write.

### 4.3 Cross-cutting work any direction inherits (named, for B1's plan)

Independent of which mechanism B1 picks:

- **`record-shape & coalescing`.** The coalesced single-record shape is the
  sharpest discriminator between the poles (§4.2); B1 derives the shape.
- **No double-delivery across the direct-delivery producer set.** The
  direct-delivery producers (innerHTML / outerHTML / `setHTMLUnsafe`; exhaustive
  set = §1/§6 grep-diff) each self-deliver via `Vm::deliver_mutation_records`
  through **separate** helpers (anchors `dom_inner_html.rs:148` /
  `native_element_set_outer_html` `:362`). If B1 adds a flush→MO path it must
  retire/reconcile direct delivery across **every** member, or a member
  double-delivers.
- **`CE-reaction preservation` (Mechanism B is not MO-only).** Any change to record
  production / coalescing / delivery ordering must preserve the CE-reaction scan
  across both flush paths (§2.2). The exact CE timing/ordering/state across moves is
  part of the §4.2 coupled-invariant set (B1 derives it).
- **`characterData + live-range` coupling — one coupled invariant.** Per WHATWG DOM
  §4.10 CharacterData "replace data" (`#concept-cd-replace`), the same algorithm
  that queues the `"characterData"` record also adjusts live ranges whose boundary
  is inside the spliced node (over §5.5 "live range"). So the record and the
  live-range boundary adjustment are **one coupled invariant** B1 satisfies together
  for all character-data splices (Comment *and* Text/CData). (Today these ride the
  dispatcher path differently per host kind — §1.6; B1 derives the per-kind
  behavior.)
- **`oldValue` threading.** `characterDataOldValue` / attribute `oldValue` need the
  pre-write value captured before the `EcsDom` write (the characterData
  capture-*timing* invariant above). `attributeNamespace` stays deferred to
  `#11-mutation-observer-extras`.
- **`dual-runtime delivery`.** Both VM and boa flush through `SessionCore`, so a
  seam-side record is runtime-uniform; a flush→MO hook must exist in the boa flush
  path until S5. A dispatcher-consumer is VM-only (dispatcher installed only by
  `Vm::bind`), a larger S5 coupling B1 weighs.

### 4.4 Candidate directions B1 weighs (neither pre-decided here)

B0 enumerates the design space without picking; §4.1 already named that **neither
pole is correct as-is**, so the answer is likely a structure separating *where the
write applies* from *where the MO record originates*:

- **Pole A — `MutationObserver` as a `ConsumerDispatcher` consumer.** *Satisfies*
  invariants 1+2 (synchronous, at the chokepoint, rides the fan-out). *Tensions*
  invariant 3 (script-observable responsibility in the engine-internal layer),
  `record-shape & coalescing`, and the §4.2 coverage invariants
  (non-dispatching `value` write, characterData-oldValue timing, shadow-root gate).
- **Pole B — ScriptSession seam owns MO record production.** Every script-visible
  mutation records a `Mutation` (via `elidex-dom-api`/`DomApiHandler`, keeping
  `vm/host/` marshalling-only), MO drains at `flush`. *Satisfies* invariant 3 and
  the coalescing shape by construction. *Tensions* invariant 1 (naive apply-at-flush
  defers the write), invariant 2 / #181 (buffered chokepoint-bypassing path loses
  fan-out), blast radius (every §1 site must record), and the `missing flush→MO
  hook` (must cover both flush paths, §2.2).

A satisfying mechanism plausibly records the MO entry at the seam (invariant 3 +
correct shapes) while keeping the synchronous write + dispatcher fan-out at the
chokepoint (invariants 1+2). Whether that, a flush-coalescing layer, or another
structure is correct — and how to thread it through the dom-api handlers, the
reflected setters (§4.5), the dual runtime, and the §4.2 constraints — is the B1
design judgment. B0 deliberately stops short of choosing.

### 4.5 B2 — `reflected-IDL-setter recording` + attribute-write convergence (named)

§1 named a uniformity gap B1's mechanism choice will shape: `setAttribute` routes
through a `DomApiHandler` while `removeAttribute` and the reflected IDL setters do
not.

**The open B2 question is *where/how*, not *whether* (named:
`reflected-IDL-setter recording`).** A *true* reflected IDL setter (`a.href`,
`form.method`, `input.type`, …) is an observable attribute mutation per HTML §2.6.1
→ WHATWG DOM §4.9 "handle attribute changes" (webref-verified): the owed
`"attributes"` record is **spec-settled, not a per-plan option**. The **sole
exception is `input.value`** (8kHF): a value-mode dispatch, not a content-attribute
reflection, which must not emit a spurious attribute record. So B2 is **gated on
B1**: which layer/mechanism records the spec-owed reflected-write record, per
§4.1's invariants.

**B2 convergence scope = the *whole* direct-`EcsDom` attribute-write host surface**
— `{removeAttribute`, reflected IDL setters, `toggleAttribute`, `Attr.value`,
`setNamedItem`, `removeNamedItem`, `setAttributeNode`, `removeAttributeNode}`.
**Each has its own conditional write path**, so B2 confirms each API's actual write
path per-API — a B1/B2 grep-diff / dispatch-path derivation, not assumed uniform.

**VM-local Attr-detach precondition (named, §4.5 / B2).** Routing `removeAttribute`
(and the other attribute-removing APIs) through the bridge is **not a pure dispatch
move**: `attr_remove` (`element_attrs.rs:180-187`) and the bridge `RemoveAttribute`
handler (`element/props.rs:108-122`) differ in their invalidation, so B2 must carry
the VM-local Attr-wrapper detach forward after any symmetry move (B1/B2 derives the
exact mechanism). The Layering mandate (`vm/host/` marshalling-only) applies
throughout.

### 4.6 Sequencing

**B1's `/elidex-plan-review` resolves the §4.1 coupled-invariant corner and picks
the mechanism** (close the §3 gap + the §4.2 constraints + the
Range/normalize/Comment/live-range coupling) **before** B2 (the §4.5 convergence),
since B2's target shape depends on B1's choice. Both are `/elidex-plan-review`-gated
per `CLAUDE.md` "Edge-dense work = multi-PR + 実装前 plan-review 必須"; whether B2 is
a separate slice or the write-site half of B1 is itself a plan-review outcome.

---

## 5. Spec / Design SSoT Cross-Reference

- **WHATWG DOM §4.3** — MutationObserver; §4.3.2 "queue a mutation record"; §4.3.3
  Interface MutationRecord (record shape).
- **`docs/design/ja/12-dom-cssom.md`** — §12.1.1 read-only `&EcsDom`, "書き込みは
  `session.record_mutation()`経由"; §12.1.2 "MutationObserver … セッション flushが…
  MutationRecordsを生成。ファーストクラス". This is the design *aspiration* B1
  reconciles against the §4.1 named invariants. B0 does **not** declare it satisfied
  or stale: §12 names a seam-recorded MO path but does not by itself resolve its
  coexistence with `synchronous-apply` (invariant 1) and the chokepoint fan-out
  (invariant 2 / #181) — B1's plan-review.
- **`docs/design/ja/28-adr.md`** — ADR #17 (`ScriptSession` = unified Script↔ECS
  boundary, Mutation Buffer + "consistent MutationObserver records" in 単一メカニズム)
  — the **SSoT for the boundary's existence**: MO visibility belongs on the seam
  (invariant 3). It establishes *that* the seam owns MO records, not the *mechanism*
  by which every write reaches the seam while preserving invariants 1+2 — the §4
  open question. ADR #14 ("MutationObserver がECS変更検出に自然にマッピング")
  describes the implementation substrate, not a license to put MO production in the
  `EcsDom` layer.
- **`CLAUDE.md`** — "ScriptSession as the sole Script↔ECS boundary … MutationObserver
  … を同一機構で守る" (invariant 3); **"Edge-dense work = multi-PR + 実装前
  plan-review 必須"** (the rule making the §4 mechanism choice a B1 judgment); "One
  issue, one way"; Layering mandate (`vm/host/` marshalling-only — §4.5).
- **lesson #181** (`attribute.rs:5-15`, `element/props.rs:61`) — the canonical
  `EcsDom::set_attribute` write-path consolidation, **named as in tension with
  naive seam-only routing** (§4.1 callout): the buffered `apply_set_attribute`
  bypasses the chokepoint, so routing writes through it re-forks what #181
  collapsed (invariant 2). Keeping #181 intact while still producing a seam-side MO
  record (invariant 3) is the corner B1 resolves.

---

## 6. Re-check Discipline (for B1/B2 plan-memos)

- Re-grep every `file:line` here at PR-open — line numbers will drift.
- **Produce the exhaustive write-site set *and each site's behavior* by grep-diff,
  not by extending §1's site pointers.** Covered set = (a) every `record_mutation`
  call-site **whose flush reaches `deliver_mutation_records`** (per-frame
  `re_render` → `content/mod.rs:258`, vs the CE-reaction `flush_with_ce_reactions`)
  ∪ (b) every **direct `deliver_mutation_records` producer**. Then enumerate every
  direct `EcsDom`/component-mutator call across `vm/host/`, `elidex-dom-api`,
  `elidex-js-boa`, and `elidex-ecs`, and diff. Diffing against `record_mutation`
  alone mis-classifies the direct-delivery producers and the record-but-only-CE-
  flushed sites.
- Re-confirm the §2 mechanism map by direct read of `attribute.rs`
  (`set_attribute`/`dispatch_event`), `tree/mutation.rs` (`Insert`/`Remove` fire
  sites + shadow-root fire-site gate), `consumer_dispatcher.rs` (consumer list),
  `mutation_observer.rs` (`deliver_mutation_records`), and the dispatcher-install
  asymmetry (only `Vm::bind` `vm_api.rs:279`, no boa install). B1 derives the §2.3
  per-member behavior from these reads; do not carry a behavioral characterization
  forward on trust.
- Re-confirm the two boa CE-reaction producers (flush-record scan
  `enqueue_ce_reactions_from_mutations` + binding-direct enqueue in
  `globals/element/core.rs`), so a record-production change does not double-enqueue
  or miss CE reactions.
- Re-confirm the §1.6 named corners by direct read: `text_content.rs:105-116`
  (`SetTextContentNodeKind`) vs `:153-155` (`SetNodeValue`, DOM §4.4
  `#dom-node-nodevalue`); the Comment `set_char_data` branch
  (`char_data_handlers.rs:59-73`). B1 derives each branch's mutation class.
- Re-confirm the §4 coupled-invariant anchors by direct read (behavior is B1's to
  derive from spec + code): `record_mutation` deferred-apply (`session.rs:78-90`);
  `apply_set_attribute` chokepoint-bypass (`mutation/mod.rs:288-313` — invariant 2 /
  #181); `apply_set_inner_html` (`html_fragment.rs:85-89`); the `missing flush→MO
  hook` (`natives_promise.rs:333-344`); the `input.value` value-mode dispatch
  (`html_input_value.rs:120-129` — 8kHF); the reflected-setter writes; the
  replace-all loops (`parentnode.rs` `replaceChildren` [parent-kind gate `:75`] +
  `text_content.rs:105-116` — DOM §4.2.3 `#concept-node-replace-all`); the
  **`move-record` / `CE-reaction timing-ordering` / `shadow-root boundary` coupled
  invariants** (anchors `apply_append_child`/`apply_insert_before`
  `mutation/mod.rs:212`/`:232`, `consumer.rs` CE handlers, `tree/mutation.rs`
  shadow-root fire-site gate) — **behavior B0 deliberately does not characterize**;
  B1 derives the per-case MO record shape, CE timing/ordering/state, and
  shadow-boundary effect from DOM §4.2.3 `suppressObservers` + HTML §4.13.6 + the
  code; and the `characterData oldValue capture-timing` anchor
  (`set_text_data` `dom/mod.rs:336`/`:340-344`).
- Re-check active branches (`git branch -r`) for convergence drift on
  `element_attrs.rs` / `vm/host/` attribute setters (MED collision risk with JS-side
  work; B is later — Axis 5).
- Slot check: `#11-mutation-observer-extras` (attributeNamespace, primitive ToObject
  for `observe`) must still be open before referencing it.

## Review guidelines (for Codex)

- This is a **doc-only, B0-altitude** audit. It deliberately states **named
  invariants + site pointers + B1 grep-diff methodology** and **does NOT
  characterize behavior** (no per-write site, per-op event sequence, per-API record
  shape, per-path delivery, or per-case behavior). Review history (R1→R5) showed
  that *any* behavioral claim in a B0 audit is a finding generator, because each
  carries its own edge-case combinatorial space — that derivation is B1's
  plan-review grep-diff job.
- Verify the `file:line` **anchors** against `main`, and challenge any **named
  invariant or finding conclusion** (§0/§3 the MO gap + F3 reframe + the
  `input.value` 8kHF non-reflection; §4.1 the three coupled invariants; §4.2 the
  named record-source invariants) that **mis-states the invariant** or whose anchor
  **does not match the code**.
- Do **NOT** flag for "missing site/op/API X", "incomplete per-op event sequence",
  "unspecified per-case (move/shadow/same-vs-cross-parent) behavior", or a "missing
  per-API gap table" — those are **deliberately B1's grep-diff**, not B0 registry
  defects. A genuinely *mis-named/mis-attributed* invariant, or an anchor that
  contradicts the code, **is** in scope.
- **§4 is deliberately not a prescribed fix** (edge-dense corner; `CLAUDE.md`
  reserves the mechanism choice for B1's `/elidex-plan-review`). Do **not** flag §4
  for "failing to pick a mechanism". **Do** flag if (a) a §4.1 coupled invariant is
  mis-named/mis-attributed, (b) a §4.2 named record-source invariant is wrong, (c)
  the §4.4 Pole-A/Pole-B trade-off mis-names the code, or (d) the #181 /
  `apply_set_attribute`-bypass tension (§4.1 callout, §5) is mis-read.
- Out of scope: implementing B1/B2; touching `element_attrs.rs`, reflected IDL
  setters, `range_proto_mutation.rs`, `char_data` handlers, or `ConsumerDispatcher`.
