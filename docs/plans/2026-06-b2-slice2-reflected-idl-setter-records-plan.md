# B2-Slice-2 — reflected IDL setter attribute-write MutationRecords (plan-memo)

**Status**: PRE-IMPLEMENTATION plan-memo → `/elidex-plan-review` (edge-dense per-slice rule). (2026-06-29)
**Program**: B (F3 mutation-path). B2 is the **LAST record gap**; Slice-1 (#428 `d09829a5`) established the record-producing attribute seam for the generic `setAttribute`/`removeAttribute`/`toggleAttribute` DOM methods. Slice-2 routes the **reflected IDL setters** (+ classList/dataset/style/hyperlink) through that same seam so they emit DOM §4.9 "attributes" records. **Largest blast radius.** Slice-3 (Attr/NamedNodeMap) follows. (dialog-close = method-driven, deferred to a new slot — §9 F1.)
**Base**: `d09829a5` (post-#428). Worktree `elidex-b2-slice2`, branch `b2-slice2` (off `origin/main`).
**Umbrella**: `docs/plans/2026-06-b2-attribute-records-umbrella-and-slice1.md` §0/§9 (Slice-2 carve).

---

## §0 What Slice-1 already built (reused verbatim — NO new mechanism)

The seam Slice-2 routes more writers through:

- **`EcsDom::set_attribute(entity, name, value) -> AttributeWrite{did_set, old_value}`** / **`EcsDom::remove_attribute(entity, name) -> Option<String>`** — the attribute-write chokepoint (`crates/core/elidex-ecs/src/dom/attribute.rs`). Fires the `MutationEvent::AttributeChange` fan-out (§4.9 steps 2–3) + surfaces the pre-write `oldValue`. **Signature already landed in Slice-1 — Slice-2 changes NO EcsDom signature.**
- **`apply_set_attribute(dom, entity, name, value) -> Option<MutationRecord>`** / **`apply_remove_attribute(dom, entity, name) -> Option<MutationRecord>`** (`crates/script/elidex-script-session/src/mutation/mod.rs:830/853`, both `pub`) — the record-producing primitives: call the real chokepoint (full fan-out preserved) + build the §4.9 step-1 record from the surfaced oldValue via `attribute_record(target, name, old_value)` (`mutation/mod.rs:246`). `apply_set_attribute` always records on a landed write (same-value fires, I4); `apply_remove_attribute` records only when something was removed (remove-missing = `None` = no record).
- **Delivery (UNCHANGED, I7)**: `session.push_notify_record(record)` → drain → `queue_mutation_record` → §4.3 microtask → `MutationObserverRegistry::notify` (attributes branch: attributeFilter / attributeOldValue / subtree — already implemented). Two drain points exist:
  - `invoke_dom_api` Phase 2.5 (`dom_bridge.rs:510`) — auto-drains after every dom-api handler call.
  - `commit_notify_records` (`dom_bridge.rs:53`; renamed from `commit_range_mutation_records` as-built, /simplify Stage 3 — it now serves a 4th non-range user) — the push+drain-as-one-step helper for natives that bypass `invoke_dom_api` (Range/Selection/Text-splice natives + the **host-shim reflected setters** join it, §4).

**The whole Slice-2 = wire more writers to `apply_set_attribute`/`apply_remove_attribute`.** That is data-flow wiring, not a mechanism or layering change (F3).

---

## §1 First-principles ideal (the convergence)

DOM §4.9 "handle attribute changes" is the single algorithm EVERY attribute mutation funnels through; step 1 = queue the "attributes" MO record. Slice-1 wired step-1 for generic `setAttribute`/`removeAttribute`/`toggleAttribute`; the ideal end state is that **every** content-attribute write — reflected IDL setter, classList/dataset/style, hyperlink — converges on the one record-producing primitive, regardless of which IDL surface drove it. "One issue (the §4.9 step-1 record), one way (`apply_*`)."

Two **layer-appropriate wiring points** (NOT two mechanisms — both call the same `apply_*`):

1. **VM host (marshalling layer, F3)** — the two existing shims **`attr_set`/`attr_remove`** (`crates/script/elidex-js/src/vm/host/element_attrs.rs:106/197`) become record-producing: each wraps `apply_*` in `with_session_and_dom` + `push_notify_record`, then drains (mirroring `invoke_dom_api` Phase 2→2.5). **Every** reflected-setter write in `vm/host/` routes through these two shims (the ~50 direct `dom().set_attribute(...)` sites + 11 reflect macros migrate onto `attr_set`; the 16 `attr_remove` sites are auto-covered the moment the shim records). Reflected setters **stay in `host/`** — they are pure marshalling (reflect IDL prop → known-lowercase content-attr write, no DOM algorithm), so they do NOT move to `invoke_dom_api` (F3).
2. **dom-api handlers (engine-independent layer)** — the single per-subsystem write helper builds the record via `apply_*` + `session.push_notify_record` (the `invoke_dom_api` Phase 2.5 drains automatically — these handlers already carry `session`):
   - classList/relList/htmlFor → `set_token_string` (`class_list.rs:82`)
   - dataset → `DatasetSet` (`element/attrs.rs:364`) + `DatasetDelete` (`element/attrs.rs:390`)
   - style → `sync_to_attribute` (`style.rs:133`)
   - hyperlink href (+ all URL-decomposition setters) → `write_href_attr` (`element/href_accessor.rs:~140`)

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

Citations webref-verified 2026-06-29 (`coverage-map`; re-verify at impl). **DOM §4.9 title = "Interface Element"** — "handle attribute changes" / "change / set / remove an attribute" are algorithm **dfns within §4.9, NOT its section title** (the Step/Branch columns name the dfn). §4.3.2 = queue a mutation record; §4.3.3 = MutationRecord; HTML §2.6.2 = reflect (IDL↔content attribute, the `[Reflect]` extended attribute; reflect concept itself = §2.6.1); §4.6.3 = API for hyperlink elements; §3.2.6.6 = data-* attributes; CSSOM §6.6.1 = CSSStyleDeclaration. **Excluded (NOT-touched) surfaces** = §2 (I1 value-mode / I6 boa) + §9 (dialog method-path / Attr-NamedNodeMap) — kept out of this touched-sections map. **`apply_*` = `apply_set_attribute`/`apply_remove_attribute` (Slice-1, `pub`).**

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.9 Interface Element | handle-attribute-changes step 1 | queue "attributes" record (localName, oldValue) | host `attr_set`/`attr_remove` + dom-api write helpers → `apply_*` | ✓ (namespace=null) | yes |
| WHATWG DOM §4.9 Interface Element | handle-attribute-changes steps 2–3 | CE reaction + attribute-change steps (fan-out) | `EcsDom::set_attribute` (UNCHANGED) | ✓ | yes |
| WHATWG DOM §4.9 Interface Element | change / set / remove an attribute (dfns) | reflected setter → set arm (`attr_set`) or remove arm (`attr_remove`) | host shims + dom-api helpers | ✓ | yes |
| WHATWG HTML §2.6.2 reflect (extended attribute) | reflect set/remove | string/bool/long reflected IDL setters | host reflect macros → `attr_set`/`attr_remove` | ✓ | yes |
| WHATWG HTML §4.6.3 API for hyperlink elements | href + URL-member set | a/area href + protocol/host/… reconstruct + write `href` | dom-api `write_href_attr` → `apply_set_attribute` | ✓ (attributeName="href") | yes |
| WHATWG HTML §3.2.6.6 data-* attributes | set / delete | `dataset[name]=` / `delete dataset[name]` (camel→kebab) | dom-api `DatasetSet`/`DatasetDelete` → `apply_*` | ✓ (attributeName=`data-*`) | yes |
| CSSOM §6.6.1 CSSStyleDeclaration | setProperty/removeProperty/cssText | inline-style serialize → `style` attr | dom-api `sync_to_attribute` → `apply_set_attribute` | ✓ (attributeName="style") | yes |
| WHATWG DOM §4.3.2 queue a mutation record | attributes branch | attributeName/attributeOldValue/attributeFilter/subtree gating | existing registry (UNFED→fed) | ✓ | n/a (delivery) |
| WHATWG DOM §4.3.3 MutationRecord | — | attributeName / oldValue fields | session+registry record (namespace deferred §9) | ✓ name/oldValue | n/a |

**Breadth**: K=3 specs (DOM, HTML, CSSOM), M=9 entries — a single mechanism (Slice-1 `apply_*`) drives every row; wide-but-shallow **data-flow** sweep. Single PR justified: B2's umbrella already decomposed by IDL-surface category (generic = Slice-1 / reflected = Slice-2 / Attr-NamedNodeMap = Slice-3); Slice-2 is the umbrella-approved per-PR base-case, and re-splitting it by spec family would fragment the one `apply_*` mechanism arbitrarily (infinite regress per the per-PR base-case rule).

### §3.1 User-input touch audit
Every site takes user-controlled values (`el.id = userStr`, `el.style.cssText = userStr`, `el.dataset.x = userStr`, `a.href = userStr`). All route through the chokepoint's existing write (value stored verbatim; no new sanitization). The record's `oldValue` exposure is gated by `attributeOldValue:true` (existing registry, I7-delivery). No new trust boundary — Slice-2 adds record *observation* of writes that already occurred.

---

## §4 The two wiring patterns (detail)

### 4.1 Host shim (`attr_set`/`attr_remove`) — self-commit (bypasses `invoke_dom_api`)
Reflected setters call the shims directly (not via `invoke_dom_api`), so the shim must commit (push+drain) itself (no Phase 2.5). **As-built** (/simplify Stage 3 converged it onto the shared `commit_notify_records` so push+drain stay one indivisible step — no hand-split that a future edit could strand):

```rust
pub(super) fn attr_set(ctx, entity, name, value) -> bool {
    let record = ctx
        .host_if_bound()
        .and_then(|host| apply_set_attribute(host.dom(), entity, name, value));
    let did_set = record.is_some();
    ctx.vm.commit_notify_records(record.into_iter().collect()); // 0-or-1 record
    did_set
}
```
`attr_remove` keeps snapshot→remove→freeze, then commits the (0-or-1) record after the freeze (freeze = VM wrapper state, commit's drain = microtask queue — independent, I9).

### 4.2 dom-api handler write helpers — push only (Phase 2.5 drains)
These handlers already receive `session: &mut SessionCore` (currently `_session`, unused). Un-underscore + build/push:
```rust
// e.g. set_token_string (classList), DatasetSet/DatasetDelete, sync_to_attribute (style), write_href_attr (hyperlink)
if let Some(record) = apply_set_attribute(dom, entity, attr_name, value) {
    session.push_notify_record(record);
}
```
Thread `session` into the shared write helper where it is not already a parameter (`set_token_string`, `sync_to_attribute`, `write_href_attr` / `set_href` / `href_url_set_component`). The `invoke_dom_api` caller drains. (All four helpers' callers are confirmed session-bearing `DomApiHandler::invoke` bodies — plan-review Agent 2 verified no dispatcher/parser caller; `close_the_dialog` was the lone exception with an off-`invoke_dom_api` shell caller → deferred §9 F1.)

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

### 5.3 `elidex-dom-api` — wire the 4 handler write helpers
- `class_list.rs` `set_token_string` (`:66/82`): thread `session`, `apply_set_attribute` + push. Covers classList/relList/htmlFor (`TokenListHandler`).
- `element/attrs.rs` `DatasetSet` (`:355/364`) + `DatasetDelete` (`:381/390`): un-underscore `session`, `apply_set_attribute`/`apply_remove_attribute` + push.
- `style.rs` `sync_to_attribute` (`:119/133`): thread `session`, `apply_set_attribute` + push (preserve the post-write `InlineStyle` re-insert, I10).
- `element/href_accessor.rs` `write_href_attr` (`:~140/149`): thread `session` through `set_href` + `href_url_set_component`, `apply_set_attribute` + push.
- Imports: `class_list.rs`/`style.rs`/`href_accessor.rs` add `apply_set_attribute`/`apply_remove_attribute` (already imported in `element/attrs.rs`).
- **NOT touched (deferred §9 F1)**: `dialog.rs` `close_the_dialog` — its shell `method=dialog` caller is off the `invoke_dom_api` drain path → new slot `#11-method-driven-attribute-records`.
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
- **attributeFilter / subtree / attributeOldValue** gating on a reflected setter (one representative) — confirms delivery path unchanged.
- **Fan-out-preserved regression (I-lock)**: a reflected `el.className=`/`a.href=` STILL drives its derived consumer (class-index / base-url) — the record wiring did not regress the chokepoint fan-out.
- **boa unaffected** (I6): no new boa test; existing boa attribute tests stay green.

---

## §7 Touched-file list + 1000-line check
`vm/host/element_attrs.rs` (shims) + the ~14 `vm/host/*_proto.rs`/`html_input_value.rs`/`form_state_sync.rs`/`canvas/mod.rs` (macro/site migration) + `elidex-dom-api/{class_list,style}.rs` + `element/{attrs,href_accessor}.rs` (handler wiring) + MO tests. **LoC (measured 2026-06-29, plan-review Agent 5)**: closest-to-1000 touched files = `html_input_proto.rs` 964, `html_element_proto.rs` 913, `vm/host/element_attrs.rs` 716, `class_list.rs` 566, `href_accessor.rs` 551, `html_input_value.rs` 537, `style.rs` 529; `mutation/mod.rs` 927 is reused **verbatim (NO Slice-2 edit)**. The dominant §5.2 edit is a macro-body swap (`dom().set_attribute(...)` → `attr_set(ctx, ...)`) = **net-neutral** (no >50-LoC add), so no touched file crosses 1000 → **no prereq split warranted**. Re-confirm at impl; any real cohesion seam crossing 1000 → standalone prereq split (NOT bundled, CLAUDE.md).

## §8 Process
fmt → `mise run ci` → `/pre-push` (6-stage: simplify/code-review/review/elidex-review) → push + `gh pr create` → `/external-converge`. At merge: update `[[project_b1-mutationobserver-next-task]]` (Slice-2 done, NEXT Slice-3) + `[[m4-12-landings-ledger]]` + MEMORY.md + `[[reference_js-tree-mutations-not-recorded]]`. Mirror = #428 (the shim/handler pattern) + #393 (handler wiring).

## §9 Deferrals + scope decisions
- **Slice-3 (Attr/NamedNodeMap)** — own plan-review. `Attr.value` setter + `setNamedItem`/`removeNamedItem` + `setAttributeNode`/`removeAttributeNode` (`vm/host/{named_node_map,attr_proto}.rs` + `element_attrs.rs:453/574` + dom-api `char_data/attr.rs:161/230/318`) + the VM-local Attr-wrapper detach asymmetry.
- **SCOPE DECISION — dialog.close() DEFERRED → NEW slot `#11-method-driven-attribute-records`** (plan-review F1, Agent 2+3; user-approved 2026-06-29): `close_the_dialog`'s `open` removal (`dialog.rs:80`) is the one non-reflected-setter production direct-write, but it is **NOT mechanism-compatible** with this slice. `close_the_dialog` is a `pub fn` (not a `DomApiHandler`) with two callers OFF the `invoke_dom_api` drain path: VM `dialog.close()` (`html_dialog_proto.rs:329`, has `ctx.vm` → could drain) AND **shell `method=dialog` form-submit (`form_input.rs:170`, has `PipelineResult.session`+`dom` but NO `ctx.vm`)**. The scratch→queue drain (`drain_notify_records`) is a VM-host-only method unreachable from the shell, so a record pushed on the form-submit path is **silently cleared** by the `SessionCore::flush` leak-guard → silent loss. Wiring only the JS `close()` path would itself be a strangler (same algorithm records inconsistently). The real missing piece = a **shell-reachable shell-driven-mutation→MO delivery path**, a distinct architectural concern (dialog is its first instance), broader than this `apply_*`-seam data-flow slice. **Why deferred**: needs shell→MO delivery infra, not the seam-wiring mechanism. **Re-eval trigger**: shell-driven-mutation MO delivery lands (S5 VM-drives-shell event-loop = natural home), OR a `method=dialog`/`dialog.close()` MutationObserver WPT/test. **Date**: 2026-08-29. **Registers in `project_open-defer-slots.md` at landing.** (NB: `dialog.open=true/false`, the reflected boolean setter (`html_dialog_proto.rs:139`), ALREADY records via the **Slice-1** `invoke_dom_api("setAttribute"/"removeAttribute")` path — independent of this slice; only the method-driven `close_the_dialog` path defers. So the reflected-setter scope stays clean+complete — a tracked cross-PR boundary like Slice-3, not a strangler.)
- **details name-group exclusion — already covered**: `html_details_proto.rs` routes `open` set/remove via `invoke_dom_api("setAttribute"/"removeAttribute")` (Slice-1) — no Slice-2 work.
- **anchor/area/img/link/meta/script non-href reflected attrs — already covered**: they use `reflect_setter!` → `invoke_dom_api("setAttribute")` (Slice-1). Only the URL-backed `href` family (hyperlink mixin) needs wiring.
- **boa `SetClassName`/`SetId`** — the record-less exemption is scoped to **current VM-unreachability** (the VM `className`/`id` path uses host `reflected_string_set`→`attr_set`, I6), **NOT** the handlers' engine-indep file location (they sit in `element/attrs.rs` beside the wired `DatasetSet`/`DatasetDelete`, so two siblings diverge for boa's lifetime — sanctioned because no *live* surface reaches them). boa-only + S5-6-deletion-bound → light-touch, not wired, no slot (closes at boa removal). ⚠ a non-boa caller of `className.set`/`id.set` appearing before boa removal must wire them (exemption rests on reachability, not permanence).
- **`attributeNamespace` record field** → rides `#11-mutation-observer-extras` (Slice-1 deferral; namespace-less shape unchanged).
- **CE↔MO ordering** (`#11-ce-reaction-mutation-observer-ordering`) + **attribute-name casing** (`#11-attribute-name-html-namespace-casing`) — Slice-1 slots, inherited unchanged (I7/I8); Slice-2 does NOT touch either.
