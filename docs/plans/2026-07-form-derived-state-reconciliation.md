# Plan — form derived-state reconciliation (umbrella)

**Slots subsumed**: `#11-input-live-pseudo-state-elementstate-reconciliation` / `#11-fieldset-disabled-dynamic-insert` / `#11-input-dirty-checkedness-flag` (all carved by #466).
**Coordinated (NOT subsumed here)**: `#11-focusable-area-fieldset-inherited-disabled` = focus-program A3 hand-off — this umbrella exports `is_effectively_disabled`; **focus-A3 closes the slot**, not this umbrella (§4.1, §6).
**Lane**: DOM/form (L3) — `elidex-form` + `elidex-css/selector` + thin VM/shell touch.
**Status**: umbrella plan-memo, **`/elidex-plan-review` CONVERGED** over 3 rounds (R1: 13 findings → R2: 9, all real → R3: 0 CRIT/0 IMP, 1 doc-MIN). Design (pull-at-reader; `elidex-form-core` leaf; I1/I2/O1/E0116 all grep-verified) is design-approved. **Next = Slice 0a**, which its own review revealed is a real crate split (E0116) → takes its own `/elidex-plan-review`. Implementation deferred.
**Base**: `7bafcb9f` (#466 merge). Branch `domform-form-state-reconciliation`.

---

## §0.5 Spec citation table

| Cite | Anchor |
|---|---|
| HTML §4.16.3 Pseudo-classes (`:disabled`/`:enabled`/`:checked`/`:indeterminate`/`:valid`/`:invalid`/`:required`/`:optional`/`:read-only`/`:read-write`) | `#pseudo-classes` |
| HTML §4.10.19.5 form-element disabled concept (own `disabled` attr OR fieldset ancestry) | `#concept-fe-disabled` |
| HTML §4.15 "actually disabled" (`:disabled`/`:enabled` match hook; for form controls ⟺ the §4.10.19.5 concept) | `#concept-element-disabled` |
| HTML §4.10.15 The fieldset element (first-`<legend>` exemption) | `#the-fieldset-element` |
| HTML §4.10.5 input — checkedness / dirty checkedness flag / indeterminate | `#the-input-element` |
| HTML §4.10.21.2 Constraint validation (validity states) | `#constraint-validation` |

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
| **I2** | **Single reader** — `matching.rs` (via the private `form_element_state()`) is the **only** production reader of the ElementState form bits; no other consumer reads them — shell / layout / a11y / render / style / dom-api / VM read either `FormControlState` **directly** or `ElementState`'s `FOCUS`/`LINK`/`VISITED`, never the form bits (grep-verified 2026-07-15) | The cache has exactly one consumer ⇒ deleting it is a *local* change, not an engine-wide one. This is what makes I1's conclusion affordable. |
| **I3** | **Attribute-less live state** — `checked` (dirty), `indeterminate`, `value` (dirty), custom validity have **no content attribute** | The `set_attribute` chokepoint **cannot** cover them by construction. Any push design needs a *second* chokepoint; a pull design needs none. |
| **I4** | **Effective disabledness = own attribute OR fieldset ancestry** (HTML §4.10.19.5, with the first-`<legend>` exemption) — an **ancestry-dependent** predicate | Ancestry changes (insert / move / `fieldset.disabled` flip / `<legend>` reorder) invalidate it. A push design must subscribe to **tree mutations**; a pull design re-derives for free. |
| **I5** | **`:valid`/`:invalid` depend on the value** — validity is a function of the *current* value + constraints (`pattern` regex, `min`/`max`, `required`, custom message) | Every keystroke invalidates it. A push design must re-validate on every value mutation (incl. IME/paste); a pull design calls the existing pure `validate_control`. |
| **I6** | **Component-absent ≠ bits-clear** (asymmetric fallbacks, `matching.rs:246-318`) — for an `<input>` with **no** `ElementState`, `:read-only` is **`true`** today (falls to the non-form branch), while `:enabled`/`:optional` are `false` | Making the derivation always answer for form elements **flips five pseudo-classes** (`:read-only` t→f; `:enabled`/`:optional`/`:read-write`/`:valid` f→t) on such controls. Intentional, but all five must be called out + tested (§8). |
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
// elidex-form-core: the SINGLE source of every form pseudo-class answer.
// Pure derivations over (Option<FormControlState>, ancestry, Attributes). No cache, no invalidation.
// Each fn internally does `dom.get::<FormControlState>(entity)` and MUST define the component-ABSENT
// answer (a detached / synthetic / pre-init `<input>` has no FCS) — never unwrap.
pub fn is_effectively_disabled(entity: Entity, dom: &EcsDom) -> bool;  // own `disabled` attr OR fieldset ancestry (§4.10.19.5); reads Attributes+tree, NOT FCS.disabled
pub fn is_checked(entity: Entity, dom: &EcsDom) -> bool;               // input: FCS.checked (absent→false); option: `selected` attr
pub fn is_indeterminate(entity: Entity, dom: &EcsDom) -> bool;         // FCS.indeterminate (absent→false)
pub fn is_required(entity: Entity, dom: &EcsDom) -> bool;              // `required` attr (absent→false)
pub fn is_read_only(entity: Entity, dom: &EcsDom) -> bool;             // `readonly` attr OR disabled (absent→attribute-only; changes :read-only per I6)
pub fn validity(entity: Entity, dom: &EcsDom) -> ValidityState;        // → validate_control (already pure); absent→valid (no FCS)
```

`matching.rs::match_form_pseudo_class` calls these instead of reading bits. `form_element_state()` (the bit fetch) **goes away**. **FCS-absent contract (I3′)**: every predicate `Option`-guards its `FormControlState` fetch and returns the explicit absent-case answer above. This **replaces** today's asymmetric fallback (I6, `matching.rs:246-318`) with a coherent always-answer — for an FCS-absent form element it **flips five pseudo-classes** vs today (`:read-only` t→f; `:enabled`/`:optional`/`:read-write`/`:valid` f→t). §8 asserts all five flips at the pseudo-class level, not just `:read-only`.

### §3.2 What gets DELETED (this is the point)

- `init.rs::apply_element_state_flags` — the form-bit half (`DISABLED`/`CHECKED`/`REQUIRED`/`READ_ONLY`/`VALID`).
- `fieldset.rs::disable_descendants` + `propagate_fieldset_disabled` — the **entire push propagation** (pull makes it dead; deleted at the program tail, §6 Slice 5, after all effective-disabledness consumers migrate — **not** in Slice 1).
- `radio.rs:83-93`, `shell/form_input.rs:36-38` — activation bit writes.
- **`clone.rs:147-149`** — the #466 `:checked`/`:indeterminate` re-sync (this plan's own predecessor).
- The 7 form bits in `ElementState` (`components.rs:430-436`) — deleted in §6 Slice 1 once I2 is re-verified at impl (says none remains).

`ElementState` keeps `HOVER`/`FOCUS`/`ACTIVE`/`LINK`/`VISITED` (genuinely UI/pure-function state with real writers).

### §3.3 Why not the alternatives

| Option | Verdict |
|---|---|
| **Complete the push** (one canonical `derive_element_state()` called from every mutation site) | Rejected. I3 + I4: needs an FCS-mutation chokepoint **and** a tree-mutation subscription, i.e. *two new mechanisms*, to reach the correctness a pull gets for free. Keeps N maintainers — the failure mode that produced G1-G7. |
| **Recompute at restyle** (`set_link_state` pattern) | **Unsound** — I1. `matches()` bypasses the style walk. |
| **Keep the cache + version counter / dirty flag** | Premature. Adds an invalidation protocol (the thing pull removes) for an unmeasured perf problem (I7). If matching ever measures hot, re-introduce a cache *behind* the canonical derivation — with a single invalidation owner. |

---

## §4 FCS-layer holes (a prerequisite, not this layer)

Deleting the ElementState cache does **not** fix `FormControlState` itself being incompletely reconciled. These are **L1** (attribute → FCS) defects the surveys found, and Slice 0b closes them because §3's derivation *reads* FCS and would faithfully report the wrong answer otherwise:

- **Missing reconciler arms**: `checked` / `rows` / `cols` (`reconciler.rs:249-321`). Consequences: `form.reset()` restores stale checkedness; `textarea.rows` never reaches layout.
- **The `disabled` field is not the effective-disabledness SoT.** `reconciler.rs:266` is `fcs.disabled = new_value.is_some()` — a bare assignment that *already* fails to encode fieldset inheritance, while `init.rs:56/89` *does* fold `is_fieldset_disabled` into `fcs.disabled` at attach time. So `FCS.disabled` today is an **inconsistently-effective** value. The pull design does **not** "re-layer it to own-only" as a step (that would silently regress every raw reader mid-program — see §4.1). Instead: `is_effectively_disabled` (§3.1) reads the **`disabled` content attribute + tree ancestry directly** and never consults `FCS.disabled`; **all** effective-disabledness consumers migrate to it (§4.1 audit + §6 slicing); then `FCS.disabled` + the whole push-propagation (`disable_descendants` / `propagate_fieldset_disabled`) are deleted at the program tail (§6 Slice 5). `FCS.disabled`'s survival is an **audit output** (§4.1): it is redundant with the `disabled` content attribute, so the default disposition is **delete** (cache-is-bug applies here too), unless the audit finds a consumer that genuinely needs the control's *own* disabledness and cannot read the attribute.
- **IDL hand-mirrors** ("the second maintainer" the reconciler doc forbids): `html_input_value.rs:206-211` (`default_value`), `:278-280` (`default_checked`), `html_textarea_proto.rs:654-661`. Remove — the chokepoint already covers them once the arms exist.
- (Noted, **not** in scope) `<select>.value`/`.selectedIndex` write only the `selected` content attribute; FCS `options`/`selected_index`/`value` are never updated after init — a **split SoT** in the select surface. → defer slot §10 (`#11-select-value-split-sot`).

### §4.1 Effective-disabledness consumer audit (replaces the §9-O4 3-item sample)

`is_effectively_disabled` is a **new gate**; the sibling-site sweep (grep `fcs.disabled` / `is_fieldset_disabled` / `is_actually_disabled`) enumerates every reader of effective-disabledness and classifies each as *own* vs *effective*. Verified sites (2026-07-15):

| Site | Reads | Lane | Migration |
|---|---|---|---|
| `matching.rs` (`:disabled`/`:enabled`) | effective | **this (CSS)** | Slice 1 → the predicate |
| `validation/mod.rs:29,43` | effective (already `state.disabled \|\| is_fieldset_disabled`) | **this (form)** | Slice 2 → collapse the manual OR onto the predicate |
| `submit.rs:353` (`collect_control_entry`, submission candidacy) | **effective** — a fieldset-disabled control is *disabled*, so it is barred from constraint validation (§4.10.19.5) and **excluded when constructing the entry list** (§4.10.22.4) | **this (form)** | **Slice 1** → the predicate — **spec-critical**: bundled with `matching.rs`'s `:disabled` flip so no release separates greyed-from-submittable |
| `radio.rs:64` | effective | **this (form)** | Slice 2 |
| `init.rs:30` (autofocus), `init.rs:56/89` | effective | **this (form)** | Slice 2 → and `init.rs` stops writing effective-ness into `FCS.disabled` |
| (future) UA disabled rendering | — (no current `FormControlState` reader — disabled appearance routes via CSS `:disabled`→`matching.rs` today) | render | n/a — a future direct reader must call the predicate |
| `focus/predicate.rs:255` (`is_actually_disabled`) | **own attr only today** (no inheritance — slot `#11-focusable-area-fieldset-inherited-disabled`) | **focus-program A3** | via exported predicate — **coordinated hand-off** (§6), respecting focus A2b→A2c→A3 |
| shell `focus.rs:213`, `event_handlers.rs:132/374`, `ime.rs:20/46`, `content/mod.rs:438`, `form_input.rs:23` | effective (gating focus/event/IME/click) | **L4 shell** | via exported predicate — **coordinated hand-off**, L4-scheduled (durable slot `#11-shell-effective-disabled-predicate-adoption`, §10) |

(`init.rs:156` — the `state.disabled` read inside `apply_element_state_flags` — is **deleted** in §3.2, not migrated, so it is not a §4.1 row.)

**Transient consistency, precisely** (correcting an earlier over-claim): `FCS.disabled` is **approximately-effective** — reconciled at init + on own-attribute change (`init.rs:56/89`, `reconciler.rs:266`) but **NOT under dynamic fieldset ancestry** (G5: `fieldset.disabled=true` never updates descendants' `FCS.disabled`). So once Slice 1 makes `matching.rs` pull live, a *dynamically* fieldset-disabled control is `:disabled` (CSS greys it) while any consumer still on `FCS.disabled` sees it enabled. This is **per-consumer non-regressing** (each consumer stays exactly as correct as `main` — uniformly stale — until it migrates) but it does introduce a **transient cross-consumer incoherence** for the dynamic-ancestry case. Bounded two ways: (1) the **spec-critical** `submit.rs:353` (greyed-but-submittable = a §4.10.22.4 violation) is bundled into **Slice 1**, flipping with `:disabled`; (2) the residual focus/shell incoherence (greyed-but-*focusable* — a minor UX quirk, not a spec break) is a bounded transient closed by the cross-lane migrations → Slice 5. `FCS.disabled` + push-propagation delete at the tail (Slice 5) once the last consumer migrates.

---

## §5 Spec coverage map

| Spec | Branch | Derivation | Slice |
|---|---|---|---|
| §4.16.3 `:disabled`/`:enabled` | own attr OR fieldset ancestry (§4.10.19.5, first-`<legend>` exempt, nested walk) | `is_effectively_disabled` | 1 |
| §4.16.3 `:checked` | input checkedness / `<option>` `selected` | `is_checked` | 1 |
| §4.16.3 `:indeterminate` | FCS.indeterminate | `is_indeterminate` | 1 |
| §4.16.3 `:required`/`:optional` | `required` attr | `is_required` | 1 |
| §4.16.3 `:read-only`/`:read-write` | `readonly` attr OR disabled; non-form → contenteditable | `is_read_only` (+ I6 behavior change) | 1 |
| §4.16.3 `:valid`/`:invalid` | §4.10.21.2 constraint validation | `validity` → `validate_control` | **3** |
| §4.10.5 dirty checkedness flag | unmodeled today | new FCS field + producers/consumers | **4** |

---

## §6 Slicing (umbrella → per-PR, each `/elidex-plan-review`-gated per CLAUDE.md edge-dense rule)

| Slice | Scope | Closes |
|---|---|---|
| **0a — crate re-layer (`elidex-form-core`)** (hard prereq, §9 O1; **takes its own `/elidex-plan-review`** — a real crate split, not a trivial move) | new leaf crate (deps: `elidex-ecs` + `elidex-plugin` + `regex` + `url` + `tracing`) holding: the **`FormControlState` component *and its entire inherent `impl` block*** (Rust **E0116** forbids the impl in another crate → the value-model methods `settle_value`/`set_value`/`reset_value`/… + their `sanitize.rs`→`elidex-plugin::CssColor` dep move down too — this is *why* `elidex-plugin` is a dep, not "pure data"); the **existing pure derivations** (**`is_fieldset_disabled`** [called by `validation/mod.rs:43`], `validate_control`, `ValidityState`, `input`/`datetime`/`sanitize`/`util`, + the E0116-forced `value-mode` cluster). **0a is MOVE-ONLY** — the NEW §3.1 pull predicates (`is_effectively_disabled`/`is_checked`/`is_indeterminate`/`is_required`/`is_read_only`) are authored in **Slice 1** where `matching.rs` consumes them (no dead code in the 0a→1 window). Detail: `docs/plans/2026-07-form-core-crate-carve-slice-0a.md`. **`fieldset.rs` splits**: pull (`is_fieldset_disabled`) → core, push (`disable_descendants`/`propagate_fieldset_disabled`) stays in `elidex-form` until Slice 5. `elidex-form` depends on it; **0a does NOT touch `elidex-css`** (that dep edge is added in Slice 1 with the first `matching.rs` call). **Behavior-invariant but not trivial** — verify the exact moved set at impl (E0116 closure + repair intra-doc links: **grep every moved module's `///`/`//!` for `crate::`** — the carve-induced breaks are `validation/mod.rs:419`→`crate::radio` AND `sanitize.rs:42`→`crate::clipboard_paste`, both fail CI's `-D warnings` `doc` job once their module moves while `radio`/`clipboard` stay in `elidex-form`). **Without this, Slice 1 does not compile** (`css → form → dom-api → css` cycle). | unblocks the whole program |
| **0b — L1 reconciliation completeness** (prereq) | missing `checked`/`rows`/`cols` arms; delete the IDL hand-mirrors. (**No `FCS.disabled` re-layer here** — see Slice 5.) | reset-restores-stale-checkedness; textarea.rows layout bug |
| **1 — kill the ElementState cache, derive at the reader** (the keystone) | **author the §3.1 pull predicates in `elidex-form-core`** + add the `elidex-css`→`elidex-form-core` dep; `matching.rs` calls them; delete every ElementState form-bit writer (§3.2) incl. `clone.rs:147-149`; delete the 7 form-bit constants in `components.rs:430-436` (B1-core, contingent on re-verifying I2 at impl — provably dead post-migration). **Also bundle `submit.rs:353`** → `is_effectively_disabled` (spec-critical: its entry-list exclusion must flip *with* `:disabled`, else a dynamically-fieldset-disabled control is greyed-but-submittable, §4.1). `matching.rs` is the sole *cache* reader (I2); the added `submit.rs` site is one same-lane call. | `#11-input-live-pseudo-state-elementstate-reconciliation`, G1/G2/G6/G7 |
| **2 — effective-disabledness consumers (this lane)** | migrate `radio.rs:64`, `init.rs:30/56/89` to `is_effectively_disabled`, and collapse `validation/mod.rs`'s manual OR onto it (§4.1). (`submit.rs:353` = Slice 1; render has no current reader.) | `#11-fieldset-disabled-dynamic-insert`, G5; the in-lane share of the `disabled` split |
| **3 — `:valid`/`:invalid` live** | wire to `validate_control`; measure (I7) before deciding cached-vs-derived | G4 (the dead pseudo-classes) |
| **4 — dirty checkedness flag** | new FCS field + user-toggle producer + `checked`-attr-change / reset consumers | `#11-input-dirty-checkedness-flag` |
| **5 — tail: delete `FCS.disabled` + push-propagation** (gated on all-lane migration) | after this lane (Slice 2), focus-program A3, and L4-shell have all migrated to the exported predicate: delete `disable_descendants` / `propagate_fieldset_disabled` and (per §4.1 audit) `FCS.disabled` | the last of the `disabled` push machinery |

**Cross-lane hand-offs (not slices of this umbrella)**: `focus/predicate.rs:255` = **focus-program A3** (slot `#11-focusable-area-fieldset-inherited-disabled`, ≥3 slices out per MEMORY focus sequencing); shell overlays (`focus.rs`/`event_handlers.rs`/`ime.rs`/`content/mod.rs`/`form_input.rs`) = **L4** (currently **unowned** — no active L4 worktree touches these). This umbrella only *exports* `is_effectively_disabled` and migrates its own-lane consumers. **Slice 5's tail-delete is gated on both hand-offs, each with a durable slot** so it cannot orphan (per `reference_spawn-task-chips-not-durable`: cross-lane hand-offs live in a memory slot, not only this archivable memo): focus-A3's existing slot + the new `#11-shell-effective-disabled-predicate-adoption` (§10). Until both land, `FCS.disabled` stays live-and-approximately-effective (redundant-but-correct, not dead) — a bounded One-issue-one-way tax, not a correctness defect.

Slice 0a/0b precede Slice 1 (0a carves the crate; Slice 1 authors the predicates + wires `matching.rs`). Slice 2 depends on **Slice 1** (which authors the exported `is_effectively_disabled`). Slice 5 is gated on cross-lane completion. 3/4 are independent follow-ons. **Slice 1 irreducibility**: at its own mandated `/elidex-plan-review`, confirm the ElementState-cache flip is atomic against the real diff — do **not** sub-slice by pseudo-class (that recreates the "new seam + N legacy impls" strangler state One-issue-one-way forbids).

---

## §7 Layering check

- **Algorithm** (all derivations) → **`elidex-form-core`** (new engine-independent leaf crate, deps `elidex-ecs` + `elidex-plugin` + `regex` + `url` + `tracing` — §9 O1 RESOLVED: `elidex-css` cannot depend on `elidex-form`, it would cycle via `dom-api`). The `FormControlState` component **plus its inherent `impl` block** (E0116 — the value-model methods + `sanitize.rs`→`elidex-plugin::CssColor`, hence the `elidex-plugin` dep) and its pure-derivation closure (`is_effectively_disabled` / `is_fieldset_disabled` / `validate_control` / `ValidityState` / `input`+`datetime`+`util`) go **down**; the higher systems (reconciler / init / radio / submit / select / push-propagation) stay in `elidex-form`. So it is not "pure data down" — it is "**the component and everything Rust's coherence rules bind to it** (all pure/leaf-safe: every moved module imports only `elidex-ecs`/`elidex-plugin`)". ✓ ECS-native by construction.
- **`elidex-css/selector/matching.rs`** depends on `elidex-form-core` (a leaf) and calls the derivations. No cycle. ✓ (**Coordination**: L1 lane `css-shorthand-ordered-serialize` is concurrently editing `elidex-css` — a different module (`shorthand.rs`) — but if Slice 0a/1 touches `elidex-css/src/lib.rs` for the new dep/import, order with L1.)
- **No `vm/host/` algorithm** — the IDL setters *shrink* (hand-mirrors deleted); no new algorithm lands there. ✓
- **B1-core (`elidex-ecs/dom`) touch = the 7 dead `ElementState` form-bit constants** (`components.rs:430-436`), deleted in Slice 1 once I2 is re-verified (provably dead — no reader survives the migration). A mechanical dead-constant removal, not structural B1-core coordination; no concurrent lane touches these constants (verified 2026-07-15). "dead code は残さず削除" → delete, not defer (§9 O2).

---

## §8 Test plan

Engine-independent (`elidex-form-core`): each predicate against its SoT — incl. fieldset ancestry (nested, first-`<legend>` exemption, `<fieldset>` itself not disableable), the move-in/move-out cases that have **no** test today, and the **FCS-absent path for all six predicates** (I3′, §3.1) — a form element with no `FormControlState` must return each predicate's defined absent answer (never panic), not just `:read-only`.

VM JS-level (the contracts that are broken on `main` today — these are the regression proof):
- `i.checked = true; i.matches(':checked')` → true (currently **false**).
- `i.indeterminate = true; i.matches(':indeterminate')` → true (currently **false**).
- `i.disabled = true; i.matches(':disabled')` → true; `:enabled`/`:read-write` flip.
- `fs.disabled = true` (dynamic) → descendant `input.matches(':disabled')` → true (currently **false**).
- move a control **out** of `<fieldset disabled>` → `:enabled` again (currently **never re-enables**).
- `input.removeAttribute('disabled')` inside `<fieldset disabled>` → still `:disabled` (currently **clobbered to enabled**).
- `form.reset()` → `:checked` follows `defaultChecked` (Slice 1 asserts defaultChecked restoration **only**; full dirty-checkedness-flag interaction is deferred to Slice 4).
- Slice 2: `submit.rs` entry list **excludes** a fieldset-disabled control (§4.10.22.4 "constructing the entry list" skips disabled controls, which per §4.10.19.5 include fieldset-disabled — the spec-critical regression Slice 2 must guard).
- Slice 3: `i.required = true; i.value=''` → `:invalid` (currently **impossible to match**).
- I6: `<input>` with no FCS → **all five** flipped pseudo-classes asserted deliberately via `querySelector` on a detached/synthetic control: `:read-only` true→false, and `:enabled`/`:optional`/`:read-write`/`:valid` false→true.

---

## §9 Open questions (for `/elidex-plan-review`)

- **O1 — RESOLVED (2026-07-13, verified in Cargo.toml): `elidex-css` CANNOT depend on `elidex-form` — it would cycle.**
  `elidex-dom-api/Cargo.toml:17` depends on `elidex-css` (its `querySelector`/`matches` handlers call the selector engine), and `elidex-form/Cargo.toml:16` depends on `elidex-dom-api`. So `css → form → dom-api → css`. `elidex-script-session` likewise depends on `elidex-css`.
  **Consequence**: the selector engine sits *below* `elidex-form` in the graph and cannot even name `FormControlState`. The naive "`matching.rs` calls `elidex-form` predicates" shape (§3.1) **does not compile**.
  **Resolution — re-layer, ECS-natively: the component + its coherence-bound behavior down, higher systems up.** Carve a new leaf crate **`elidex-form-core`** (deps: `elidex-ecs` + `elidex-plugin` + `regex` + `url` + `tracing`; `regex`/`url`/`tracing` are `validate_control`'s transitive needs [`FCS.cached_pattern_regex`, `url::Url::parse`, `tracing`], and **`elidex-plugin` is `sanitize.rs`'s `CssColor`** — dragged down by the `FormControlState` inherent impl, below) holding:
  - the **`FormControlState` component AND its entire inherent `impl` block** — Rust **E0116** forbids an inherent impl in a crate other than the type's, so the value-model methods (`settle_value`/`set_value`/`reset_value`/…) + their `sanitize.rs` dep move down *with the type* (it is **NOT** "pure data"), and
  - the **pure-derivation closure**: `is_effectively_disabled` (`EcsDom` tree-nav + `Attributes` only), **`is_fieldset_disabled`** (called by `validation/mod.rs:43` — must move too), `is_checked` / `is_indeterminate` / `is_required` / `is_read_only`, and **`validate_control`** (`&FormControlState -> ValidityState`, `validation/mod.rs:104`) with `ValidityState` + the `input`/`datetime`/`util` subtree.
  Then `elidex-css` **and** `elidex-form` both depend on `elidex-form-core`; no cycle (verified: every moved module imports only `elidex-ecs`/`elidex-plugin`). The higher *systems* (reconciler, init, radio, submit, select, validation bindings, push-propagation) stay in `elidex-form`; **`fieldset.rs` splits** (pull down, push up).
  This is the ECS-native split by construction — **the component + its coherence-bound value model + pure derivations low, higher systems high** — and it is what makes the pull design reachable. It becomes **Slice 0a** (§6). Behavior-invariant but **not a trivial move** (a crate split under E0116 + intra-doc-link repair — grep moved modules for `crate::` doc links; known breaks: `validation/mod.rs:419`→`crate::radio`, `sanitize.rs:42`→`crate::clipboard_paste`); Slice 0a takes its own `/elidex-plan-review`.
- **O2 — RESOLVED: delete the 7 form-bit constants in Slice 1**, not defer. Post-migration they are provably dead (I2: `matching.rs` is the sole reader). Removing 7 unused enum constants from `components.rs:430-436` is a mechanical dead-constant removal, not structural B1-core coordination, and no concurrent lane touches them (verified 2026-07-15). "dead code は残さず削除" governs; contingent only on re-verifying I2 at impl.
- **O3 — I7 perf**: is a per-match `validate_control` (regex!) acceptable for `:valid`/`:invalid`, or does Slice 3 need a derived-and-cached validity with a single invalidation owner? **Measure, don't guess** (Slice 3 gates on the measurement).
- **O4 — RESOLVED into §4.1**: the `FCS.disabled` consumer set is no longer a 3-item sample. §4.1 is the exhaustive audit (grep-verified) classifying every effective-disabledness reader own-vs-effective and assigning each to a lane; the `submit.rs:353` submission-candidacy reader (spec-critical) was the sample's key omission. The re-layer is replaced by "read Attributes+ancestry directly + migrate all consumers + tail-delete `FCS.disabled`" (§4, §6 Slice 5).

---

## §10 Defer slots (new)

1. **`#11-select-value-split-sot`** — `<select>.value`/`.selectedIndex` write only the `selected` content attribute; FCS `options`/`selected_index`/`value` go stale after init (§4). Separate surface (select option-list model), not this derived-state layer. **Adjacent open slot**: `#11-select-setters-attr-snapshot-symmetric` (select setters don't snapshot the `(option,"selected")` cache) shares the select-setter surface — distinct facet (cache-snapshot symmetry vs FCS split-SoT), but the select surface pass should evaluate both together. **Trigger**: a select-state WPT/site, or the select surface pass. **Date**: 2026-10.
2. **`#11-shell-effective-disabled-predicate-adoption`** — the L4-shell overlays (`focus.rs:213`, `event_handlers.rs:132/374`, `ime.rs:20/46`, `content/mod.rs:438`, `form_input.rs:23`) migrate their effective-disabledness gating (focus / event / IME / click) from raw `FCS.disabled` to the exported `is_effectively_disabled` (§4.1). **Why deferred**: cross-lane — L4 owns shell; no active L4 worktree touches these overlays today (so it can't ride this umbrella). **Durable home** for the hand-off (per `reference_spawn-task-chips-not-durable`) so Slice 5's tail-delete can't orphan. **Trigger**: an L4 shell-overlay pass, or Slice 5 readiness. **Date**: 2026-11.

Program total = 2 new slots (this + `#11-select-value-split-sot`); focus-A3's `#11-focusable-area-fieldset-inherited-disabled` is pre-existing. Per-PR ≤3 across the program; each slice re-audits at its own landing.

## §11 Supersedes

This umbrella **supersedes the per-symptom framing** of the three #466 carves — they are one root (`#1.3`). `#11-fieldset-disabled-dynamic-insert` in particular **dissolves** rather than being "implemented": with pull-derivation there is no propagation to schedule.
