# B2-Slice-2 — reflected IDL setter attribute-write MutationRecords (plan-memo)

**Status**: PRE-IMPLEMENTATION plan-memo → `/elidex-plan-review` (edge-dense per-slice rule). (2026-06-29)
**Program**: B (F3 mutation-path). B2 is the **LAST record gap**; Slice-1 (#428 `d09829a5`) established the record-producing attribute seam for the generic `setAttribute`/`removeAttribute`/`toggleAttribute` DOM methods. Slice-2 routes the **reflected IDL setters** (and the handful of method-driven attribute writes) through that same seam so they emit DOM §4.9 "attributes" records. **Largest blast radius.** Slice-3 (Attr/NamedNodeMap) follows.
**Base**: `d09829a5` (post-#428). Worktree `elidex-b2-slice2`, branch `b2-slice2` (off `origin/main`).
**Umbrella**: `docs/plans/2026-06-b2-attribute-records-umbrella-and-slice1.md` §0/§9 (Slice-2 carve).

---

## §0 What Slice-1 already built (reused verbatim — NO new mechanism)

The seam Slice-2 routes more writers through:

- **`EcsDom::set_attribute(entity, name, value) -> AttributeWrite{did_set, old_value}`** / **`EcsDom::remove_attribute(entity, name) -> Option<String>`** — the attribute-write chokepoint (`crates/core/elidex-ecs/src/dom/attribute.rs`). Fires the `MutationEvent::AttributeChange` fan-out (§4.9 steps 2–3) + surfaces the pre-write `oldValue`. **Signature already landed in Slice-1 — Slice-2 changes NO EcsDom signature.**
- **`apply_set_attribute(dom, entity, name, value) -> Option<MutationRecord>`** / **`apply_remove_attribute(dom, entity, name) -> Option<MutationRecord>`** (`crates/script/elidex-script-session/src/mutation/mod.rs:830/853`, both `pub`) — the record-producing primitives: call the real chokepoint (full fan-out preserved) + build the §4.9 step-1 record from the surfaced oldValue via `attribute_record(target, name, old_value)` (`mutation/mod.rs:246`). `apply_set_attribute` always records on a landed write (same-value fires, I4); `apply_remove_attribute` records only when something was removed (remove-missing = `None` = no record).
- **Delivery (UNCHANGED, I7)**: `session.push_notify_record(record)` → drain → `queue_mutation_record` → §4.3 microtask → `MutationObserverRegistry::notify` (attributes branch: attributeFilter / attributeOldValue / subtree — already implemented). Two drain points exist:
  - `invoke_dom_api` Phase 2.5 (`dom_bridge.rs:510`) — auto-drains after every dom-api handler call.
  - `commit_range_mutation_records` (`dom_bridge.rs:53`) — the push+drain-as-one-step helper for natives that bypass `invoke_dom_api` (Range natives today; the **host-shim reflected setters** join it, §4).

**The whole Slice-2 = wire more writers to `apply_set_attribute`/`apply_remove_attribute`.** That is data-flow wiring, not a mechanism or layering change (F3).

---

## §1 First-principles ideal (the convergence)

DOM §4.9 "handle attribute changes" is the single algorithm EVERY attribute mutation funnels through; step 1 = queue the "attributes" MO record. Slice-1 made the generic methods emit step-1; the ideal end state is that **every** content-attribute write — reflected IDL setter, classList/dataset/style, hyperlink, dialog-close — converges on the one record-producing primitive, regardless of which IDL surface drove it. "One issue (the §4.9 step-1 record), one way (`apply_*`)."

Two **layer-appropriate wiring points** (NOT two mechanisms — both call the same `apply_*`):

1. **VM host (marshalling layer, F3)** — the two existing shims **`attr_set`/`attr_remove`** (`crates/script/elidex-js/src/vm/host/element_attrs.rs:106/197`) become record-producing: each wraps `apply_*` in `with_session_and_dom` + `push_notify_record`, then drains (mirroring `invoke_dom_api` Phase 2→2.5). **Every** reflected-setter write in `vm/host/` routes through these two shims (the ~50 direct `dom().set_attribute(...)` sites + 11 reflect macros migrate onto `attr_set`; the 16 `attr_remove` sites are auto-covered the moment the shim records). Reflected setters **stay in `host/`** — they are pure marshalling (reflect IDL prop → known-lowercase content-attr write, no DOM algorithm), so they do NOT move to `invoke_dom_api` (F3).
2. **dom-api handlers (engine-independent layer)** — the single per-subsystem write helper builds the record via `apply_*` + `session.push_notify_record` (the `invoke_dom_api` Phase 2.5 drains automatically — these handlers already carry `session`):
   - classList/relList/htmlFor → `set_token_string` (`class_list.rs:82`)
   - dataset → `DatasetSet` (`element/attrs.rs:364`) + `DatasetDelete` (`element/attrs.rs:390`)
   - style → `sync_to_attribute` (`style.rs:133`)
   - hyperlink href (+ all URL-decomposition setters) → `write_href_attr` (`element/href_accessor.rs:~140`)
   - dialog close → `close_the_dialog`'s `open` removal (`dialog.rs:80`) — §9 scope decision

This is **dead-code-free convergence**: `attr_set`/`attr_remove` ALREADY exist as the host shims (`attr_remove` is already the convergence point for all 16 boolean-detach removals — One-issue-one-way is half-done); Slice-2 finishes it by making them record + migrating the direct-write stragglers onto them.

---

## §2 Coupled-invariant enumeration (edge-dense, mandatory)

| # | Invariant | Resolution |
|---|-----------|------------|
| **I1** | **value-mode exclusion** (Slice-1 I6, now Slice-2's to honor) | `input.value` text/value-mode (`ValueSetAction::SetLiveValue` → `state.set_value`, `html_input_value.rs:122`), `valueAsNumber`, `checked`, `indeterminate`, `clear_file_value`, and the reconciler's `EcsDom::set_attribute_without_dispatch` (`value_mode.rs:222`) are NOT content-attribute changes → produce **NO record by construction** (they never reach `apply_*`). The `ValueSetAction::SetContentAttr` arm (default-mode `value`, `html_input_value.rs:129`), `defaultValue` (`:182`), `defaultChecked` (`:253/255`) ARE real reflections → they DO record. The migration (only `dom().set_attribute` call-sites → `attr_set`) separates these automatically: `set_value`/`_without_dispatch` are left untouched. |
| **I2** | **coalescing** (one attribute write = one record) | `classList.add("a","b")` writes `class` ONCE (after `run_update_steps` re-serializes) → 1 record. `style.color="red"` writes `style` once → 1 record. Slice-2 does NOT change WHEN/whether the attribute is written — it only attaches record production at the existing write site, so coalescing follows the existing write timing. Same-value writes still fire (Slice-1 I4: §4.9 "change an attribute" queues unconditionally; `apply_set_attribute` records on any landed `did_set`). |
| **I3** | **oldValue fidelity** | record `oldValue` = the prior **content-attribute** string (prior `class` / serialized `style` / `data-foo` / `href`), surfaced by the chokepoint and consumed by `apply_set_attribute` — correct by construction (NOT re-derived from the post-write component). |
| **I4** | **dataset name conversion** | record `attributeName` = the converted **content-attr local name** (`data-foo-bar`), not the JS camelCase key (`fooBar`). `apply_set_attribute` records the `name` it is passed, and the handler passes the already-converted `camel_to_data_attr(key)` result. |
| **I5** | **hyperlink URL-decomposition** | `a.protocol=`/`a.host=`/… all reconstruct + write the **`href`** attribute (via `href_url_set_component` → `write_href_attr`), as does `a.href=` (`set_href` → `write_href_attr`). So a SINGLE record-emission at `write_href_attr` covers the entire hyperlink mixin: `attributeName="href"`, `oldValue`=prior href. Shared by `<a>`/`<area>` (and `<link>` where applicable). |
| **I6** | **dual-runtime** (Slice-1 I8) | The VM `className`/`id` path is host `reflected_string_set` → `attr_set` (`element_attrs.rs:699/715`) → **covered** by the shim. The dom-api `SetClassName`/`SetId` handlers (`element/attrs.rs:207/255`) are invoked **only by boa** (`elidex-js-boa/.../properties.rs:43/81`) — S5-6-deletion-bound, **light-touch → NOT wired** (leaving them record-less is sanctioned, not a strangler: boa is a whole separate runtime being deleted, per `[[feedback_boa-findings-light-touch]]`). |
| **I7** | **CE ↔ MO delivery ordering** | Inherited UNCHANGED from Slice-1's deferred slot `#11-ce-reaction-mutation-observer-ordering` (the VM drains MO microtasks before CE reactions). Reflected setters on a custom element now also surface it, but it is **general, not owned here** — do NOT re-carve, do NOT attempt a per-site fix (would re-fork the chokepoint). |
| **I8** | **attribute-name casing** | Reflected setters write **literal lowercase** content-attr names (`"class"`, `"style"`, `"data-*"`, `"href"`, `"id"`, `"value"`, …) — no casing decision at these sites. Maintain the **uniform-lowercase baseline**; do NOT introduce `is_html_namespace` gating here (the whole-surface fix is owned by slot `#11-attribute-name-html-namespace-casing`; partial gating = forbidden strangler, the exact #428 R3→R4 trap). |
| **I9** | **borrow/drain discipline** | Host shims: `with_session_and_dom(\|s,d\| apply_*(d,…).map(\|r\| s.push_notify_record(r)))` (host_data borrow), THEN `ctx.vm.drain_notify_records()` (vm re-borrow) — the proven `invoke_dom_api` Phase 2→2.5 ordering. `attr_remove` preserves its snapshot→remove→freeze ordering and inserts the record build inside the same `with_session_and_dom` (after `apply_remove_attribute`), drains after freeze (freeze = VM wrapper state, drain = microtask queue — order between them is independent). |
| **I10** | **style CSSOM-cache re-insert** | `sync_to_attribute` re-inserts the cloned `InlineStyle` AFTER the `style`-attribute write to keep the CSSOM cache warm (the write drops the memoized component). Routing the write through `apply_set_attribute` (which calls `EcsDom::set_attribute` internally) drops the component identically → the re-insert is preserved unchanged. |
| **I11** | **no-op / failed write** | A write that does not land (destroyed/non-Element receiver) returns `None` from `apply_*` → no record (the shim's `bool` return = `did_set`). `removeAttribute`-of-absent on a reflected boolean detach (e.g. `el.hidden=false` when already absent) → `apply_remove_attribute` returns `None` → no record (I4 from Slice-1). |

---

## §3 Spec coverage map

Citations webref-verified (re-verify at impl per CLAUDE.md). §4.9 = handle attribute changes / change an attribute / set an attribute value / remove an attribute by name; §4.3.2 = queue a mutation record; §4.3.3 = MutationRecord. Reflected-attribute reflection = HTML §2.6.2 (reflecting content attributes in IDL).

| Surface | Spec | Branch | Wiring site | Record? |
|---|---|---|---|---|
| Reflected string/bool/long IDL setters (`id`, `className`, `hidden`, `disabled`, `type`, `value` default-mode, `defaultValue`, `name`, `tabIndex`, …) | HTML §2.6.2 reflect → DOM §4.9 set/remove an attribute | set / remove arms | host `attr_set`/`attr_remove` | ✓ |
| classList / relList / htmlFor (DOMTokenList) | DOM §7.1 + §4.9 | add/remove/toggle/replace/value/`className` | dom-api `set_token_string` | ✓ |
| dataset (DOMStringMap) | HTML §3.2.6.6 + DOM §4.9 | set / delete | dom-api `DatasetSet`/`DatasetDelete` | ✓ |
| style (CSSStyleDeclaration) | CSSOM §6 + DOM §4.9 | setProperty/removeProperty/cssText/camel setter | dom-api `sync_to_attribute` | ✓ |
| hyperlink (`a`/`area` URL members) | HTML §4.6.3 + URL §6 + DOM §4.9 | href + protocol/host/…/hash | dom-api `write_href_attr` | ✓ (attributeName="href") |
| dialog close (`open` removal) | HTML §4.11.4 "close the dialog" step 5 → DOM §4.9 | remove `open` | dom-api `close_the_dialog` | ✓ (§9 scope) |
| value-mode live-value / `_without_dispatch` | HTML §4.10.5.4 value IDL / type-change | live-value / reconcile | — (NOT a content-attr change) | **✗ by construction (I1)** |
| `SetClassName`/`SetId` (boa) | — | className/id | boa-only | **✗ light-touch (I6)** |

**Breadth**: K=4 spec families (DOM/HTML/CSSOM/URL), M≈8 surfaces — but a single mechanism (the Slice-1 `apply_*` primitive) drives all; this is a wide-but-shallow **data-flow** sweep, umbrella-decomposed, plan-reviewed. Justified as one PR per the umbrella's per-slice base-case rule.

### §3.1 User-input touch audit
Every site takes user-controlled values (`el.id = userStr`, `el.style.cssText = userStr`, `el.dataset.x = userStr`, `a.href = userStr`). All route through the chokepoint's existing write (value stored verbatim; no new sanitization). The record's `oldValue` exposure is gated by `attributeOldValue:true` (existing registry, I7-delivery). No new trust boundary — Slice-2 adds record *observation* of writes that already occurred.

---

## §4 The two wiring patterns (detail)

### 4.1 Host shim (`attr_set`/`attr_remove`) — push **and** drain (bypasses `invoke_dom_api`)
Reflected setters call the shims directly (not via `invoke_dom_api`), so the shim must drain itself (no Phase 2.5). Mirror `commit_range_mutation_records`:

```rust
pub(super) fn attr_set(ctx, entity, name, value) -> bool {
    let Some(host) = ctx.host_if_bound() else { return false };
    let did_set = host.with_session_and_dom(|session, dom| {
        match apply_set_attribute(dom, entity, name, value) {
            Some(record) => { session.push_notify_record(record); true }
            None => false,
        }
    });
    ctx.vm.drain_notify_records();   // host_data borrow ended above; re-borrow vm (Phase-2.5 ordering)
    did_set
}
```
`attr_remove` keeps snapshot→remove→freeze; the record build goes inside the same `with_session_and_dom` after `apply_remove_attribute`, drain after freeze (I9).

### 4.2 dom-api handler write helpers — push only (Phase 2.5 drains)
These handlers already receive `session: &mut SessionCore` (currently `_session`, unused). Un-underscore + build/push:
```rust
// e.g. set_token_string (classList), DatasetSet/DatasetDelete, sync_to_attribute (style), write_href_attr (hyperlink)
if let Some(record) = apply_set_attribute(dom, entity, attr_name, value) {
    session.push_notify_record(record);
}
```
Thread `session` into the shared write helper where it is not already a parameter (`set_token_string`, `sync_to_attribute`, `write_href_attr` / `set_href` / `href_url_set_component`, `close_the_dialog`). The `invoke_dom_api` caller drains.

### 4.3 Why NOT route host reflected setters through `invoke_dom_api` (F3)
`invoke_dom_api("setAttribute")` re-runs the §4.9 setAttribute *method* layer (name validation, `is_html_namespace` lowercasing, brand re-check) — none of which apply to a reflected setter that already knows the exact lowercase attr name. Reflected setters are marshalling, not the DOM algorithm → host shim is the correct layer (F3). (Contrast: the generic `setAttribute` native DID converge onto `invoke_dom_api` in Slice-1 because it IS the §4.9 method.)

---

## §5 Changes (enumerated)

### 5.1 `vm/host/element_attrs.rs` — make the two shims record-producing
- `attr_set` (`:106`): rewrite per §4.1 (with_session_and_dom + apply_set_attribute + push + drain). Keep `-> bool` (did_set) for callers that use it.
- `attr_remove` (`:197`): insert `apply_remove_attribute` + push inside the existing snapshot/freeze bracket; drain after freeze (§4.1).

### 5.2 `vm/host/*` — migrate every direct reflected `dom().set_attribute(...)` onto `attr_set`
The ~50 direct set-sites + the **11 reflect macros** (`button_string_attr!`/`button_bool_attr!`/`form_string_attr!`/`iframe_string_attr!`/`input_string_attr!`/`input_bool_attr!`/`sel_string_attr!`/`sel_bool_attr!`/`ta_string_attr!`/`ta_bool_attr!` + the `long_set`/`set_canvas_dim_attr`/`string_reflect_set`/`bool_*_reflect`/`bool_reflect_set` shared helpers) swap their `ctx.host().dom().set_attribute(entity, $attr, &s)` body for `super::element_attrs::attr_set(ctx, entity, $attr, &s)`. (Editing each macro BODY covers all its instantiations.) Files: `html_{button,element,fieldset,form,iframe,input,label,optgroup,option,select,textarea,details}_proto.rs`, `html_input_value.rs`, `form_state_sync.rs`, `canvas/mod.rs`. The 16 `attr_remove` sites need NO edit (auto-covered by 5.1). **Borrow note**: a few sites hold a live `let dom = ctx.host().dom()` (e.g. `html_input_value.rs:108`); restructure to drop that borrow before `attr_set(ctx, …)`.
- **value exclusion (I1)**: do NOT touch `state.set_value`/`set_value`/`clear_file_value`/`set_attribute_without_dispatch` sites; only the `ValueSetAction::SetContentAttr` arm + `defaultValue`/`defaultChecked` migrate.

### 5.3 `elidex-dom-api` — wire the 5 handler write helpers
- `class_list.rs` `set_token_string` (`:66/82`): thread `session`, `apply_set_attribute` + push. Covers classList/relList/htmlFor (`TokenListHandler`).
- `element/attrs.rs` `DatasetSet` (`:355/364`) + `DatasetDelete` (`:381/390`): un-underscore `session`, `apply_set_attribute`/`apply_remove_attribute` + push.
- `style.rs` `sync_to_attribute` (`:119/133`): thread `session`, `apply_set_attribute` + push (preserve the post-write `InlineStyle` re-insert, I10).
- `element/href_accessor.rs` `write_href_attr` (`:~140/149`): thread `session` through `set_href` + `href_url_set_component`, `apply_set_attribute` + push.
- `dialog.rs` `close_the_dialog` (`:51/80`): thread `session`, `apply_remove_attribute` + push (§9 scope decision).
- Imports: `class_list.rs`/`style.rs`/`dialog.rs`/`href_accessor.rs` add `apply_set_attribute`/`apply_remove_attribute` (already imported in `element/attrs.rs`).
- **NOT touched**: `SetClassName`/`SetId` (boa-only, I6); `char_data/attr.rs` (Attr/NamedNodeMap = Slice-3).

### 5.4 Delivery — already wired (zero change, §0/I7).

---

## §6 Tests (MO-driven, mirror #428/#424 harness)
- **Reflected string/bool/long**: `el.id="x"` / `el.className="a b"` / `el.hidden=true` (set) + `el.hidden=false` (remove) / `input.type="email"` / `input.defaultValue="d"` → each 1 `attributes` record (attributeName, oldValue with/without `attributeOldValue:true`).
- **value-mode exclusion (I1, load-bearing)**: `input.type="text"; input.value="x"` → **NO record** (live-value); `input.type="hidden"; input.value="x"` (default-mode SetContentAttr) → **1 record** (attributeName="value"). Negative-control confirms the exclusion is real.
- **classList coalescing (I2)**: `el.classList.add("a","b")` → **1** record (attributeName="class", oldValue=prior); `el.classList.remove("a")` → 1; `el.className="c"` → 1.
- **dataset (I4)**: `el.dataset.fooBar="x"` → 1 record attributeName=**"data-foo-bar"**; `delete el.dataset.fooBar` → 1 record (remove).
- **style (I3/I10)**: `el.style.color="red"` → 1 record attributeName="style", oldValue=prior serialization; `el.style.removeProperty("color")` → 1.
- **hyperlink (I5)**: `a.href="http://x/"` → 1 record attributeName="href"; `a.protocol="https"` → 1 record attributeName=**"href"** (URL-decomposition writes href).
- **dialog (§9)**: `dialog.close()` → 1 record attributeName="open" (remove).
- **attributeFilter / subtree / attributeOldValue** gating on a reflected setter (one representative) — confirms delivery path unchanged.
- **Fan-out-preserved regression (I-lock)**: a reflected `el.className=`/`a.href=` STILL drives its derived consumer (class-index / base-url) — the record wiring did not regress the chokepoint fan-out.
- **boa unaffected** (I6): no new boa test; existing boa attribute tests stay green.

---

## §7 Files + 1000-line
`vm/host/element_attrs.rs` (shims) + the ~14 `vm/host/*_proto.rs`/`html_input_value.rs`/`form_state_sync.rs`/`canvas/mod.rs` (macro/site migration) + `elidex-dom-api/{class_list,style,dialog}.rs` + `element/{attrs,href_accessor}.rs` (handler wiring) + MO tests. **Re-check LoC at impl** on each touched file (`element_attrs.rs`, `style.rs`, `class_list.rs`, `html_input_*` are the candidates); any real cohesion seam crossing 1000 → standalone prereq split (NOT bundled, CLAUDE.md).

## §8 Process
fmt → `mise run ci` → `/pre-push` (6-stage: simplify/code-review/review/elidex-review) → push + `gh pr create` → `/external-converge`. At merge: update `[[project_b1-mutationobserver-next-task]]` (Slice-2 done, NEXT Slice-3) + `[[m4-12-landings-ledger]]` + MEMORY.md + `[[reference_js-tree-mutations-not-recorded]]`. Mirror = #428 (the shim/handler pattern) + #393 (handler wiring).

## §9 Deferrals + scope decisions
- **Slice-3 (Attr/NamedNodeMap)** — own plan-review. `Attr.value` setter + `setNamedItem`/`removeNamedItem` + `setAttributeNode`/`removeAttributeNode` (`vm/host/{named_node_map,attr_proto}.rs` + `element_attrs.rs:453/574` + dom-api `char_data/attr.rs:161/230/318`) + the VM-local Attr-wrapper detach asymmetry.
- **SCOPE DECISION — dialog.close() included (not deferred)**: the dom-api census surfaced exactly one non-reflected-setter production direct-write — `close_the_dialog`'s `open` removal (`dialog.rs:80`, JS-reachable via `dialog.close()`/`requestClose()`/form `method=dialog`). It is NOT literally a "reflected IDL setter", but **leaving it record-less while every sibling attribute write records is a strangler** (B2 must be "the LAST record gap"), and the wiring is identical (one `apply_remove_attribute`). Decided per `[[feedback_decide-via-philosophy-before-asking]]` (One-issue-one-way + no-strangler lens converged). **Fallback** if session-threading through `close_the_dialog`'s callers proves edge-dense at impl: carve `#11-method-driven-attribute-records` and drop dialog from this slice (flag at plan-review).
- **details name-group exclusion — already covered**: `html_details_proto.rs` routes `open` set/remove via `invoke_dom_api("setAttribute"/"removeAttribute")` (Slice-1) — no Slice-2 work.
- **anchor/area/img/link/meta/script non-href reflected attrs — already covered**: they use `reflect_setter!` → `invoke_dom_api("setAttribute")` (Slice-1). Only the URL-backed `href` family (hyperlink mixin) needs wiring.
- **boa `SetClassName`/`SetId`** — boa-only, S5-6-deletion-bound, light-touch (I6); not wired, no slot (closes at boa removal).
- **`attributeNamespace` record field** → rides `#11-mutation-observer-extras` (Slice-1 deferral; namespace-less shape unchanged).
- **CE↔MO ordering** (`#11-ce-reaction-mutation-observer-ordering`) + **attribute-name casing** (`#11-attribute-name-html-namespace-casing`) — Slice-1 slots, inherited unchanged (I7/I8); Slice-2 does NOT touch either.
