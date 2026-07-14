# Plan — form derived-state reconciliation (umbrella)

**Slots subsumed**: `#11-input-live-pseudo-state-elementstate-reconciliation` / `#11-fieldset-disabled-dynamic-insert` / `#11-input-dirty-checkedness-flag` (all carved by #466) + `#11-focusable-area-fieldset-inherited-disabled`.
**Lane**: DOM/form (L3) — `elidex-form` + `elidex-css/selector` + thin VM/shell touch.
**Status**: umbrella plan-memo, pre-`/elidex-plan-review`. Implementation deferred to the following session.
**Base**: `7bafcb9f` (#466 merge). Branch `domform-form-state-reconciliation`.

---

## §0.5 Spec citation table

| Cite | Anchor |
|---|---|
| HTML §4.16.3 Pseudo-classes (`:disabled`/`:enabled`/`:checked`/`:indeterminate`/`:valid`/`:invalid`/`:required`/`:optional`/`:read-only`/`:read-write`) | `#pseudo-classes` |
| HTML §4.10.19.2 "actually disabled" / disabled-by-fieldset | `#concept-fe-disabled` |
| HTML §4.10.18.5 The fieldset element (first-`<legend>` exemption) | `#the-fieldset-element` |
| HTML §4.10.5 input — checkedness / dirty checkedness flag / indeterminate | `#the-input-element` |
| HTML §4.10.22.3 Constraint validation (validity states) | `#constraint-validation` |

---

## §1 Goal — close the derived-state hole by construction

`ElementState`'s **form bits** (`DISABLED` / `CHECKED` / `REQUIRED` / `VALID` / `INVALID` / `READ_ONLY` / `INDETERMINATE`) are a **cache of derived state** that is maintained by *incremental ad-hoc writes at N sites* — and almost none of the sites do it. The result is a systemic staleness class, several live user-visible bugs, and a source-of-truth that has split three ways.

This umbrella **deletes the cache** and derives the form pseudo-class answers from their source of truth at the single point that reads them.

### §1.1 The codebase already states the ideal — and stops one layer short

`elidex-form/src/reconciler.rs:10-17` (verbatim):

> *ECS-native first: `FormControlState` is a derived component (source-of-truth = `Attributes` content attribute). Per ECS first principles, derived-state reconciliation belongs to a system subscribed to mutations of the source state, NOT a side effect of every IDL setter. Single reconciler path covers IDL setter / `setAttribute` / parser / `innerHTML` / future Custom Element attribute callback uniformly via the `EcsDom::set_attribute` / `EcsDom::remove_attribute` chokepoint.*

The chokepoint reaches **`FormControlState`** and stops. `ElementState` — the *only* thing CSS reads — is written by exactly one function, `apply_element_state_flags` (`init.rs:150-171`), called from exactly one place, `finalize_control` (`init.rs:146`), i.e. **at FCS-creation time only**. `reconciler.rs` contains **zero** `ElementState` references (934 lines).

### §1.2 Evidence (3 delegated surveys, all file:line-verified)

**Gap matrix — every non-init mutation path leaves the bits stale:**

| # | Path | file:line | FCS | bit |
|---|---|---|---|---|
| G1 | `setAttribute("disabled"/"required"/"readonly")` + reflected IDL | `reconciler.rs:266-268` | ✅ | ❌ |
| G2 | IDL `input.checked =` / `.indeterminate =` (direct FCS write, bypasses the attribute chokepoint by construction — these are *attribute-less live state*) | `html_input_value.rs:244,313` | ✅ | ❌ |
| G3 | `setAttribute("checked")` — **no `checked` arm exists** in the reconciler | `reconciler.rs:249-321` | **❌** | ❌ |
| G4 | constraint validation | `validation/**` | ✅ | ❌ (`VALID` insert-once at `init.rs:169`; **`INVALID` has no production writer at all**) |
| G5 | `fieldset.disabled = true` (dynamic) | `html_fieldset_proto.rs` → `reconciler.rs:266` | ❌ (descendants untouched) | ❌ |
| G6 | `apply_element_state_flags` is **insert-only** (never `remove`) | `init.rs:156-169` | — | cannot clear a stale bit even if re-run |
| G7 | `form.reset()` / `type` change | `submit.rs:478`, `reconciler.rs:152` | ✅ | ❌ |

Only **four** sites sync a bit today: init (`init.rs`), radio activation (`radio.rs:83-93`), shell checkbox click (`form_input.rs:36-38`), and the #466 clone overlay (`clone.rs:147-149`).

**Live user-visible consequences:**
- `input.checked = true` → `input.matches(':checked')` is **false**. Same for `.indeterminate`. (The #466 finding.)
- `input.disabled = true` → `:disabled` / `:enabled` / `:read-only` / `:read-write` all stale (**one bit, four pseudo-classes**).
- **`:valid` matches every form control forever; `:invalid` can never match.**
- `setAttribute("checked")` updates neither `checked` nor `default_checked` → **`form.reset()` restores a stale value**.
- `textarea.rows = 10` never reaches `fcs.rows` (no arm) → **`sizing.rs:50` intrinsic size uses the old row count** (a layout bug).
- `removeAttribute("disabled")` on a control inside `<fieldset disabled>` sets `fcs.disabled = false` (`reconciler.rs:266` is a bare assignment) → **clobbers the inherited disabledness**.
- Moving a control *out* of a `<fieldset disabled>` never re-enables it (`disable_descendants` only ever sets `true`; no inverse exists).

### §1.3 The asymmetry that names the root cause

Everything that is **broken** is a *push/cache* (bits patched at mutation sites).
Everything that is **correct** is a *pull/derive-on-read*:

| Correct today | Mechanism |
|---|---|
| Constraint validation's disabledness | `state.disabled \|\| is_fieldset_disabled(entity, dom)` — **live pull** every call (`validation/mod.rs:31-43`) |
| `:checked` on `<option>` | reads the `selected` **content attribute** on the fly (`matching.rs:279-281`) |
| `LINK` / `VISITED` | recomputed on **every restyle** (`elidex-style/walk.rs:707-719`) |
| `ValidityState` (JS) | `validate_control(&FormControlState)` — an **already-pure derivation** (`validation/mod.rs:104`) |

**The cache *is* the bug.** Not "the cache is missing N updates".

---

## §2 Coupled invariants (edge-dense enumeration)

Edge-dense: SoT-layering × derivation-timing × reader-set × spec-fidelity × perf. Each pairwise intersection is where a prose-only plan leaks.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Synchronous JS read** — `el.matches(':checked')` / `querySelector` / `closest` **bypass the style walk entirely** (verified: `element_proto.rs:355` → `invoke_dom_api` → `Matches::invoke` (`child_node/selectors.rs:38`) → `selector/mod.rs:43` → `matching.rs`; **no** call to `resolve_styles*`) | A "recompute every restyle, like `LINK`" design is **unsound** — a script mutates and re-reads in the same turn. The derivation must sit at the **reader**, not in the style walk. |
| **I2** | **Single reader** — `matching.rs` is the **only** production reader of the form bits (shell / layout / a11y / render / style / dom-api / VM all read `FormControlState` **directly**) | The cache has exactly one consumer ⇒ deleting it is a *local* change, not an engine-wide one. This is what makes I1's conclusion affordable. |
| **I3** | **Attribute-less live state** — `checked` (dirty), `indeterminate`, `value` (dirty), custom validity have **no content attribute** | The `set_attribute` chokepoint **cannot** cover them by construction. Any push design needs a *second* chokepoint; a pull design needs none. |
| **I4** | **Effective disabledness = own attribute OR fieldset ancestry** (HTML §4.10.19.2, with the first-`<legend>` exemption) — an **ancestry-dependent** predicate | Ancestry changes (insert / move / `fieldset.disabled` flip / `<legend>` reorder) invalidate it. A push design must subscribe to **tree mutations**; a pull design re-derives for free. |
| **I5** | **`:valid`/`:invalid` depend on the value** — validity is a function of the *current* value + constraints (`pattern` regex, `min`/`max`, `required`, custom message) | Every keystroke invalidates it. A push design must re-validate on every value mutation (incl. IME/paste); a pull design calls the existing pure `validate_control`. |
| **I6** | **Component-absent ≠ bits-clear** (asymmetric fallbacks, `matching.rs:246-318`) — for an `<input>` with **no** `ElementState`, `:read-only` is **`true`** today (falls to the non-form branch), while `:enabled`/`:optional` are `false` | Making the derivation always answer for form elements **changes `:read-only`** on such controls. Intentional fix, but must be called out + tested. |
| **I7** | **Matching is a hot path** (every element × every selector) | The derivation must stay cheap: gated on `is_form_element` (already), bounded ancestor walk (already `MAX_ANCESTOR_DEPTH`-capped), and `:valid`/`:invalid` must not run a regex per match without measurement. |

**Pairwise intersections (the load-bearing ones):**
- **I1 × I2** — the sync-read constraint *rules out* the style-walk placement, and the single-reader fact *makes the reader placement free*. Together they select the design uniquely: **derive inside `form_element_state()`**.
- **I3 × I4** — the two things a push design cannot reach (attribute-less live state; ancestry) are exactly the two things a pull design gets for free. The push design would need *two more* subscription mechanisms (an FCS-mutation chokepoint + a tree-mutation subscription) to reach parity with doing nothing.
- **I5 × I7** — validity is the one derivation with real cost. It is the reason `:valid`/`:invalid` is its **own slice** (§6, Slice 3), not folded into Slice 1.
- **I4 × I6** — unifying the disabledness predicate also fixes focusability (`focus/predicate.rs:255-257` reads the **content attribute** directly and does not implement fieldset inheritance at all — slot `#11-focusable-area-fieldset-inherited-disabled`), so the canonical predicate must be exported from `elidex-form` for **three** consumers (CSS, validation, focus).

---

## §3 The design — delete the cache, derive at the reader

### §3.1 Canonical derivation (engine-independent, `elidex-form`)

```rust
// elidex-form: the SINGLE source of every form pseudo-class answer.
// Pure derivations over (FormControlState, ancestry, Attributes). No cache, no invalidation.
pub fn is_effectively_disabled(entity: Entity, dom: &EcsDom) -> bool;  // own attr OR fieldset ancestry (§4.10.19.2)
pub fn is_checked(entity: Entity, dom: &EcsDom) -> bool;               // input: FCS.checked; option: `selected` attr
pub fn is_indeterminate(entity: Entity, dom: &EcsDom) -> bool;         // FCS.indeterminate
pub fn is_required(entity: Entity, dom: &EcsDom) -> bool;
pub fn is_read_only(entity: Entity, dom: &EcsDom) -> bool;
pub fn validity(entity: Entity, dom: &EcsDom) -> ValidityState;        // → validate_control (already pure)
```

`matching.rs::match_form_pseudo_class` calls these instead of reading bits. `form_element_state()` (the bit fetch) **goes away**.

### §3.2 What gets DELETED (this is the point)

- `init.rs::apply_element_state_flags` — the form-bit half (`DISABLED`/`CHECKED`/`REQUIRED`/`READ_ONLY`/`VALID`).
- `fieldset.rs::disable_descendants` + `propagate_fieldset_disabled` — the **entire push propagation** (pull makes it dead).
- `radio.rs:83-93`, `shell/form_input.rs:36-38` — activation bit writes.
- **`clone.rs:147-149`** — the #466 `:checked`/`:indeterminate` re-sync (this plan's own predecessor).
- The 7 form bits in `ElementState` (`components.rs:430-436`) — **only** if no consumer remains (I2 says none does; re-verify at impl).

`ElementState` keeps `HOVER`/`FOCUS`/`ACTIVE`/`LINK`/`VISITED` (genuinely UI/pure-function state with real writers).

### §3.3 Why not the alternatives

| Option | Verdict |
|---|---|
| **Complete the push** (one canonical `derive_element_state()` called from every mutation site) | Rejected. I3 + I4: needs an FCS-mutation chokepoint **and** a tree-mutation subscription, i.e. *two new mechanisms*, to reach the correctness a pull gets for free. Keeps N maintainers — the failure mode that produced G1-G7. |
| **Recompute at restyle** (`set_link_state` pattern) | **Unsound** — I1. `matches()` bypasses the style walk. |
| **Keep the cache + version counter / dirty flag** | Premature. Adds an invalidation protocol (the thing pull removes) for an unmeasured perf problem (I7). If matching ever measures hot, re-introduce a cache *behind* the canonical derivation — with a single invalidation owner. |

---

## §4 FCS-layer holes (a prerequisite, not this layer)

Deleting the ElementState cache does **not** fix `FormControlState` itself being incompletely reconciled. These are **L1** (attribute → FCS) defects the surveys found, and Slice 0 closes them because §3's derivation *reads* FCS and would faithfully report the wrong answer otherwise:

- **Missing reconciler arms**: `checked` / `rows` / `cols` (`reconciler.rs:249-321`). Consequences: `form.reset()` restores stale checkedness; `textarea.rows` never reaches layout.
- **`disabled` clobber**: `reconciler.rs:266` is `fcs.disabled = new_value.is_some()` — a bare assignment that erases fieldset inheritance. **Fix by re-layering**: `FCS.disabled` becomes a faithful reflection of the control's **own** attribute; the *effective* (inheritance-aware) answer moves to `is_effectively_disabled` (§3.1). This is what makes I4 dissolve.
- **IDL hand-mirrors** ("the second maintainer" the reconciler doc forbids): `html_input_value.rs:206-211` (`default_value`), `:278-280` (`default_checked`), `html_textarea_proto.rs:654-661`. Remove — the chokepoint already covers them once the arms exist.
- (Noted, **not** in scope) `<select>.value`/`.selectedIndex` write only the `selected` content attribute; FCS `options`/`selected_index`/`value` are never updated after init — a **split SoT** in the select surface. → defer slot, §9.

---

## §5 Spec coverage map

| Spec | Branch | Derivation | Slice |
|---|---|---|---|
| §4.16.3 `:disabled`/`:enabled` | own attr OR fieldset ancestry (§4.10.19.2, first-`<legend>` exempt, nested walk) | `is_effectively_disabled` | 1 |
| §4.16.3 `:checked` | input checkedness / `<option>` `selected` | `is_checked` | 1 |
| §4.16.3 `:indeterminate` | FCS.indeterminate | `is_indeterminate` | 1 |
| §4.16.3 `:required`/`:optional` | `required` attr | `is_required` | 1 |
| §4.16.3 `:read-only`/`:read-write` | `readonly` attr OR disabled; non-form → contenteditable | `is_read_only` (+ I6 behavior change) | 1 |
| §4.16.3 `:valid`/`:invalid` | §4.10.22.3 constraint validation | `validity` → `validate_control` | **3** |
| §4.10.5 dirty checkedness flag | unmodeled today | new FCS field + producers/consumers | **4** |

---

## §6 Slicing (umbrella → per-PR, each `/elidex-plan-review`-gated per CLAUDE.md edge-dense rule)

| Slice | Scope | Closes |
|---|---|---|
| **0a — crate re-layer (`elidex-form-core`)** (hard prereq, §9 O1) | new leaf crate (deps: `elidex-ecs` + `elidex-plugin`): move the **`FormControlState` component** + the **pure derivations** (incl. `validate_control`) down; `elidex-css` and `elidex-form` both depend on it. Mechanical move, no behavior change. **Without this, Slice 1 does not compile** (`css → form → dom-api → css` cycle). | unblocks the whole program |
| **0b — L1 reconciliation completeness** (prereq) | missing `checked`/`rows`/`cols` arms; `FCS.disabled` = own-attribute-only re-layering; delete the IDL hand-mirrors | reset-restores-stale-checkedness; textarea.rows layout bug |
| **1 — kill the cache, derive at the reader** (the keystone) | `elidex-form` canonical predicates (§3.1); `matching.rs` calls them; delete every form-bit writer (§3.2) incl. `clone.rs:147-149`; delete `disable_descendants`/`propagate_fieldset_disabled` | `#11-input-live-pseudo-state-elementstate-reconciliation`, `#11-fieldset-disabled-dynamic-insert`, G1/G2/G5/G6/G7 |
| **2 — unify the third consumer** | `focus/predicate.rs:255-257` calls `is_effectively_disabled` (today: raw attribute read, no inheritance) | `#11-focusable-area-fieldset-inherited-disabled`; the 3-way `disabled` split |
| **3 — `:valid`/`:invalid` live** | wire to `validate_control`; measure (I7) before deciding cached-vs-derived | G4 (the dead pseudo-classes) |
| **4 — dirty checkedness flag** | new FCS field + user-toggle producer + `checked`-attr-change / reset consumers | `#11-input-dirty-checkedness-flag` |

Slice 1 is the keystone; 0 must precede it (its derivation reads FCS). 2/3/4 are independent follow-ons.

---

## §7 Layering check

- **Algorithm** (all derivations) → **`elidex-form-core`** (new engine-independent leaf crate — §9 O1 RESOLVED: `elidex-css` cannot depend on `elidex-form`, it would cycle via `dom-api`). Data (the `FormControlState` component) and its pure derivations go **down**; the systems (reconciler / init / radio / submit / select) stay in `elidex-form`. ✓ ECS-native by construction.
- **`elidex-css/selector/matching.rs`** depends on `elidex-form-core` (a leaf) and calls the derivations. No cycle. ✓
- **No `vm/host/` algorithm** — the IDL setters *shrink* (hand-mirrors deleted); no new algorithm lands there. ✓
- **No B1-core (`elidex-ecs/dom`) touch** except possibly removing 7 `ElementState` constants (`components.rs:430-436`) — that IS B1 core → coordinate or defer the constant removal (§9 O2).

---

## §8 Test plan

Engine-independent (`elidex-form`): each predicate against its SoT — incl. fieldset ancestry (nested, first-`<legend>` exemption, `<fieldset>` itself not disableable), and the move-in/move-out cases that have **no** test today.

VM JS-level (the contracts that are broken on `main` today — these are the regression proof):
- `i.checked = true; i.matches(':checked')` → true (currently **false**).
- `i.indeterminate = true; i.matches(':indeterminate')` → true (currently **false**).
- `i.disabled = true; i.matches(':disabled')` → true; `:enabled`/`:read-write` flip.
- `fs.disabled = true` (dynamic) → descendant `input.matches(':disabled')` → true (currently **false**).
- move a control **out** of `<fieldset disabled>` → `:enabled` again (currently **never re-enables**).
- `input.removeAttribute('disabled')` inside `<fieldset disabled>` → still `:disabled` (currently **clobbered to enabled**).
- `form.reset()` → `:checked` follows `defaultChecked`.
- Slice 3: `i.required = true; i.value=''` → `:invalid` (currently **impossible to match**).
- I6: `<input>` with no FCS → `:read-only` behavior change is asserted deliberately.

---

## §9 Open questions (for `/elidex-plan-review`)

- **O1 — RESOLVED (2026-07-13, verified in Cargo.toml): `elidex-css` CANNOT depend on `elidex-form` — it would cycle.**
  `elidex-dom-api/Cargo.toml:17` depends on `elidex-css` (its `querySelector`/`matches` handlers call the selector engine), and `elidex-form/Cargo.toml:16` depends on `elidex-dom-api`. So `css → form → dom-api → css`. `elidex-script-session` likewise depends on `elidex-css`.
  **Consequence**: the selector engine sits *below* `elidex-form` in the graph and cannot even name `FormControlState`. The naive "`matching.rs` calls `elidex-form` predicates" shape (§3.1) **does not compile**.
  **Resolution — re-layer, ECS-natively: data down, systems up.** Carve a new leaf crate **`elidex-form-core`** (deps: `elidex-ecs` + `elidex-plugin` only) holding:
  - the **`FormControlState` component** (pure data), and
  - the **pure derivations** over it: `is_effectively_disabled` (needs only `EcsDom` tree-nav + `Attributes`), `is_checked` / `is_indeterminate` / `is_required` / `is_read_only`, and **`validate_control`** (already a pure `&FormControlState -> ValidityState` fn, `validation/mod.rs:104`).
  Then `elidex-css` **and** `elidex-form` both depend on `elidex-form-core`; no cycle. The *systems* (reconciler, init, radio, submit, select, validation bindings) stay in `elidex-form`.
  This is the ECS-native split by construction — **components are data (low), systems are behavior (high)** — and it is what makes the pull design reachable. It becomes **Slice 0a** (§6).
- **O2** — removing the 7 form bits from `ElementState` (`components.rs`, B1 core) vs leaving them unused. Lane boundary says don't touch B1 core → likely leave the constants, delete only the writers/readers, and carve a cleanup note.
- **O3** — I7 perf: is a per-match `validate_control` (regex!) acceptable for `:valid`/`:invalid`, or does Slice 3 need a derived-and-cached validity with a single invalidation owner? **Measure, don't guess.**
- **O4** — Slice 0's `FCS.disabled` re-layering (own-attribute-only) changes what `radio.rs:64` / `init.rs:30` (autofocus) / render read. Audit those consumers: do they want *own* or *effective* disabledness? (Almost certainly effective → they must call the predicate too.)

---

## §10 Defer slots (new)

1. **`#11-select-value-split-sot`** — `<select>.value`/`.selectedIndex` write only the `selected` content attribute; FCS `options`/`selected_index`/`value` go stale after init (§4). Separate surface (select option-list model), not this derived-state layer. **Trigger**: a select-state WPT/site, or the select surface pass. **Date**: 2026-10.

Per-PR ≤3 across the program; each slice re-audits at its own landing.

## §11 Supersedes

This umbrella **supersedes the per-symptom framing** of the three #466 carves — they are one root (`#1.3`). `#11-fieldset-disabled-dynamic-insert` in particular **dissolves** rather than being "implemented": with pull-derivation there is no propagation to schedule.
