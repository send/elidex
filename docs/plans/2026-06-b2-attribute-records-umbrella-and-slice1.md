# B2 — attribute-write MutationRecord convergence (umbrella + Slice-1)

**Status**: plan-memo, pre-`/elidex-plan-review`. (2026-06-28)
**Program**: B (F3 mutation-path) — B2 is the **LAST record gap**. childList (#379–#418) + characterData (#424/#426) already converged onto record-producing `apply_*` in `elidex-script-session::mutation` delivered via `push_notify_record`→drain→§4.3 microtask. B2 does the same for the **`attributes`** record kind.
**Base**: `dc6970ad` (post-#426). Worktree `elidex-b2`, branch `b2-attribute-records`.
**Edge-dense → umbrella + per-PR slices** (CLAUDE.md "Edge-dense work = multi-PR + 実装前 plan-review 必須"). This memo = the umbrella decomposition (§0) + **Slice-1** (mechanism + generic `setAttribute`/`removeAttribute`/`toggleAttribute`). Slices 2–3 carved (§9). `/elidex-plan-review` is on Slice-1.

---

## §0 Umbrella decomposition

B2 intersects ≥3 coupled invariant axes (synchronous-apply / ConsumerDispatcher-fan-out-vs-record / seam-ownership / same-value+remove-missing gating / `_without_dispatch` value-mode boundary / Attr-node detach / dual-runtime). So:

- **Slice-1 (THIS) — mechanism + generic `setAttribute`/`removeAttribute`/`toggleAttribute`**: resolve the chokepoint-vs-seam corner ONCE; establish the record-producing attribute seam; wire the generic dom-api handlers (`element/props.rs`/`attrs.rs`). The mechanism the later slices reuse verbatim.
- **Slice-2 — reflected IDL setters** (~53 direct `dom().set_attribute` host call-sites across ~17 `html_*_proto.rs` + `invoke_dom_api` anchor/area + classList/dataset/style): route reflected content-attribute writes through the Slice-1 seam. **Largest blast radius.** Carries the `input.value` value-mode **8kHF exclusion** (no record — not a reflection) + the `value_mode.rs` `set_attribute_without_dispatch` boundary. **F3 framing — Slice-2 is DATA-FLOW wiring, NOT a layering fix**: these reflected setters are legitimate `vm/host/` *marshalling* call-sites (reflect IDL property → simple content-attribute write, not a DOM algorithm), so they stay in `host/`; Slice-2 only swaps each one's marshalling target from the bare chokepoint to the record-producing `apply_*` seam (so they emit records). The toggleAttribute dup (F2, §5.3) is the one genuine *layering* item, fixed in Slice-1. **F6 interim**: after Slice-1, generic `setAttribute`/`removeAttribute`/`toggleAttribute` emit records but reflected setters (`el.id=`, `a.href=`) do NOT yet — a bounded, acknowledged partial-coverage interim (strangler-free: each slice routes more writers through the one already-correct seam), closed by Slice-2.
- **Slice-3 — `Attr`/`NamedNodeMap`** (`Attr.value` setter, `setNamedItem`/`removeNamedItem`, `*AttributeNode`) + the VM-local Attr-wrapper detach asymmetry + classList/dataset/style coverage confirmation.
- **DEFERRED (not B2)**: `attributeNamespace` on the record struct → existing slot `#11-mutation-observer-extras` (both session + registry `MutationRecord` lack the field; Slice-1 records the namespace-less shape, namespace parked). `*AttributeNS`/`*NamedItemNS` are UNIMPLEMENTED (zero call-sites) — out of scope until implemented.

Each slice = its own `/elidex-plan-review` before impl (per-PR rule; this base-case under the umbrella).

---

## §1 First-principles ideal (the core resolution)

**The spec hands us the design.** DOM §4.9 "**handle attribute changes**" (the single algorithm every attribute mutation — set/change/remove — funnels through) runs three steps for one attribute change:
1. **Queue a mutation record of "attributes"** (localName, namespace, oldValue) ← the MO record.
2. Enqueue the CE `attributeChangedCallback` reaction.
3. Run the **attribute change steps** (derived state) ← elidex's ConsumerDispatcher fan-out.

elidex's `EcsDom::set_attribute` **already IS "handle attribute changes"**: it fires `MutationEvent::AttributeChange { node, name, old_value, new_value }` (→ the 7 ConsumerDispatcher consumers = step 3 + the CE tap = step 2) + reconciles derived components + syncs the materialized `Attr` node (§4.9 identity) + bumps `rev_version`. The ONLY missing piece is **step 1 — the MO record** is never produced for the production path.

**So the ideal mirrors B1.3-ii's `replace_comment_data` fix exactly**: the primitive already does the work + fires the event; we make the record originate from the seam that calls it, WITHOUT re-forking the chokepoint.

> **Slice-1 resolution**: the dom-api record-producing seam (`apply_set_attribute`/`apply_remove_attribute`) calls the **real `EcsDom::set_attribute`/`remove_attribute` chokepoint** (full fan-out + reconcile + Attr-sync + CE tap = §4.9 steps 2–3, ALL preserved) and the chokepoint **surfaces the captured `oldValue`** so the seam builds the §4.3.2 "attributes" record (§4.9 step 1) + `push_notify_record`. Synchronous write at the chokepoint (read-your-writes), MO record owned by the ScriptSession seam.

**This is dead-code revival + a root fix.** `apply_set_attribute`/`apply_remove_attribute` ALREADY exist in `mutation/mod.rs` and ALREADY build the Attribute record — but they are (a) production-dead (only the boa iframe setter reaches them) AND (b) **broken: they bypass `EcsDom::set_attribute`** (mutate `Attributes` directly + hand-roll a partial reconcile) so they **DROP the ConsumerDispatcher fan-out** (base-url / form-control / event-handler / canvas / CE) = invariant-2 violation. Slice-1 **rewrites them to route through the chokepoint** (the One-issue-one-way fix), making the existing seam correct + wiring it to production.

**The one load-bearing API change**: `EcsDom::set_attribute` returns only `bool` today and **discards the `oldValue`** it captures internally (`attribute.rs` `write_attribute_no_dispatch`). Change it to **surface `oldValue`** (return `Option<String>` old value, or a small struct) so the seam can build the record. `remove_attribute` likewise (it already gates the event on `old_value.is_some()` — surface that). This is the minimal, by-construction-correct seam (vs. the rejected poles, §4).

---

## §2 Coupled-invariant enumeration (edge-dense, mandatory)

| # | Invariant | Intersection / resolution |
|---|-----------|---------------------------|
| I1 | **synchronous-apply (read-your-writes)** | The write applies at `EcsDom::set_attribute` synchronously (unchanged); the MO record is captured in the SAME call (seam reads the surfaced oldValue, builds record, pushes). NOT deferred to flush. `getAttribute` after `setAttribute` reflects immediately. |
| I1×I2 | sync-apply × **fan-out preservation** | Both satisfied by routing through the chokepoint (write + fan-out + record-data all at one synchronous call). The current `apply_set_attribute` bypass satisfies I1 but VIOLATES I2 (dropped fan-out) — Slice-1 fixes that. |
| I2×I3 | **fan-out (chokepoint)** × **seam-ownership (MO record)** | §4.9 splits cleanly: steps 2–3 (CE + attribute-change-steps = fan-out) stay at the chokepoint; step 1 (MO record) originates at the ScriptSession seam (`apply_*` builds it from the surfaced oldValue). No MO-as-EcsDom-consumer (would invert invariant 3); no seam-owns-the-write (would break I1/I2/#341). |
| I3×#341 | seam-ownership × **lesson #341 chokepoint** | #341 consolidated ALL attribute writes onto `set_attribute` for derived-state reconcile + Attr-identity sync. Slice-1 **honors** #341 (the write + reconcile stay at the chokepoint) and adds only a non-mutating return-value surface for the record — no re-fork. |
| I4 | **same-value set fires; remove-missing does NOT** | DOM §4.9: "change an attribute" (set) queues a record even when newValue==oldValue (set fires unconditionally); "remove an attribute" only when the attribute existed. `set_attribute` already fires AttributeChange on same-value; `remove_attribute` already gates on `old_value.is_some()`. The record-build mirrors this gating (Option from the chokepoint = None ⇒ no record for remove-missing). |
| I5 | **oldValue timing** | §4.9 "change" step 1: oldValue = value BEFORE the write. The chokepoint captures it pre-write (`write_attribute_no_dispatch`); surfacing it is the record's oldValue. attributeOldValue gating is delivery-side (registry, already present). |
| I6 | **value-mode / `_without_dispatch` boundary** (Slice-2 territory, named here) | `input.value` text-mode live-value (8kHF) + `value_mode.rs` `set_attribute_without_dispatch` are NOT content-attribute changes → must produce NO record. Slice-1 touches only generic setAttribute/removeAttribute (always real content-attr writes); the exclusion is Slice-2's, flagged so Slice-1's mechanism doesn't accidentally capture it. |
| I7 | **delivery readiness (unfed pipeline)** | `MutationObserverRegistry::notify` already implements the §4.3.2 attributes branch (attributeFilter / attributeOldValue / subtree gating) + `observe()` option normalization — UNFED, exactly like childList pre-B1. Slice-1 feeds it; zero registry change. |
| I8 | **dual-runtime** | boa iframe setter (the one existing buffered record path, `record_mutation(SetAttribute)`) is S5-6-deletion-bound — out of scope, light-touch (do not model on it). |

---

## §3 Spec coverage map

Citations webref-verified 2026-06-28 (`dfn`/`body`/`heading dom`). §4.9 = Interface Element (hosts the attribute-mutation algorithms + "handle attribute changes"); §4.3.2 = Queuing a mutation record; §4.3 = MutationObserver.

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.9 handle attribute changes | step 1 | queue "attributes" record (localName, namespace, oldValue) | `apply_set_attribute`/`apply_remove_attribute` build record from chokepoint-surfaced oldValue | ✓ (namespace=null this slice, §9) | yes (name+value) |
| WHATWG DOM §4.9 handle attribute changes | steps 2–3 | CE reaction + attribute change steps (derived state) | `EcsDom::set_attribute` AttributeChange→ConsumerDispatcher fan-out (UNCHANGED) | ✓ | yes |
| WHATWG DOM §4.9 change an attribute | step 1 / 3 | oldValue=pre-write value; same-value fires | chokepoint captures oldValue pre-write; surfaces it | ✓ | yes |
| WHATWG DOM §4.9 set an attribute value | — | new attr vs change existing | dom-api `setAttribute` handler → `apply_set_attribute` | ✓ | yes |
| WHATWG DOM §4.9 remove an attribute | — | only-if-present record gating | `apply_remove_attribute` → chokepoint Option(oldValue)=None ⇒ no record | ✓ | yes |
| WHATWG DOM §4.9 `toggleAttribute()` method (F8: not a named "toggle an attribute" algorithm) | set / remove arms | force vs toggle (dispatches to "set an attribute value" / "remove an attribute by name") | dom-api `toggleAttribute` handler → apply_set/remove | ✓ | yes |
| WHATWG DOM §4.3.2 queue a mutation record | attributes branch | attributeName/attributeOldValue/attributeFilter gating | existing `MutationObserverRegistry::notify` (UNFED→fed) | ✓ | n/a (delivery) |
| WHATWG DOM §4.3.3 MutationRecord | — | attributeName/attributeNamespace/oldValue fields | session+registry `MutationRecord` (namespace field absent → `#11-mutation-observer-extras`) | ✓ name/oldValue; namespace deferred | n/a |

### §3.1 User-input touch audit

- `setAttribute(name, value)` / `removeAttribute(name)` / `toggleAttribute(name, force?)`: name + value user-controlled; name lowercased (F7: **DOM §4.9 `setAttribute()`/`toggleAttribute()` method step 2** "if this is in the HTML namespace and an HTML document, set qualifiedName to ASCII lowercase" — existing chokepoint behavior, unchanged; NOT HTML §4.9 which is Tabular data). The record's attributeName = the stored (lowercased) local name.
- **Adjacent pre-existing surfaces flagged for later slices** (NOT touched Slice-1): reflected IDL setters (Slice-2), `value_mode.rs` `_without_dispatch` (Slice-2 exclusion), Attr/NamedNodeMap (Slice-3). **F4 — sig-change migration surface is LARGER than the ~53 reflected setters**: the `EcsDom::set_attribute`/`remove_attribute` return-signature change is consumed by **~175 `.set_attribute(` + ~26 `.remove_attribute(` call-sites total** across crates (the ~53 reflected-IDL-setter family is a *subset*) — most just `let _ =` the widened return (non-breaking), but **enumerate ALL of them at impl** (`grep -rn '\.set_attribute(\|\.remove_attribute(' crates/`) so the signature change is total (no half-migrated caller). The *record-wiring* is the per-slice work; the *sig change* is total in Slice-1.

**Breadth**: K=1 spec (DOM), M=8 entries → single-PR scope (under K≥4/M≥20 floor). Justified: Slice-1 is the mechanism + one API family (generic setAttribute), umbrella-decomposed.

---

## §4 The design corner — chosen resolution vs rejected poles

The B0 audit (§4.4) named two poles; neither is correct as-is:

- **Pole A — MO as a ConsumerDispatcher consumer** (translate `AttributeChange` event → session record inside the EcsDom fan-out): tensions invariant I3 (inverts seam-ownership — MO record produced inside engine-internal `EcsDom`), and the same-value/`_without_dispatch` gating becomes consumer-side policy. REJECTED.
- **Pole B — seam owns the write** (route the write through the buffered `apply_set_attribute` that bypasses the chokepoint): the CURRENT broken state — drops the fan-out (I2) + breaks read-your-writes if deferred (I1) + re-forks #341. REJECTED (it's the bug Slice-1 fixes).
- **CHOSEN — chokepoint-surfaces-oldValue, seam-builds-record** (§1): write + fan-out + reconcile at `EcsDom::set_attribute` (I1/I2/#341 honored); MO record built at the dom-api seam from the surfaced oldValue (I3 honored); §4.9's own step-split (1 vs 2–3) made structural. This is the One-issue-one-way revival of the existing `apply_*` (rewritten to route through the chokepoint).

---

## §5 Changes (Slice-1)

### 5.1 `elidex-ecs` — `EcsDom::set_attribute` / `remove_attribute` surface oldValue
- Change `set_attribute(entity, name, value) -> bool` → return the **old value** (e.g. `-> Option<String>` = prior value, `None` if newly-added; keep a bool-success notion if a caller needs it — assess: the captured `old_value` at `write_attribute_no_dispatch` is already there, just discarded). `remove_attribute(entity, name) -> Option<String>` (the removed value, `None` if absent ⇒ no record).
- **Enumerate + migrate ALL callers** of the changed signatures (the ~53 reflected setters + dom-api handlers + boa + value_mode-adjacent) — most ignore the return (`let _ =`); the signature change must be total (§3.1 audit). The fan-out/reconcile/event behavior is UNCHANGED — only the return type widens.

### 5.2 `elidex-script-session::mutation` — rewrite + promote the producers
- **Rewrite** `apply_set_attribute`/`apply_remove_attribute` to call `EcsDom::set_attribute`/`remove_attribute` (the real chokepoint — full fan-out) and build the record from the **returned oldValue** (NOT the current direct-`attrs.set` + hand-rolled reconcile bypass). Promote to `pub`. Reuse the `MutationKind::Attribute` record shape (it already exists; consider an `attribute_record(target, name, old_value)` builder paralleling `character_data_record` — One-issue-one-way). `apply_set_attribute` always emits (same-value fires); `apply_remove_attribute` emits only when oldValue=Some (remove-missing = no record).
- This deletes the bypass comment + the manual `reconcile_attribute_derived_components`/`rev_version` re-fork (now done by the chokepoint).

### 5.3 `elidex-dom-api` — wire the generic handlers
- `element/props.rs` `setAttribute`/`removeAttribute` handlers + `element/attrs.rs` `toggleAttribute`: call the record-producing `apply_set_attribute`/`apply_remove_attribute` + `session.push_notify_record(record)` (mirror the B1 childList/characterData handlers). Preserve the `removeAttribute` `AttrEntityCache` evict.
- VM `vm/host/element_attrs.rs` stays marshalling-only. **F2 (MANDATED, not optional): the VM `native_element_toggle_attribute` (`element_attrs.rs:547-598`) currently re-implements the toggle force/present-check/set-remove algorithm in `host/` (a real DOM algorithm in the Layering-mandate target) calling `attr_set`/`attr_remove` shims — it does NOT route through the engine-indep dom-api `ToggleAttribute` handler, so Slice-1's record wiring would NOT cover JS `toggleAttribute`.** Slice-1 MUST converge it onto `invoke_dom_api("toggleAttribute")` → the dom-api handler (One-issue-one-way; deletes the host-side algorithm dup), so the toggle record path fires. (Mirror the setAttribute/removeAttribute marshalling-dispatcher convergence from B1.2b.)
- **AS-IMPLEMENTED drift correction (this memo originally claimed "VM already routes generic setAttribute/removeAttribute via `invoke_dom_api`" — accurate for `setAttribute` only).** Grep-verified at impl: `native_element_set_attribute` routes via `invoke_dom_api("setAttribute")` ✓, but `native_element_remove_attribute` calls the `attr_remove` shim (`dom.remove_attribute` chokepoint + a **VM-local `Attr`-wrapper snapshot** — `attr_states.detached_value` freeze, the §4.9.2 / Chrome-FF removal-time snapshot — which the engine-indep handler cannot do). So Slice-1 ALSO converges `native_element_remove_attribute` onto `invoke_dom_api("removeAttribute")` (same shape as the F2 toggle convergence), preserving the wrapper snapshot VM-side as marshalling that brackets the call (snapshot before → `invoke_dom_api` → freeze/invalidate after — extracted into shared `snapshot_attr_wrapper` / `freeze_detached_attr_wrapper` helpers reused by `attr_remove`). `attr_remove` itself stays (record-less) for the ~14 **reflected boolean-attribute detach** call-sites (`el.hidden=false`, `<input>.checked=false`, …) — those are B2-Slice-2 (reflected IDL setters). The VM-local Attr-wrapper detach asymmetry vs `removeAttributeNode` (its own inline snapshot) is unchanged here — B2-Slice-3. Wrapper-freeze regression locks: `tests_element_attributes::attr_held_across_remove_*` stay green.

### 5.4 Delivery — already wired
- `push_notify_record` → `drain_notify_records` (`dom_bridge.rs`) → `queue_mutation_record` → §4.3 microtask → `MutationObserverRegistry::notify` (attributes branch, attributeFilter/attributeOldValue/subtree — UNCHANGED). Zero registry/delivery change (I7).

---

## §6 Tests
- MO-driven VM tests (mirror the B1 childList/characterData test harness): `setAttribute` new attr → 1 `attributes` record (target, attributeName, oldValue=null); `setAttribute` change → oldValue=prior (with `attributeOldValue:true`) / null (without); **same-value `setAttribute` → record still fires** (I4); `removeAttribute` present → record (oldValue=prior); **`removeAttribute("missing")` → NO record** (I4); `toggleAttribute` add/remove arms; `attributeFilter` gating (only listed attrs); `subtree:true` on a descendant attr; `attributeOldValue:false`→oldValue null.
- **Fan-out-preserved regression** (the I2 lock): a `setAttribute("style"/"id"/iframe-attr)` through the new path STILL drives the derived-state consumer (InlineStyle / id-index / base-url) — assert the reconcile didn't regress (the whole point of routing through the chokepoint vs the old bypass).
- engine-indep unit: `apply_set_attribute`/`apply_remove_attribute` route through the chokepoint (assert fan-out fires) + record shape.

## §7 Files + 1000-line
- `elidex-ecs/dom/attribute.rs` (return-sig), `elidex-script-session/mutation/mod.rs` (rewrite producers + builder), `elidex-dom-api/element/props.rs`+`attrs.rs` (wire), `elidex-js/vm/host/element_attrs.rs` (confirm marshalling). + the caller-migration sweep for the sig change. + MO tests. Re-check each file's LoC at impl; `mutation/mod.rs` (~910) is the one to watch.

## §8 Process
fmt → `mise run ci` → pre-push 6-stage (`/pre-push`) → push + `gh pr create` → `/external-converge`. At merge: update [[project_b1-mutationobserver-next-task]] (B2-Slice-1 done, NEXT Slice-2) + ledger + MEMORY.md + [[reference_js-tree-mutations-not-recorded]] (attribute Slice-1 ✅). Mirror = #426/#424 (record path) + #393 (handler wiring).

## §9 Deferrals
- **Slice-2 (reflected IDL setters)** + **Slice-3 (Attr/NamedNodeMap)** — own plan-reviews (umbrella §0).
- **`attributeNamespace`** record field → existing slot `#11-mutation-observer-extras` (verified OPEN in `project_open-defer-slots.md`; both session+registry `MutationRecord` lack the field; Slice-1 records the namespace=null shape). **Why**: orthogonal record-struct-field addition (+ the `*AttributeNS` write APIs that would set a non-null namespace are themselves unimplemented). **Re-eval trigger**: a WPT/site asserting `MutationRecord.attributeNamespace`, OR when `setAttributeNS` is implemented. **Date**: fold into the slot's next touch (no separate date — rides `#11-mutation-observer-extras`).
- **`*AttributeNS`/`*NamedItemNS`** — UNIMPLEMENTED (no call-sites); out of scope until implemented.
- boa iframe buffered path — S5-6 deletion-bound, no slot.
