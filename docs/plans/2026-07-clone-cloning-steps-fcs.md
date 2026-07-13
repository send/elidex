# Plan — `#11-clone-cloning-steps-event`: form-control cloning steps (FCS copy)

**Slot**: `#11-clone-cloning-steps-event` (carved #331; clone-time CE upgrade enqueue already done in #331 R13/R14 — this slice is the **narrow FCS value-copy** remainder).
**Lane**: DOM/form (OWN = `elidex-dom-api` + `elidex-form` + their thin VM shims).
**Status**: plan-memo for `/elidex-plan-review`; implementation deferred to a following session.

## §0.5 Spec citation table

| Cite | Anchor |
|---|---|
| WHATWG DOM §4.4 Interface Node — "clone a node" | `#concept-node-clone` (step 3 = cloning-steps hook `#concept-node-clone-ext`) |
| WHATWG HTML §4.10.5 The input element — cloning steps | `#the-input-element` |
| WHATWG HTML §4.10.11 The textarea element — cloning steps | `#the-textarea-element` |

---

## §1 Goal (spec)

Implement the **cloning steps** hook (DOM §4.4 "clone a node" step 3: *"Run any
cloning steps defined for node in other applicable specifications and pass node,
copy, and subtree as parameters"*) for the two form controls that define one:

- **`<input>`** (HTML §4.10.5) — *propagate the value, dirty value flag,
  checkedness, dirty checkedness flag, and indeterminateness from node to copy*.
- **`<textarea>`** (HTML §4.10.11) — *propagate the raw value and dirty value flag
  from node to copy*.

**Observable gap today**: `FormControlState` (FCS) is **not** in the clone-policy
copy-set (`clone.rs:39` copies only `TagType / TextContent / CommentData /
DocTypeData / Attributes / Namespace / InlineStyle / IframeData`). A cloned
`<input>`/`<textarea>` therefore has **no** FCS after the ECS clone; it gets a
**default** FCS re-derived from attributes when `FormControlReconciler::handle_insert`
runs on insertion (`reconciler.rs:84`). So a user-edited or JS-set live value /
checkedness / indeterminate state is **lost** across `cloneNode`:
`sourceInput.value='x'; sourceInput.cloneNode().value` returns the `value`-attribute
default, not `'x'`. This slice makes the clone carry the source's live form state.

`<select>` defines **no** cloning steps (selectedness re-derives from the `selected`
content attribute + cloned `<option>` children); the copy set is input + textarea only.

---

## §2 Coupled invariants (edge-dense enumeration)

This is edge-dense: a **novel ECS seam** (there is no existing clone→form-state
consumer) under a **crate-boundary constraint**, intersecting ≥3 invariant axes.
The design must satisfy all four simultaneously; each pairwise intersection is the
place a prose-only plan would leak a gap:

| # | Invariant | What it forces |
|---|---|---|
| I1 | **Crate boundary** `elidex-dom-api ⊥ elidex-form` (dep runs form→dom-api; verified: `elidex-form` absent from `elidex-dom-api/Cargo.toml`) | The cloner (dom-api) cannot read/write `FormControlState` at all → the copy MUST be done by a consumer at/above `elidex-form`. |
| I2 | **Lane boundary** — no B1-core touch (`crates/core/elidex-ecs/src/dom/` off-limits) | No `MutationEvent::Clone` variant (that enum is B1 core) → the seam is NOT a mutation event. |
| I3 | **Detached-clone timing** — `detachedInput.cloneNode().value` must read correctly *before* any insertion | The FCS copy MUST be **synchronous at clone time**, not a deferred event consumed on insertion. |
| I4 | **Reconciler composition** — `handle_insert` re-derives a *default* FCS for FCS-absent nodes (`reconciler.rs:84`) but SKIPS nodes that already have one (`reconciler.rs:90` absence guard) | The copy must **CREATE** the dst FCS at clone time so the later insert-time reconciler skips it (no overwrite). |
| I5 | **Subtree / shadow-inclusive coverage** (plan-review Axis 2 MIN) — a deep clone's form controls include ones **encapsulated in a replicated clonable shadow root** (`clone.rs:168-216` grows the worklist over shadow + template-content descendants) | The linkage/consumer must span the **full clone worklist**, not just the root — every form-control dst in the shadow-inclusive subtree is copied, synchronously (composes with I1: linkage carries all pairs; I3: all synchronous). The Option-B `for_each_shadow_inclusive_descendant(clone_root)` re-walk covers this by construction. |

**Pairwise intersections (the cross-cutting mechanism):**

- **I1 × I3** — the copy runs **synchronously in the VM cloneNode shim right after
  the dom-api clone returns** (mirrors the existing `apply_clone_creation_ce_semantics`
  marshalling step), not via a cross-crate event → detached `.value` correct AND no
  dom-api→form dependency.
- **I1 × I2** — the src→dst *linkage* the copy needs is carried by **dom-api-owned
  data** (a pairs return, or a dom-api marker component), never by a core
  `MutationEvent` → both boundaries held by keeping the linkage in dom-api / the
  copy in elidex-form.
- **I3 × I4** — a synchronous *CREATE* of the dst FCS at clone time is exactly what
  makes the insert-time absence guard a no-op on the clone → the copy and the guard
  share one invariant ("FCS present ⇒ don't re-derive"), co-located in elidex-form
  (I1 × I4).

**Constraint vs the #331 §6.1 sketch**: that sketch proposed the cloner emitting
`MutationEvent::Clone { pairs }` with a deferred per-layer consumer. I2 rules out
the event; I3 rules out "deferred." The forced design (dom-api exposes linkage →
elidex-form copies synchronously → VM marshals) is strictly cleaner and is the
existing CE-clone precedent's shape.

---

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG DOM §4.4 "clone a node" | step 3 | run cloning steps (node, copy, subtree) | VM `native_node_clone_node` (`node_methods_extras.rs:197`) → `elidex_form::apply_clone_form_state` (NEW) at clone time | ✓ | yes (`cloneNode`) |
| WHATWG HTML §4.10.5 input cloning steps | propagate | value | `apply_clone_form_state` (NEW) copies `FormControlState.value` | ✓ | yes |
| WHATWG HTML §4.10.5 input cloning steps | propagate | dirty value flag | copies `dirty_value` | ✓ | yes |
| WHATWG HTML §4.10.5 input cloning steps | propagate | checkedness | copies `checked` | ✓ | yes |
| WHATWG HTML §4.10.5 input cloning steps | propagate | dirty checkedness flag | **UNMODELED** in `FormControlState` → defer `#11-input-dirty-checkedness-flag` | ✗ (deferred, §5.1) | yes |
| WHATWG HTML §4.10.5 input cloning steps | propagate | indeterminateness | copies `indeterminate` | ✓ | yes |
| WHATWG HTML §4.10.11 textarea cloning steps | propagate | raw value | copies `value` (= textarea raw value) | ✓ | yes |
| WHATWG HTML §4.10.11 textarea cloning steps | propagate | dirty value flag | copies `dirty_value` | ✓ | yes |

**Breadth**: K=2 specs, M=8 entries → single-PR scope (below K≥4/M≥20 split threshold; coverage-map verdict `ok`).

### §3.1 User-input touch audit

- `native_node_clone_node` (`node_methods_extras.rs:197`, `cloneNode`): the source
  control's live state (typed value / toggled checkedness / set indeterminate) is
  user-controllable and is what this slice propagates.
- `apply_clone_form_state` (NEW, elidex-form): reads source FCS, creates dst FCS —
  the five propagated fields are user-controllable; no new parsing of untrusted
  strings (values are copied verbatim, already sanitized on the source).
- Adjacent pre-existing: `FormControlReconciler::handle_insert` absence guard
  (`reconciler.rs:90`) — exposure UNCHANGED (it already skips FCS-present entities;
  this slice adds a new producer of "FCS-present at insert time," which the guard
  already handles by construction).

---

## §4 The src→dst linkage: seam options

The cloner (`clone_node_with_shadow_honor`, `clone.rs:149`) already builds
`pairs: Vec<(Entity, Entity)>` internally (src→dst for the whole subtree) and
discards them at the boundary (returns `Option<Entity>`). FCS cannot copy in the
cloner (I1 — dom-api can't read the source's FCS), only carry the **link** to the
source for a downstream elidex-form consumer.

**Precedent correction (plan-review Axis 2 IMP)**: the CE-clone precedent is
structurally **Option B, not Option A**. `propagate_ce_identity` (`clone.rs:256`)
writes a *per-dst component* (`CustomElementState`) in the worklist, then
`apply_clone_creation_ce_semantics` **re-walks the clone root**
(`for_each_shadow_inclusive_descendant(clone_root)`) reading each dst's OWN
component — it consumes only the standard `Option<Entity>` return, **not** pairs.
**No current dom-api→VM caller threads `pairs` across that boundary.** So an earlier
draft's claim that Option A "is not a new precedent / the CE step already reaches
past name-dispatch" was factually wrong: Option A *introduces* new cross-boundary
pairs threading; Option B is the faithful CE twin.

- **Option B (recommended — CE-precedent twin, ECS-native-first).** The cloner
  attaches a **transient dom-api marker `ClonedFrom { source }` (NEW)** on each dst
  in the worklist (beside `propagate_ce_identity`). The elidex-form consumer
  `apply_clone_form_state(dom, clone_root)` (NEW) **re-walks the clone root**
  (`for_each_shadow_inclusive_descendant`, mirroring the CE pass — NOT a global hecs
  query, which risks a stale-global-marker footgun), reads each dst's
  `ClonedFrom.source`, copies FCS, and **removes the marker**. Per CLAUDE.md
  ECS-native-first (*"observer registry → marker component + query"*,
  *"class-owned state → ECS component on entity"*): provenance ("dst cloned from
  src X") is naturally component-on-entity data. This is the mandated idiom, the
  exact CE-pass shape, and needs no new dom-api→VM threading.
- **Option A (leak-free fallback — pure data return).** Thread the existing `pairs`
  out (`clone_node_recording` (NEW) → `Option<(Entity, Vec<(Entity, Entity)>)>`);
  VM calls `apply_clone_form_state(dom, &pairs)`. No marker, no cleanup obligation.
  Cost: introduces the new dom-api→VM pairs-threading noted above (less ECS-native);
  but the provenance here is **ephemeral** (needed only for the duration of the
  clone op), which a return value arguably models more honestly than a component
  that must then be cleaned up.
- **Option C — session buffer.** Rejected pre-review (cross-cutting session state
  for a clone-local concern).

### §4.1 The deciding factor — marker cleanup vs I1 (impl-time caller audit)

Option B's marker carries a **source `Entity` ref** and MUST be removed, else a
leaked `ClonedFrom` becomes a **dangling entity ref** after the source despawns
(the CLAUDE.md stale-entity-ref hazard) — unlike CE's `CustomElementState`, which is
legitimate persistent state, not a scratch link. Removal is done by the consumer;
so **every clone-algorithm entry point must run the consumer** (this coincides with
*correctness* — `importNode`, `Range.cloneContents`/`extractContents`, and
declarative-shadow cloning all invoke DOM §4.4 "clone a node" and so all owe the
cloning-steps copy anyway). The hazard is **I1**: a clone path that runs *entirely
inside dom-api / the parser* (e.g. declarative-shadow clone in the HTML parser)
**cannot call the elidex-form consumer** and would leak the marker.

**Impl-time audit (next session, deciding factor for B vs A):** enumerate the
callers of `clone_node_with_shadow_honor` and each clone-algorithm entry point;
confirm each reaches an elidex-form-side consumer at its VM/session marshalling
boundary. If all do → **Option B** (ECS-native-first, CE-twin). If any dom-api /
parser-internal clone path cannot (I1) → either make `ClonedFrom` generation-safe
+ swept at that boundary, or use **Option A** there (a return value can't leak).
The plan **recommends Option B contingent on that audit, with Option A as the
leak-free fallback** — this is the load-bearing implementation decision.

---

## §5 The copy (engine-independent, `elidex-form`)

New `elidex-form` public fn `apply_clone_form_state` (NEW) — the algorithm home per
the Layering mandate. **Module home** (plan-review Axis 5 MIN): NOT `elidex-form/src/lib.rs`
(994 LoC — adding the fn crosses the 1000-line convention). Place it in a **new
`elidex-form/src/clone.rs`** module; the fields it touches (`dirty_value` lib.rs:406,
`char_count` lib.rs:444) are `pub(crate)`, so a sibling module inside the crate can
access them. (`char_count` re-synced via the `pub(crate)` `update_char_count`.)

```rust
/// HTML §4.10.5 / §4.10.11 cloning steps. Re-walk the clone root; for each dst
/// carrying a `ClonedFrom { source }` marker whose source has a form-control
/// FormControlState, CREATE dst's FormControlState by propagating the cloning-step
/// fields, then REMOVE the marker. No-op for non-form-control srcs and for a src
/// with no FCS (a never-materialized control has only its attribute default, which
/// the reconciler re-derives identically on the clone).
pub fn apply_clone_form_state(dom: &mut EcsDom, clone_root: Entity);   // Option B (recommended)
// Option-A fallback: apply_clone_form_state(dom: &mut EcsDom, pairs: &[(Entity, Entity)]).
```

Per-pair copy of the **modeled** cloning-step fields (`FormControlState` field names
verified in `lib.rs`): input → `value` / `dirty_value` / `checked` / `indeterminate`;
textarea → `value` (raw value) / `dirty_value`. `char_count` re-synced from the copied
`value` (cache invariant). `default_value` / `default_checked` come from the clone's own
copied `Attributes` (unchanged by cloning steps).

**Copy ctor shape** (open question O2, §9): build the dst FCS via `from_element(tag,
attrs)` (correct defaults for every non-cloning-step field: name / readonly / min /
max / step / pattern …, consistent with the clone's copied attributes) and then
overwrite the ≤4 cloning-step fields from the source. Recommended over a dedicated
`clone_form_state(src_fcs)` ctor because the former keeps non-step fields
attribute-consistent (exactly what the reconciler would have produced).

### §5.1 Deferred sub-facet — dirty checkedness flag

`FormControlState` has **no** dirty-checkedness field (verified: no `dirty_check*`
symbol in `elidex-form/src/lib.rs`). Propagating it needs the field **plus** its
producers/consumers (set on user toggle; consulted by the `checked` content-attribute
change steps and by form reset) — a broader FCS-modeling change, not a clone-local
copy. **Defer** → new slot `#11-input-dirty-checkedness-flag`. This slice copies the
four modeled fields; a cloned control's *observable* value/checkedness/indeterminate
are correct — only a dirtied clone's subsequent `checked`-attribute-change behavior
differs, which is already the pre-existing unmodeled behavior for non-cloned controls
(no regression).

---

## §6 Composition safety (the reconciler absence guard)

`FormControlReconciler::handle_insert` (`reconciler.rs:84`) walks the inserted subtree
and, **per descendant, skips any entity that already has a `FormControlState`**
(`reconciler.rs:90` `if …get::<&FormControlState>(entity).is_ok() { continue; }`). So:

1. dom-api clone → dst has no FCS.
2. `apply_clone_form_state` (synchronous, clone time) → dst gets its copied FCS.
3. later insertion → `handle_insert`'s absence guard sees FCS present → SKIPS
   re-derivation → the copied value survives.

A detached clone never reaching step 3 already has correct FCS from step 2 (I3).

---

## §7 Layering (mandate check)

- **Algorithm** (`apply_clone_form_state` — the field propagation) → `elidex-form`
  (engine-independent). ✓
- **dom-api** exposes only the src→dst *linkage* (pairs return, Option A; or a
  `ClonedFrom` marker, Option B) — never reads/writes `FormControlState` (I1). ✓
- **VM `node_methods_extras.rs::native_node_clone_node`** = marshalling only: clone →
  call `elidex_form::apply_clone_form_state` → return. Mirrors `apply_clone_creation_ce_semantics`.
  No algorithm in `vm/host/`. ✓
- **No B1-core touch** — no `MutationEvent` variant, no `elidex-ecs/src/dom` edit (I2). ✓

---

## §8 Test plan

Engine-independent (`elidex-form` unit tests over `EcsDom` + pairs):
- input: `value`='x' + `dirty_value` → clone copies both.
- input: `checked`=true / `indeterminate`=true → clone copies.
- textarea: raw `value` + `dirty_value` → clone copies.
- no-op: `<div>` pair / form-control src with **no** FCS → clone gets no spurious FCS.
- reconciler composition: after `apply_clone_form_state`, run `handle_insert` on the
  clone → the absence guard preserves the copied value (does not reset).

VM JS-level (elidex-js `tests_*`), the observable contract:
- `i.value='x'; i.cloneNode().value === 'x'` (detached — the I3 timing case).
- `c.checked=true; c.cloneNode().checked === true`; indeterminate likewise.
- `t.value='raw'; t.cloneNode().value === 'raw'` (textarea).
- deep clone `<form><input value=x></form>` → nested input copied.
- **shadow-inclusive (I5)**: deep-clone a host whose *clonable* shadow root contains
  a live-valued `<input>` → the shadow-encapsulated input's value is copied on the
  clone (not just light-tree controls).
- negative: `i.cloneNode()` then insert → value still 'x' (no reconciler reset).

---

## §9 Open review questions

- **O1 — seam B vs A** (§4/§4.1): RESOLVED to **Option B recommended, contingent on the
  impl-time clone-caller/leak audit**; Option A is the leak-free fallback for any
  dom-api/parser-internal clone path that can't reach an elidex-form consumer (I1).
- **O2 — copy ctor shape** (§5): `from_element`-then-overwrite (keeps non-step fields
  attribute-consistent) vs a dedicated `clone_form_state`. Recommend the former.

---

## §11 In-repo drift reconciliation (plan-review Axis 5)

Two stale in-repo references still describe this slot's work as deferred; the plan
supersedes both:

1. **`clone.rs:118-128` (dom-api, OWN) — reconcile IN this slice.** The
   `CustomElementState`-identity doc block *inside* `clone_node_with_shadow_honor` (the
   function this slice edits) still says *"no upgrade reaction is enqueued at clone time
   yet … slot `#11-clone-cloning-steps-event`"* — contradicting §1's verified premise
   (#331 R13/R14 DID the clone-time CE enqueue) AND its own sibling doc at
   `clone.rs:250-255` ("the …clone-time upgrade reactions (Codex PR331 R13+R14)"). This
   slice **drops the stale "not enqueued yet" / slot-deferral language** at
   `clone.rs:118-128` (in-lane, in the edited function) so the slot doesn't close with a
   comment still claiming its CE work is pending. **(plan-review Axis 5 IMP.)**
2. **`tree_clone.rs:28` (`crates/core/elidex-ecs/src/dom/`, **B1 core — out of lane**).**
   The clone-policy SoT enumeration row labels `FormControlState` a *"cloning-steps hook
   (deferred) … future `MutationEvent::Clone` consumer seam"* — which this slice
   supersedes **event-lessly** (I2 rejects `MutationEvent::Clone`). §7 + the lane
   boundary forbid editing B1 core. **Handling**: flag as a **landing-memo reconcile +
   B1-lane / PM coordination** item (retag the row to the chosen synchronous
   VM-marshalling seam + "implemented", and — if Option B — add the transient
   `ClonedFrom` component's "deliberate non-copy" note). NOT a defer slot; a
   cross-lane coordination note. **(plan-review Axis 5 MIN + Axis 2 cross-note.)**

Note the slot name `#11-clone-cloning-steps-event` embeds "event," but the delivered
design is deliberately event-less (I2); the name is retained (ledger identity) with this
plan as the record of the seam choice.

---

## §10 Defer slots (this slice)

1. **`#11-input-dirty-checkedness-flag`** (NEW) — model the dirty checkedness flag in
   `FormControlState` (field + user-toggle producer + `checked`-attr-change / reset
   consumers); the input cloning steps then also propagate it. **Re-eval trigger**: a
   form/WPT test observing checkedness dirtiness across `checked` mutation or reset.
   **Re-eval date**: 2026-09. **Registration**: registered in the memory defer ledger
   (`memory/project_open-defer-slots.md`) **at landing** via the PM landing reconcile
   (elidex-review Axis 5 ship-time-registration exception; the landing summary flags it).
   (§5.1.)
2. If **Option B** chosen: the `ClonedFrom` marker (dom-api component) is transient —
   attached in the cloner worklist, removed by `apply_clone_form_state` in the same
   synchronous pass — so it never persists into a subsequent shallow clone and needs
   **no** clone-policy copy row. No slot; documented in `clone.rs` at the attach site.
   The `tree_clone.rs` clone-policy enumeration is B1 core (out of lane) → the
   coordination note is folded into §11 (not a defer slot).

Per-PR ≤3: **1 new slot**. Within budget.
