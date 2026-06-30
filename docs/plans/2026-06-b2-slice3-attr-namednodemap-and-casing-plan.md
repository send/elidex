# B2-Slice-3 — Attr / NamedNodeMap attribute MutationRecords + whole-surface attribute-name casing fold

> Program B (F3 mutation-path) · the **LAST attribute record gap**. After this slice, attribute-write
> MutationObserver records are complete (modulo the 3 deferred off-record-path slots). Closes
> defer-slot `#11-attribute-name-html-namespace-casing`.
>
> Anchor = first-principles ideal (not a surgical patch over the current inline impls). Edge-dense
> (≥3 intersecting invariant axes) → `/elidex-plan-review`-gated BEFORE implementation, per CLAUDE.md
> "Edge-dense work = multi-PR program + 実装前 plan-review 必須".

---

## §0. Status / lineage

- **Base**: `origin/main` `71ac54bb` (#432). Worktree `b2-slice3`.
- **Predecessors** (the seam this slice extends):
  - #428 B2-Slice-1 `d09829a5` — generic `setAttribute`/`removeAttribute`/`toggleAttribute` → §4.9
    "attributes" records via `apply_set_attribute`/`apply_remove_attribute`. Carved this casing slot
    (R3→R4 strangler-trap: a partial HTML-namespace gate desynced → reverted to uniform-lowercase
    baseline + carved the WHOLE casing surface here).
  - #431 B2-Slice-2 `ab0cefae` — reflected IDL setters + classList/dataset/style/hyperlink via the
    `attr_set`/`attr_remove` shims → `commit_notify_records`.
- **This slice (B2-Slice-3, unified PR)**: route the remaining **node-identity** attribute mutators —
  `Attr.value` setter, `Element.setAttributeNode`/`removeAttributeNode`, `NamedNodeMap.setNamedItem`/
  `removeNamedItem` — through the same record-producing `apply_*` seam, converge their ad-hoc
  Attr-wrapper bookkeeping onto the shared `snapshot_attr_wrapper`/`freeze_detached_attr_wrapper`
  helpers, AND fold the whole attribute-name casing surface onto ONE canonical `is_html_namespace`-aware
  resolver.

### §0.5 Spec citation table (anchors)

- WHATWG DOM §4.9 Interface Element — `#interface-element` (setAttribute / change / set / remove /
  set-an-attribute-value / get-an-attribute-by-name / setAttributeNode / removeAttributeNode)
- WHATWG DOM §4.9.1 Interface NamedNodeMap — `#interface-namednodemap` (setNamedItem / removeNamedItem)
- WHATWG DOM §4.9.2 Interface Attr — `#interface-attr` (value setter = set-an-existing-attribute-value)

### Scope-decomposition decision (recorded for plan-review)

**One unified PR**, not split into casing-prereq + records. Rationale (philosophy, not convenience):
the casing surface and the record reroute **intersect** at the name-based node lookups
(`getAttributeNode`/`getNamedItem`/`removeNamedItem` — each is BOTH a casing site AND an
Attr/NamedNodeMap record/identity site). Splitting along casing|records forces a double-touch at
exactly those sites and fragments each site's change across two PRs. The real cohesion seam is
"attribute name + Attr-node identity", which runs *through* both concerns. The edge-dense mandate's
requirement is **plan-review + upfront edge-matrix mapping** (§2), provided here — not mechanical
PR-count maximization. The base-case rule (CLAUDE.md) sanctions a narrowly-scoped, plan-reviewed
slice as a single terminal PR; "attribute name + Attr identity records" is that scope. The casing
fold is nonetheless implemented as an **atomic whole-surface sub-step** (every name-based site in one
change — never a partial gate; the #428 R3→R4 lesson, One-issue-one-way). Preflight breadth = 1 spec
/ 3 sections → single-PR scope.

---

## §1. The gap (verified, not assumed)

VM-host node-identity attribute mutators call the `EcsDom::set_attribute`/`remove_attribute`
**chokepoint directly**:

| API | VM native | current write call | MO record? |
|-----|-----------|--------------------|-----------|
| `Attr.value` setter (attached) | `attr_proto.rs:379` `native_attr_set_value` | `host.dom().set_attribute(owner, name, val)` `:416` | **NO** |
| `Element.setAttributeNode` | `element_attrs.rs:412` `native_element_set_attribute_node` | `host.dom().set_attribute(entity, name, val)` `:473` | **NO** |
| `Element.removeAttributeNode` | `element_attrs.rs:518` `native_element_remove_attribute_node` | `host.dom().remove_attribute(entity, name)` `:594` | **NO** |
| `NamedNodeMap.setNamedItem` | `named_node_map.rs:273` `native_nnm_set_named_item` | `host.dom().set_attribute(owner, name, val)` `:345` | **NO** |
| `NamedNodeMap.removeNamedItem` | `named_node_map.rs:381` `native_nnm_remove_named_item` | `host.dom().remove_attribute(owner, key)` `:431` | **NO** |

`EcsDom::set_attribute` (`elidex-ecs/src/dom/attribute.rs:125`) dispatches `MutationEvent::AttributeChange`
→ `ConsumerDispatcher` fan-out (Mechanism A: style / CE-tap / Attr-sync / `rev_version`). It does
**not** build the WHATWG DOM §4.9 "attributes" MutationObserver record — that is produced only by
`apply_set_attribute` / `apply_remove_attribute` (`elidex-script-session/src/mutation/mod.rs:830`/`:853`),
which call the same chokepoint *and* build `attribute_record(...)` (`:246`). So these 5 APIs are
**MO-silent today** (Mechanism A fires, Mechanism B missing) — exactly the remaining attribute gap in
[[reference_js-tree-mutations-not-recorded]].

**Duplication to converge (One-issue-one-way)**: the engine-independent dom-api handlers
`char_data/attr.rs` `SetAttrValue` (`:282`), `SetAttributeNode` (`:111`), `RemoveAttributeNode` (`:177`)
are **registered** (`registry.rs:141`/`142`/`145`) but ALSO call `dom.set_attribute`/`remove_attribute`
directly (`attr.rs:318`/`:161`/`:230`) — also MO-silent. The VM natives do not dispatch to them (they
inline). Two parallel MO-silent impls per op; this slice routes BOTH through `apply_*`.

---

## §2. Coupled-invariant enumeration (edge-dense)

Six invariant axes the design must satisfy **simultaneously**, plus each load-bearing pairwise
intersection (one line) and the concrete corner cell it produces:

- **A1 record-production** — `apply_set_attribute`/`apply_remove_attribute` build the §4.9 record at
  the `EcsDom` chokepoint (fan-out preserved).
- **A2 Attr-node identity / liveness** — per-VM `AttrState{owner, qualified_name, detached_value}` +
  `wrapper_store` cache; live wrapper reads owner's `Attributes`, detached reads its snapshot.
- **A3 detach lifecycle** — `snapshot → mutate → freeze → invalidate-cache`, ordering load-bearing.
- **A4 name casing** — HTML-namespace-gated lowercase, applied at **name-based** entry points only.
- **A5 validation contracts** — NotFoundError (removeAttributeNode list-contains, removeNamedItem null)
  thrown *before* any mutation. (set-an-attribute's InUseAttributeError is **NOT** enforced by the VM —
  Phase-2 does a cross-element value-copy, not an owner-retarget; as-built deviation, §4.1 / §6.)
- **A6 NS-variant boundary** — Phase-2 null-namespace simplification (`*NS` reject/pass-through).

Pairwise intersections + corner cells:

- **A1 × A2 (record × identity)** — setAttributeNode replacing an existing same-named attr = "set an
  attribute" step 6 "replace" → handle-attribute-changes ONCE ⇒ **1 change record** (not remove+append).
  `apply_set_attribute` on an existing name = 1 change record. ✓
- **A1 × A5 (record × oldAttr==attr)** — `setAttributeNode(el.getAttributeNode("id"))` on the SAME
  element with the SAME attr → set-an-attribute step 4 "return attr" BEFORE any change ⇒ **NO record**,
  identity preserved. ⚠ **must short-circuit before `apply_set_attribute`** (today's impl re-writes
  unconditionally; with `apply_*` that would wrongly emit a same-value record).
- **A1 × A4 (record × casing)** — the record's `attributeName` must equal the name that landed in
  storage: resolved name for name-based ops, verbatim local name for node-identity ops. `apply_*` is
  name-agnostic; the entry point owns resolution. ✓
- **A1 × A5 (record × absent)** — removeNamedItem("missing") throws NotFoundError *before* apply (no
  record); removeAttributeNode of a not-contained attr throws (no record). `apply_remove_attribute`
  `None`-when-absent (I11) is belt-and-suspenders. ✓
- **A2 × A3 (identity × detach)** — removeAttributeNode freezes **the passed-in object** (identity
  preserved, caller holds it); removeNamedItem allocates a **fresh** detached Attr (caller passed a
  name). Same snapshot/freeze helper, differing only in *which ObjectId* — a parameter, not a
  duplicated path (§4.3).
- **A2 × A4 (identity × casing)** — `getAttributeNode("ID")` on an HTML element resolves "ID"→"id",
  caches the wrapper under `intern("id")` = the storage key (consistent). On an SVG element
  `getAttributeNode("viewBox")` stays "viewBox". ✓
- **A3 × A1 (detach × record ordering)** — `snapshot → apply_*(mutate+record) → freeze → commit(drain)`.
  Freeze (VM wrapper state) and drain (microtask queue) are order-independent, but snapshot→apply→freeze
  is load-bearing (I2/I9, `attr_remove` precedent). ✓
- **A4 × A6 (casing × NS)** — `*NS` variants never lowercase (use the Attr's local name / validated
  qualifiedName); resolver applied to non-NS name-based path only. ✓
- **A4 × node-identity** — setAttributeNode of an Attr named "viewBox" onto an HTML element stores
  "viewBox" verbatim (set-an-attribute uses the attr's local name, NO lowercase). Then
  `getAttribute("viewbox")` lowercases→miss / `getAttribute("viewBox")` hit. Spec-correct; test-locked.
- **A2 × A2 detached `.value` (record-less)** — a detached Attr's `.value =` (createAttribute /
  removed-then-held) sets the snapshot only, no chokepoint, **no record** (set-an-existing-attribute-value
  step 1; negative control, mirrors B2-Slice-2 I1).

Out-of-design (deferred, §6): CE×MO delivery ordering; dialog method-driven removal.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.9 Interface Element | setAttribute step 2 | HTML-ns → lowercase / SVG-MathML → case-preserve | `resolve_attribute_qname` @ props.rs:61 | ✓ | yes (qualifiedName) |
| WHATWG DOM §4.9 Interface Element | get-an-attribute-by-name step 1 | HTML-ns lowercase (get/has/getAttributeNode) | resolver @ props.rs:31 / attrs.rs:31 / element_attrs.rs:386 | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | change-an-attribute steps 1–3 | oldValue capture + handle-changes → record | `apply_set_attribute` | ✓ | yes (value) |
| WHATWG DOM §4.9 Interface Element | append-an-attribute step 4 | fresh attr → record oldValue=null | `apply_set_attribute` (fresh) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | remove-an-attribute step 4 | handle-changes(oldValue→null) → record | `apply_remove_attribute` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | set-an-attribute step 2 | owner ≠ null & ≠ recv → spec InUseAttributeError; **VM deviates** = Phase-2 value-copy (§6), not throw | setAttributeNode/setNamedItem | ✗ (as-built deviation, §6) | yes (Attr arg) |
| WHATWG DOM §4.9 Interface Element | set-an-attribute step 4 | oldAttr == attr → return, NO record | short-circuit before `apply_set_attribute` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | set-an-attribute steps 6–7 | replace (1 record) / append (1 record) | `apply_set_attribute` | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | removeAttributeNode step 1 | list ∌ attr → NotFoundError | `native_element_remove_attribute_node` | ✓ | yes (Attr arg) |
| WHATWG DOM §4.9.1 Interface NamedNodeMap | setNamedItem | = set-an-attribute | `native_nnm_set_named_item` | ✓ | yes |
| WHATWG DOM §4.9.1 Interface NamedNodeMap | removeNamedItem steps 1–2 | remove-by-name (resolver) + null → NotFoundError | `native_nnm_remove_named_item` | ✓ | yes |
| WHATWG DOM §4.9.2 Interface Attr | set-an-existing-attribute-value step 1 | element == null → set snapshot, NO record | `native_attr_set_value` detached arm | ✓ | yes (value) |
| WHATWG DOM §4.9.2 Interface Attr | set-an-existing-attribute-value step 5 | element ≠ null → change-attribute → record | `native_attr_set_value` → `apply_set_attribute` | ✓ | yes (value) |

### §3.1 User-input touch audit

User-controllable input flowing into the touched sites:
- attribute **qualifiedName** (get/set/has/remove/toggle, getAttributeNode, get/removeNamedItem) →
  the casing resolver; **value** (setAttribute, Attr.value, setAttributeNode/setNamedItem source) →
  `apply_set_attribute`; **Attr node arg** (setAttributeNode/removeAttributeNode/set/removeNamedItem)
  → brand-check + NotFound validation (InUseAttributeError not enforced — §6 value-copy deviation).
- **Adjacent pre-existing surface — exposure delta**: applying the resolver to `getAttributeNode` /
  `getNamedItem` / `removeNamedItem` (currently RAW, no lowercase) is a **behavior change** — on HTML
  elements these now HTML-gate-lowercase the lookup name (spec-correcting: `getAttributeNode("ID")`
  begins finding `id`). `native_element_has_attribute` currently bypasses lowercasing entirely (raw
  ECS read) while the dom-api `HasAttribute` lowercases → this slice removes that path-dependence
  (exposure: converges two divergent behaviors to the spec one). No new user-reachable surface beyond
  the 5 in-scope APIs + the casing normalization of existing lookups.

---

## §4. Ideal design

### §4.1 Record reroute — the `attr_set`/`attr_remove` shim pattern (already canonical)

Each VM native becomes a thin marshalling shim around the shared record-producing primitive, mirroring
B2-Slice-2's `attr_set` (`element_attrs.rs:112`) / `attr_remove` (`:212`):
`snapshot wrapper → apply_*(dom) → freeze wrapper → commit_notify_records`. The per-VM Attr-wrapper
identity bookkeeping (ObjectId cache, `AttrState.detached_value`) is **marshalling** — stays VM-side
(per-VM identity handle = CLAUDE.md side-store exception (a), `world_id` not yet landed); the attribute
mutation + record is engine-independent — goes through `apply_*`.

- **`Attr.value` setter** (`native_attr_set_value`, attr_proto.rs:379)
  - detached (`AttrState.detached_value.is_some()`) → update snapshot in place, **no record** (spec
    set-an-existing-attribute-value step 1). Unchanged (`:398`).
  - attached → `apply_set_attribute(host.dom(), owner, &name, &new_value)` + `commit_notify_records`
    (spec step 5 change-attribute → record; same-value still records, I4). Replaces raw `set_attribute`
    at `:416`. Preserve the already-removed re-attach guard (`:410`) + post-unbind no-op (`:407`).
- **`Element.setAttributeNode`** (element_attrs.rs:412) & **`NamedNodeMap.setNamedItem`** (named_node_map.rs:273)
  - spec set-an-attribute. **step 4 short-circuit** (A1×A5 corner): if the resolved oldAttr IS the
    passed attr → return it, no write, no record. Else route through `apply_set_attribute(dom, recv,
    &attr_local_name, &source_value)` + commit. `attr_local_name` = the Attr's stored qualified name
    **verbatim** (NO resolver — §4.2). **As-built deviation (§6)**: the VM does NOT throw
    InUseAttributeError for a cross-element source (step 2) — its per-VM Attr wrapper cannot retarget
    owner, so Phase-2 does a documented cross-element **value-copy** (now record-producing, since it is a
    real write on `recv`); 5 pre-existing tests lock the no-throw value-copy. Spec-strict InUse needs the
    Phase-3 Attr-node-ownership model. Returns the prior Attr (detached) or null. Same-element identity
    preservation preserved.
- **`Element.removeAttributeNode`** (element_attrs.rs:518) & **`NamedNodeMap.removeNamedItem`** (named_node_map.rs:381)
  - spec remove-an-attribute / remove-an-attribute-by-name. Route through `apply_remove_attribute(dom,
    recv, &name)` + commit (records only when removed, I11). removeAttributeNode's `name` = attr's
    stored local name; removeNamedItem's `name` = the **resolved** qualifiedName (§4.2, name-based).
    NotFoundError preserved (removeAttributeNode list-not-contains step 1 / removeNamedItem null step 2).
    The removed Attr frozen via the shared helper (§4.3).
- **dom-api `char_data/attr.rs` handlers** (`SetAttrValue`/`SetAttributeNode`/`RemoveAttributeNode`):
  rewrite the direct `dom.set_attribute`/`remove_attribute` (`:318`/`:161`/`:230`) → `apply_*` +
  `session.push_notify_record` (the `invoke_dom_api` Phase-2.5 drains). One-issue-one-way: the VM shim
  AND the dom-api handler both funnel into the single `apply_*` primitive.
  - **Dual-model note (Axis 5 F4)**: these `char_data/attr.rs` handlers operate on a *distinct*
    entity-backed Attr model (`AttrEntityCache` / `AttrData`), parallel to the VM's per-VM `AttrState`
    wrapper model (§4.3). They are currently **VM-unreachable** (the VM uses the inline natives above,
    not these handlers) and wasm exposes only get/setAttribute — so they are dormant for live traffic.
    The reroute connects them to the canonical `apply_*` seam for engine-independent consistency (any
    future boa/wasm reach is then record-producing by construction), NOT because they are live today.
    The casing fold likewise folds the dormant `char_data/attr.rs` `GetAttributeNode` (§4.2) so the
    entity-backed get-by-name path is not left as a future strangler. The dom-api `SetAttributeNode` got
    the set-an-attribute step-4 oldAttr==attr short-circuit (Codex R1 — via the `AttrEntityCache` identity
    check, matching the VM native; the B2-Slice-3 reroute through the recording `apply_set_attribute` would
    otherwise have emitted a spurious same-value record). **One remaining boa-only-reachable gap**
    (pre-existing, NOT introduced here): `RemoveAttributeNode` does not evict the `AttrEntityCache` — the
    production VM native does; resolves when the VM unifies onto the entity-backed handlers (or at boa
    deletion).

### §4.2 Casing fold — ONE canonical resolver (closes `#11-attribute-name-html-namespace-casing`)

New engine-independent resolver on `EcsDom`, next to `is_html_namespace` (`dom/mod.rs:815`) — the
lowest common denominator reachable by dom-api handlers, VM natives, and `apply_*`:

```rust
// EcsDom::resolve_attribute_qname (NEW)
/// DOM §4.9 get/set/has/remove-an-attribute-by-name step 1: ASCII-lowercase iff `entity` is an
/// HTML-namespace element. SVG/MathML keep case-preserved local names (e.g. `viewBox`).
pub fn resolve_attribute_qname<'a>(&self, entity: Entity, qname: &'a str) -> Cow<'a, str> {
    if self.is_html_namespace(entity) {
        Cow::Owned(qname.to_ascii_lowercase())
    } else {
        Cow::Borrowed(qname)
    }
}
```

- The `Cow` borrows the **argument** `qname`, not `&self` → no conflict with a later `&mut dom`
  (`apply_set_attribute(dom, …)`). `is_html_namespace` short-circuits non-elements to false (→ Borrowed).
- **Spec-faithfulness note (for plan-review Axis 4)**: DOM gates on *"HTML namespace AND node document
  is an HTML document"*. elidex has **no** node-document-type predicate at the ECS layer (verified:
  no `is_html_document`/`DocumentKind`/`content_type` in elidex-ecs / elidex-dom-api) and the
  `Namespace` enum is `{Html, Svg, MathMl}` — "HTML-namespace element in an XML document" is not a
  representable state. So `is_html_namespace` is the **complete** available gate and correctly resolves
  the only representable case (SVG/MathML case-preservation = the slot's actual concern). The
  document-type half is a single bounded *noted* limitation; because there is exactly ONE resolver,
  folding a future `is_html_document(entity)` conjunct is a one-line localized change (One-issue-one-way
  preserved — no re-sweep).
- **Data-flow (verified, for Axis 2)**: the resolver reads the sparse `Namespace` component via
  `namespace_of` (`dom/mod.rs:431`); it is populated **only** by `EcsDom::create_element_ns`
  (`dom/mod.rs:408`, attaches `Namespace` only for non-HTML at `:416`). JS `createElementNS` is **not
  VM-wired** (no native / no registry
  entry — verified), so foreign (SVG/MathML) elements arrive **via the parser** (`convert.rs` →
  `create_element_ns`). The resolver therefore sees `Namespace::Svg`/`MathMl` for parser-sourced foreign
  elements (correct) and the default `Html` for everything else (incl. `createElement("svg")`, which is
  spec-correctly an HTML-namespace element). No new write-path needed.

**Applied at the name-based sites** (whole-surface, atomic — no partial gate):

| site | file:line | change |
|------|-----------|--------|
| `GetAttribute` (dom-api) | `element/props.rs:31` | `to_ascii_lowercase()` → `dom.resolve_attribute_qname(this, &raw)` |
| `SetAttribute` (dom-api) | `element/props.rs:61` | ditto (after `validate_attribute_name`) |
| `RemoveAttribute` (dom-api) | `element/props.rs:112` | ditto |
| `HasAttribute` (dom-api) | `element/attrs.rs:31` | ditto |
| `ToggleAttribute` (dom-api) | `element/attrs.rs:65` | ditto |
| `GetAttributeNode` (dom-api) | `char_data/attr.rs:56` | **fold** (F3) — resolver on BOTH the lookup name AND the `AttrEntityCache` key (get-by-name; the entity-backed parallel to the VM native — §4.1 dual-model note; dormant but folded so it is not a future strangler) |
| `native_element_has_attribute` (VM) | `element_attrs.rs:298` | **bug fix** — converge onto `invoke_dom_api("hasAttribute")` so the VM path matches the dom-api `HasAttribute` (today: raw read → path-dependent) |
| `native_element_remove_attribute` (VM) | `element_attrs.rs:262` | **re-key via resolver** (F1/F2) — replace the VM-side `to_ascii_lowercase()` with `resolve_attribute_qname(entity, raw)` for BOTH the handler-dispatch arg AND the wrapper-snapshot key. *Not* delete-the-lowercase (raw key regresses HTML `removeAttribute("ID")`) nor keep-unconditional (regresses SVG). See §8 I-CACHE-KEY |
| `native_element_toggle_attribute` (VM) | `element_attrs.rs:606` | ditto — re-key via resolver |
| `getAttributeNode` name lookup (VM) | `element_attrs.rs:386` | **add** resolver; wrapper-cache key = `intern(resolved)` (I-CACHE-KEY) |
| `getNamedItem` name lookup (VM) | `named_node_map.rs:244` | **add** resolver; wrapper-cache key = `intern(resolved)` |
| `removeNamedItem` name lookup (VM) | `named_node_map.rs:381` | **add** resolver; the single `intern(resolved)` feeds the get-by-name match, the wrapper-snapshot key, AND the `apply_remove_attribute` key |

(12 name-based sites — enumerated with file:line; verified 2026-06-29 against the three Explore maps +
reads. The cache-key consequence is design-pinned as §8 **I-CACHE-KEY**, not left to impl. Bare
`element/props.rs`·`element/attrs.rs`·`char_data/attr.rs` paths are dom-api crate
[`crates/dom/elidex-dom-api/src/`]; `element_attrs.rs`·`named_node_map.rs` are VM host
[`crates/script/elidex-js/src/vm/host/`].)

**NOT applied — name-based, explicitly excluded (auditable whole-surface)**. A name-based attribute-name
site is excluded ONLY for a spec reason, recorded so the whole-surface claim is auditable:
- dom-api `CreateAttribute` (`char_data/attr.rs:34`) — `createAttribute(localName)` lowercases on
  *"this is an HTML **document**"* (DOM `#dom-document-createattribute` step 2, webref-verified): a
  **document**-gated rule, NOT the namespace-gated one, and the new Attr has **no owner element** so
  `resolve_attribute_qname(entity, …)` does not fit. elidex has no `is_html_document` predicate and the
  handler is VM-unreachable + wasm-unexposed (dormant), so its current unconditional-lowercase stands as
  the HTML-document-assumed baseline. Folds into the same future `is_html_document` work as the §4.2 note.
- VM `try_indexed_get` named-property access (`named_node_map.rs:611`) — `nnm["ID"]` resolves via WebIDL
  **supported property names** (DOM `#interface-namednodemap`, webref-verified): the supported names ARE
  the stored qualified names; step 2 only *removes* non-lowercase names from exposure for HTML-namespace
  elements — it does **not** lowercase the lookup name. So bracket access is **case-sensitive**
  (`nnm["ID"]` ⇒ undefined; `nnm["id"]` ⇒ the Attr); the exact-match (`n == key`) is spec-correct and
  must NOT use the resolver.

**NOT applied — node-identity / verbatim local name per spec**: `setAttributeNode`/`setNamedItem`
(attr's stored name), `removeAttributeNode` (operates on the attr node), `Attr.value` setter,
`Attr.name`/`localName` accessors. All `*NS` variants stay raw (§6). `apply_*` stays **name-agnostic**
(resolution is a property of the name-based entry point, not the primitive — node-identity ops must reach
`apply_*` with a non-resolved name).

### §4.3 Detach-asymmetry resolution — converge onto the shared wrapper helpers (One-issue-one-way)

Today the node-identity natives each hand-roll wrapper detach/freeze/invalidate (removeAttributeNode
freezes the passed-in object at `element_attrs.rs:596`; removeNamedItem allocates a fresh detached Attr
at `named_node_map.rs:450`; set ops return a detached old Attr) — *separately* from the
`snapshot_attr_wrapper` (`element_attrs.rs:153`) / `freeze_detached_attr_wrapper` (`:185`) helpers that
`attr_remove` uses. This divergence is the "VM-local Attr-wrapper detach asymmetry": two mechanisms for
the same concept, with the documented hazard (attr_proto.rs:43–48) that any removal bypassing the
shared helper must remember to call `invalidate_attr_cache_entry` itself.

**Resolution**: the removal natives adopt the `snapshot_attr_wrapper → apply_remove_attribute →
freeze_detached_attr_wrapper` sequence (= the `attr_remove` shape), so there is ONE detach mechanism.
The two return-shape differences (removeAttributeNode freezes the passed-in object identity-preservingly;
removeNamedItem allocates a fresh detached Attr) are spec/identity-mandated and expressed as the *which
ObjectId* parameter to the shared helper — not duplicated code paths. The CRITICAL INVARIANT (every
removal path calls `invalidate_attr_cache_entry`) becomes **structural** (the helper does it) rather than
a per-site reminder. Set ops route the displaced-oldAttr freeze through `freeze_detached_attr_wrapper`
likewise.

---

## §5. Layering + ECS-native check

- **Layering mandate**: the attribute mutation algorithm + record live in engine-independent crates
  (`apply_*` in `elidex-script-session`; the dom-api `attr.rs` handlers; the resolver on `EcsDom`).
  VM host/ keeps ONLY marshalling: brand/Attr-union checks, per-VM ObjectId/`AttrState` wrapper identity,
  JsValue↔Entity. No new algorithm in host/. ✓ (mirrors #399/#402/#428/#431).
- **ECS-native / side-store**: `AttrState` + the wrapper cache stay per-VM HostData — they hold ObjectId
  (per-VM identity handle = CLAUDE.md exception (a), `world_id` not yet landed), so correctly NOT ECS
  components yet (migration = post-S5 `#11-wrapper-identity-component-migration`). No new side-store.
- **Namespace component**: `resolve_attribute_qname` reads the existing sparse `Namespace` component via
  `namespace_of` (`dom/mod.rs:431`) — no new per-entity state.

---

## §6. Out of scope (deferred — explicit)

- **`*NS` variants** (`getAttributeNS`/`setAttributeNodeNS`/`setNamedItemNS`/`removeNamedItemNS`):
  Phase-2 simplification — every Attr has `namespaceURI=null`, `localName==qualifiedName`.
  setNamedItemNS/removeNamedItemNS keep the existing null-namespace pass-through / non-null reject. Full
  XML namespaces = Deferred #21 (`attr_proto.rs` module doc).
- **`setAttributeNode`/`setNamedItem` cross-element InUseAttributeError** (as-built deviation, surfaced at
  impl — the plan's original "keep the existing InUse check" premise was wrong; there is none). Spec
  set-an-attribute step 2 throws InUseAttributeError when the source Attr is owned by another element; the
  VM instead does a documented Phase-2 cross-element **value-copy** (its per-VM Attr wrapper has no
  owner-retarget), locked by 5 pre-existing tests. This slice **records** the value-copy write (correct —
  it is a real write on `recv`) but does not add the throw. Spec-strict InUse needs the Phase-3
  Attr-node-ownership model (the VM using real Attr-node entities — cf. the §4.1 dual-model note). →
  slot `#11-set-attribute-node-cross-element-inuse` (register at merge, §9).
- **`#11-ce-reaction-mutation-observer-ordering`** — VM drains MO microtasks before CE reactions; fix =
  event-loop drain order, not `apply_*`. Stays deferred.
- **`#11-method-driven-attribute-records`** — dialog `close()` / shell `method=dialog` open-removal, off
  the `invoke_dom_api` drain path. Stays deferred.
- **`attributeNamespace` record field** — parked (`#11-mutation-observer-extras` (a)); records emit
  `attributeName` only. Unchanged.

---

## §7. Test plan (supported-surface, JS-driven)

Mirror #428/#431 — drive real JS mutations through the VM, assert delivered MutationRecords. New tests in
the MO attribute test module.

**Input-construction constraints (from the Step-1.5 dry-run — no JS `createElementNS` / VM `createAttribute`)**:
- **SVG/MathML elements** are **parser-constructed** (e.g. parse `<svg viewBox="…">`), since JS
  `createElementNS` is not VM-wired; the parser sets `Namespace::Svg` so the resolver preserves case.
- A **detached source Attr** (for `setAttributeNode`/`setNamedItem` "fresh") is obtained by detaching an
  existing one (`a = el.getAttributeNode("x"); el.removeAttributeNode(a)` → `a` detached), since
  `document.createAttribute` is not VM-reachable.

**Record production (positive)**:
- `attr = el.getAttributeNode("id"); attr.value = "x"` → 1 attributes record (target=el, oldValue=prev).
- `el.setAttributeNode(detachedAttr)` (fresh name) → 1 record (oldValue=null); (replacing same name) →
  1 record (oldValue=prev); returns the prior Attr.
- `el.removeAttributeNode(attr)` → 1 record (oldValue=prev); returns the same frozen attr.
- `el.attributes.setNamedItem(attr)` / `removeNamedItem("x")` → records mirroring set/remove.

**Negative controls (record-less by construction)**:
- detached `attr.value = "x"` (createAttribute / removed-then-held) → NO record (I1; A2×A2 corner).
- `el.setAttributeNode(el.getAttributeNode("id"))` (oldAttr==attr) → NO record (A1×A5 corner 4).
- `el.removeNamedItem("missing")` → throws NotFoundError, NO record.

**Casing (whole-surface)**:
- HTML element: `setAttribute("ID","x")` then `getAttribute("id")` / `hasAttribute("Id")` all hit.
- SVG element: `setAttribute("viewBox", v)` then `getAttribute("viewBox")` hit, `getAttribute("viewbox")`
  miss (case-preserved); `removeNamedItem("viewBox")` / `removeAttribute("viewBox")` remove it; the
  record's attributeName = "viewBox".
- **path consistency**: `el.hasAttribute("ID")` (VM) == dom-api `HasAttribute` on HTML and SVG (the fix).
- `getAttributeNode("ID")` on HTML element finds the `id` attr (was a latent miss).
- **bracket-access stays case-sensitive** (NOT-applied lock): `el.attributes["id"]` hits, `el.attributes["ID"]`
  ⇒ undefined (HTML); `svg.attributes["viewBox"]` hits, `["viewbox"]` ⇒ undefined (SVG) — WebIDL
  supported-property-names, no lookup-name lowercase.

**Wrapper identity / detach**:
- remove→same-name re-add allocates a fresh canonical wrapper; a held detached wrapper keeps its snapshot
  value across the re-add (existing module-doc scenario, now record-producing).

`cargo test -p elidex-js -p elidex-dom-api -p elidex-ecs --all-features` (changed crates); full
`mise run ci` in the pre-push gate.

---

## §8. Invariants (carry into impl + review)

- **I1** value-mode / detached writes are record-less by construction (no `apply_*` call).
- **I2/I9** snapshot→apply→freeze ordering; commit = push+drain indivisible (`commit_notify_records`).
- **I4** same-value content-attribute writes still record.
- **I11** absent-attribute removals produce no record.
- **I-CASE** resolution is HTML-namespace-gated, applied at name-based entry points only; `apply_*` is
  name-agnostic; node-identity ops use the Attr's verbatim local name.
- **I-CACHE-KEY** (F1/F2) every name-based Attr-cache hit / invalidation / snapshot site — the VM
  `wrapper_store` (getAttributeNode / getNamedItem / removeNamedItem / remove-attr & toggle snapshots)
  AND the dom-api `AttrEntityCache` (`GetAttributeNode` lookup/insert; `RemoveAttribute` evict, which
  reuses the single resolver-applied `name` binding; the chokepoint `sync_cached_attr_value` read on the
  currently-dormant cache) — keys on `intern(resolve_attribute_qname(entity, raw))`, the SAME resolved
  `StringId` as the storage lookup. Never `intern(raw)` (regresses HTML re-key) nor
  `intern(unconditional-lowercase)` (regresses SVG). A resolved-vs-raw key mismatch silently leaks a
  stale cached `Attr` (attr_proto.rs:43–48 hazard) and breaks SameObject identity.
  - **Node-identity verbatim-key safety (load-bearing, pin):** setAttributeNode/setNamedItem/
    removeAttributeNode populate/evict the wrapper cache under the Attr's **verbatim** `qualified_name`
    (the §4.2 node-identity exclusion), which stays consistent with the resolved hit-keys ONLY because a
    cacheable Attr's `qualified_name` is itself always `intern(resolve_attribute_qname(owner, name))` at
    allocation (holds across all four `AttrState` ctor paths today). Preserve this on any new
    Attr-allocation path — a cacheable Attr minted from a raw name would silently diverge the verbatim
    populate key from the resolved hit key.
- **I-ONE-DETACH** exactly one wrapper detach mechanism (`snapshot`/`freeze` helpers); cache
  invalidation structural.

---

## §9. At-merge ledger

- **CLOSE** `#11-attribute-name-html-namespace-casing` in `project_open-defer-slots.md` (whole-surface fold
  landed; record the `is_html_namespace`-only gate + the noted document-type limitation).
- **REGISTER** `#11-set-attribute-node-cross-element-inuse` (as-built deviation, §6): VM
  setAttributeNode/setNamedItem value-copy a cross-element source instead of throwing InUseAttributeError
  (spec set-an-attribute step 2); needs the Phase-3 Attr-node-ownership model. Trigger = cross-element
  setAttributeNode WPT or the VM-Attr-as-entity unification. Date 2026-09-30. Net cap-neutral with the
  casing CLOSE above.
- Attribute records **complete** after this slice → update [[reference_js-tree-mutations-not-recorded]]
  (remaining = the 3 off-record-path deferred slots only).
- Keep deferred: `#11-ce-reaction-mutation-observer-ordering`, `#11-method-driven-attribute-records`,
  `#11-mutation-observer-extras` (attributeNamespace), Deferred #21 (XML namespaces).
- Program B record coverage = childList + characterData + attributes all complete.

---

## §10. Implementation order (single PR)

1. `EcsDom::resolve_attribute_qname` (NEW) + unit tests (elidex-ecs) — the resolver first.
2. Casing fold (atomic, whole-surface): apply the resolver at the 12 name-based sites (§4.2 table) incl.
   the dom-api `GetAttributeNode` fold, re-keying the wrapper / `AttrEntityCache` caches via the resolved
   name (§8 I-CACHE-KEY); fix `native_element_has_attribute` path-dependence; `CreateAttribute` +
   `try_indexed_get` stay NOT-applied per §4.2. + casing tests (incl. SVG case-preserve + `nnm["ID"]`
   case-sensitive).
3. dom-api `char_data/attr.rs` handlers (`SetAttrValue`/`SetAttributeNode`/`RemoveAttributeNode`) → route
   through `apply_*` + `push_notify_record`.
4. VM shims: `native_attr_set_value` / `native_element_set_attribute_node` /
   `native_element_remove_attribute_node` / `native_nnm_set_named_item` / `native_nnm_remove_named_item`
   → `apply_*` + shared wrapper helpers + `commit_notify_records` (incl. the step-4 short-circuit). +
   record/identity tests.
5. Pre-push 6-stage gate → push + `gh pr create` → `/external-converge`.
