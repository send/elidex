# Plan — delete the `ElementState` form-bit cache, derive at the reader (form derived-state Slice 1, the keystone)

## §0 Status

**Parent**: `docs/plans/2026-07-form-derived-state-reconciliation.md` §6 slicing-table row "1" (umbrella, `/elidex-plan-review`-converged R1-R3).
**Predecessors landed**: Slice 0a `elidex-form-core` carve (#472 `3e797897`, plan `docs/plans/2026-07-form-core-crate-carve-slice-0a.md`); Slice 0b L1 reconciliation completeness (#479 `2d0459c3`, plan `docs/plans/2026-07-form-l1-reconciliation-completeness-slice-0b.md`).
**HARD PREREQ (not yet landed)**: **PR-A** — `docs/plans/2026-07-form-submittable-category-repair.md`. Slice 1 wires `:valid`/`:invalid` to `is_constraint_validation_candidate`, whose basis (`FormControlKind`'s `is_submittable`) is not the §4.10.2 category today; on the uncorrected basis this slice would regress `<button type=submit>:valid` true→false, which its own D2 forbids. Same prereq shape as 0a (crate reachability) and 0b (component faithfulness): PR-A makes the *predicate* correct before Slice 1 promotes it to a CSS-visible answer.
**Lane**: DOM/form (L3). **Branch**: `domform-slice1`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-slice1`.
**Base**: `f7d9b5ce` (current `origin/main`).
**Status**: plan-memo draft — **`/elidex-plan-review` REQUIRED before implementation** (CLAUDE.md edge-dense rule; this is the umbrella's mandated per-slice gate).
**Nature**: **not** behavior-invariant. Slice 1 deletes a derived-state cache and replaces it with spec-correct pull derivations, so it ships a deliberate, enumerated set of behavior deltas (§6). Every delta is a spec-fidelity fix or an explicitly-scoped boundary.

---

## §0.5 Spec citation table

Every § number ↔ title pair below was looked up with `.claude/tools/webref` on **2026-07-26** (`heading --exact` / `dfn` / `body`); none is quoted from memory. Where an existing in-tree comment cites a different number, the drift is recorded in §6-C.

| Cite | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| Pseudo-classes (the operative source for all 10) | HTML §4.16.3 | Pseudo-classes | `#pseudo-classes` | `heading --exact html 4.16.3` / `body html pseudo-classes` |
| "actually disabled" (8 branches) | HTML §4.15 | Disabled elements | `#disabled-elements` / dfn `#concept-element-disabled` | `heading --exact html 4.15`; `dfn html 'actually disabled'` |
| a form control "is disabled" (own attr OR fieldset ancestry, first-`<legend>` exempt) | HTML §4.10.19.5 | Enabling and disabling form controls: the disabled attribute | `#enabling-and-disabling-form-controls:-the-disabled-attribute` | `heading --exact html 4.10.19.5` |
| "disabled fieldset" (own attr OR fieldset ancestry) | HTML §4.10.15 | The fieldset element | `#concept-fieldset-disabled` | `dfn html 'disabled fieldset'`; `body html concept-fieldset-disabled` |
| "candidate for constraint validation" / "satisfies its constraints" | HTML §4.10.21.1 | Definitions | `#candidate-for-constraint-validation` / `#concept-fv-valid` | `dfn html 'candidate for constraint validation'`; `dfn html 'satisfies its constraints'` |
| submittable elements (the candidate basis) | HTML §4.10.2 | Categories | `#category-submit` | `dfn html 'submittable element'` |
| a non-submit `<button>` is barred from constraint validation | HTML §4.10.6 | The button element | `#the-button-element` | `body html the-button-element` → "Constraint validation: If the element is not a submit button, the element is barred from constraint validation." |
| `<input type=submit>` — **NOT barred** (only a `formnovalidate` note); cited by §6-A row 10a, branch enumeration owned by **PR-A** | HTML §4.10.5.1.18 | Submit Button state (type=submit) | `#submit-button-state-(type=submit)` | `body html 'submit-button-state-(type=submit)' \| grep -i 'barred from constraint'` → **no match** |
| `<input type=reset>` is barred — **PR-A-owned**; here for the §13 record only | HTML §4.10.5.1.20 | Reset Button state (type=reset) | `#reset-button-state-(type=reset)` | `body html 'reset-button-state-(type=reset)'` → "Constraint validation: The element is barred from constraint validation." |
| `<input type=button>` is barred — **PR-A-owned**; here for the §13 record only | HTML §4.10.5.1.21 | Button state (type=button) | `#button-state-(type=button)` | `body html 'button-state-(type=button)'` → same statement |
| `<input type=hidden>` is barred (the existing `validation/mod.rs:32` branch) | HTML §4.10.5.1.1 | Hidden state (type=hidden) | `#hidden-state-(type=hidden)` | `body html 'hidden-state-(type=hidden)'` → "…it is barred from constraint validation." |
| `<output>` (not a submittable element ⇒ not a candidate) | HTML §4.10.12 | The output element | `#the-output-element` | `heading --exact html 4.10.12` |
| entry-list construction: step 5.1 "field is a button but it is not submitter" | HTML §4.10.22.4 | Constructing the entry list | `#constructing-form-data-set` | `body html constructing-form-data-set` (step 5.1 quoted verbatim in §5-M1) |
| "mutable" form control | HTML §4.10.18.2 | Mutability | `#mutability` | `heading --exact html 4.10.18.2` |
| `readonly` attribute + its apply-list | HTML §4.10.5.3.3 | The readonly attribute | `#the-readonly-attribute` | `heading --exact html 4.10.5.3.3` |
| `required` attribute + its apply-list ("the element is required") | HTML §4.10.5.3.4 | The required attribute | `#attr-input-required` / `#concept-input-required` | `heading --exact html 4.10.5.3.4`; `body html attr-input-required` |
| `<option>` selectedness (live, not the content attribute) | HTML §4.10.10 | The option element | `#concept-option-selectedness` | `dfn html 'selectedness'` |
| radio button group | HTML §4.10.5.1.16 | Radio Button state (type=radio) | `#radio-button-group` | `dfn html 'radio button group'` |
| editing host | HTML §6.8.4 | Editing APIs | `#editing-host` | `dfn html 'editing host'` (the *dfn* is `editing host`; the *section title* is "Editing APIs" — do not conflate) |
| `:enabled` / `:disabled` delegation | CSS Selectors L4 §12.1.1 | The :enabled and :disabled Pseudo-classes | `#enableddisabled` | `heading --exact selectors-4 12.1.1` |
| `:read-only` / `:read-write` delegation | CSS Selectors L4 §12.1.2 | The Mutability Pseudo-classes: :read-only and :read-write | `#rw-pseudos` | `heading --exact selectors-4 12.1.2` |
| `:checked` / `:unchecked` / `:indeterminate` delegation | CSS Selectors L4 §12.2.1 | The Selected-option Pseudo-classes: :checked, :unchecked, and :indeterminate | `#checked` | `heading --exact selectors-4 12.2.1` |
| `:valid` / `:invalid` delegation | CSS Selectors L4 §12.3.1 | The Validity Pseudo-classes: :valid and :invalid | `#validity-pseudos` | `heading --exact selectors-4 12.3.1` |
| `:required` / `:optional` delegation | CSS Selectors L4 §12.3.3 | The Optionality Pseudo-classes: :required and :optional | `#opt-pseudos` | `heading --exact selectors-4 12.3.3` |

⚠ **The input-state sections are NOT consecutive for the three button types**: §4.10.5.1.18 = Submit Button, **§4.10.5.1.19 = Image Button**, §4.10.5.1.20 = Reset Button, §4.10.5.1.21 = Button. A "§4.10.5.1.19/20/21 = submit/reset/button" reading is off-by-one and lands on Image Button — looked up, not assumed (§13 item 10).

---

## §1 Ideal anchor — the answer becomes derivable, so the cache has nothing to be stale about

The umbrella's thesis (§1.3): **the cache *is* the bug**. Slice 1 is where that thesis becomes code. The ideal end-state, stated positively:

> Every form pseudo-class answer is a **pure function of its own source of truth** — content attributes, tree ancestry, and the live `FormControlState` value model — evaluated **at the single site that asks the question**. There is no second representation of the answer, therefore no invalidation protocol, therefore no staleness class.

Concretely: `crates/css/elidex-css/src/selector/matching.rs` (671 lines; production 1–453, inline `mod tests` 454–671) stops reading bits and starts calling predicates authored in the `elidex-form-core` leaf crate, and the bits are deleted from `crates/core/elidex-ecs/src/components.rs`.

### §1.1 What deletes

| Deleted | Where | Why it can go |
|---|---|---|
| 7 `ElementState` form-bit constants | `crates/core/elidex-ecs/src/components.rs:430-436` | provably unread post-migration (I3) |
| `form_element_state()` (the bit fetch) | `crates/css/elidex-css/src/selector/matching.rs:126-133` | its whole job was fetching bits |
| `is_form_element()` (its only caller's only gate) | `crates/css/elidex-css/src/selector/matching.rs:371-385` | dead once `form_element_state` goes (§5) |
| 6 dead `state_flag_for_pseudo` arms | `crates/css/elidex-css/src/selector/matching.rs:360-365` | 4 already dead today; `valid`/`invalid` join them |
| 6 ElementState-writing sites (W1–W6) | see §5 | nothing reads what they write |

`ElementState` **survives** with `HOVER`/`FOCUS`/`ACTIVE`/`LINK`/`VISITED` (`components.rs:425-429`) — genuinely UI/interaction state with real writers and a real reader (`matching.rs:204-211`).

### §1.2 Why this is the keystone and not one more increment

Slice 0a made the derivation *reachable* (leaf crate, no `css→form→dom-api→css` cycle). Slice 0b made `FormControlState` *faithful* to its content attributes. PR-A makes the *candidacy predicate* spec-correct. Slice 1 is the only one of the four that removes a **representation**. After it, "where does `:checked` come from?" has exactly one answer.

---

## §2 Coupled invariants (edge-dense enumeration + every pairwise intersection)

Seven axes intersect here. Per `.claude/skills/elidex-plan-review/SKILL.md` Pre-condition #3, each invariant is named **and** every pair's intersection is stated in one line — prose 一筆書き is what leaks in this class of work.

| # | Axis | Invariant | What it forces |
|---|---|---|---|
| **I1** | SoT-layering | Each pseudo-class answer has exactly **one** source of truth: content attribute (`disabled`/`required`/`readonly`/`selected`/`value`), tree ancestry (fieldset / editing host), or the live `FormControlState` value model (checkedness / indeterminateness / value). | A predicate must read *its own* SoT, not a convenience mirror. `is_effectively_disabled` therefore reads `Attributes` + tree, **never** `FCS.disabled` (D5) — otherwise it inherits the field's init-time fieldset fold-in and dies with it in Slice 5. |
| **I2** | Derivation-timing | `el.matches(':checked')` / `querySelector` / `closest` **bypass the style walk entirely** — the `Matches` handler's `invoke` (`crates/dom/elidex-dom-api/src/child_node/selectors.rs:21-41`) calls `sel.matches(this, dom)` (`crates/css/elidex-css/src/selector/mod.rs:43`) → `matching.rs`; no `resolve_styles*` on the path (verified 2026-07-26). | Derivation must sit at the **reader**, not in a restyle pass. A script mutates and re-reads in the same task. |
| **I3** | Reader-set | `matching.rs` is the **only** production reader of any of the 7 form bits (re-verified below). But three *whole-value* read-modify-write helpers copy the `u16` through and keep compiling after the bits are deleted. | The deletion must be **driven by removing the constants** — the compiler then finds every writer. A grep-driven deletion would miss nothing here but would not *prove* it; constant-removal does. |
| **I4** | Spec-fidelity | HTML §4.16.3 defines each pseudo-class over an **element set** × a **condition**, and the two are not complements (`:enabled` is scope-list-gated; `:disabled` is unrestricted; `:required`/`:optional` both exclude non-applying inputs; `:valid`/`:invalid` are tri-state). | The matcher cannot be written as `flag` / `!flag`. Each pseudo-class needs its own scope gate + a tri-state-capable predicate where the spec is tri-state (§4). |
| **I5** | Perf (hot path) | Matching runs per element × per selector. | The derivation must be gated on the pseudo-class actually appearing in a selector (it is — `match_pseudo_class` `matching.rs:213-214` dispatches by name), must not allocate on the common path, and must not compile a regex per match. |
| **I6** | FCS-absent contract | `FormControlState` attaches only to `input`/`button`/`textarea`/`select`/`output`/`meter`/`progress` (`crates/dom/elidex-form-core/src/lib.rs:819-840` `from_element`) — **not** `fieldset`, **not** `option`. Detached / synthetic / pre-init elements may have none. | **Every** predicate must define its FCS-absent answer explicitly and never unwrap (§4.3). This replaces today's *ElementState*-absent asymmetry (`matching.rs:246-318`) with an *FCS*-absent contract — a different, smaller set, so the umbrella's "five flips" claim needs restating (§6-B). |
| **I7** | Crate-graph acyclicity | `elidex-css` cannot depend on `elidex-form` (`css → form → dom-api → css`). `elidex-form-core` deps = `elidex-plugin`, `elidex-ecs`, `regex`, `tracing`, `url` (`crates/dom/elidex-form-core/Cargo.toml`) — a true leaf. | The predicates must live in `elidex-form-core`, and the new `elidex-css → elidex-form-core` edge is the *only* Cargo change. Precedent shape: `elidex-custom-elements` was added to `elidex-css` for `:defined` (`matching.rs:226-234`). |

**I3 verification artifact** — form-bit readers/writers, **verified 2026-07-26** via
`grep -rn "ElementState::\(DISABLED\|CHECKED\|REQUIRED\|VALID\|INVALID\|READ_ONLY\|INDETERMINATE\)" crates/ --include='*.rs'` → 49 hits total:

- **production reads (9)** — all in `matching.rs`: lines 247/254/272/288/297/303/307/314 + the flag table at 360-365.
- **production writes (11 statements at 6 sites)** — `init.rs` 92/157/160/163/166/169, `fieldset.rs` 66, `radio.rs` 84/93, `clone.rs` 148/149, `form_input.rs` 38.
- **tests (29)** — `matching.rs` 465/476/487/516/534/545/556/570/582/593/603, `selector/tests/matching.rs` 719/741/800/813/831/843/861/872, `elidex-form/clone.rs` 225/227/253, `elidex-form/fieldset.rs` 102.
- **zero** hits in a11y / render / layout / devtools / crawler / wpt / serde / `Debug` formatting / any second pseudo-class matcher.

**I3 pass-through helpers (compile-clean after deletion, so they must be reasoned about, not grepped for)** — `crates/css/elidex-style/src/walk.rs:711-718` (`set_link_state`), `crates/dom/elidex-dom-api/src/focus/sot.rs:194-205` (`update_state`), `crates/shell/elidex-shell/src/app/hover.rs:53-66` (`update_element_state`). Each does `get::<&ElementState>().map_or(default, |s| *s)` → mutate → `insert_one`. They touch only kept bits and survive unchanged; only `form_input.rs:36-39`'s *use* of `update_element_state` dies.

### §2.1 Pairwise intersections (all 21)

| Pair | Intersection |
|---|---|
| I1 × I2 | The SoT is live at read time (attributes/tree/FCS all current), so reader-placement needs no snapshot — the two axes select "derive inside the matcher" uniquely. |
| I1 × I3 | Single reader ⇒ moving to per-SoT derivation is a *local* change; no consumer has to be re-taught which SoT a bit summarized. |
| I1 × I4 | Per-SoT reads are what make the non-complementary spec shapes expressible: `:required`/`:optional` need the *type* attribute (apply-list) as well as the `required` attribute — a single bit can never encode that. |
| I1 × I5 | Attribute + tree reads are `hecs` component gets and a bounded ancestor walk (`MAX_ANCESTOR_DEPTH`, `crates/core/elidex-ecs/src/dom/mod.rs:42`), so per-SoT reading costs no more than the bit fetch it replaces — except for validity (I5 × I6). |
| I1 × I6 | For attribute/tree-sourced predicates the SoT survives FCS absence ⇒ they answer fully; only the FCS-sourced predicates (`:checked` input branch, `:indeterminate` checkbox branch, `:valid`/`:invalid`) have a real absent case. This is *why* the absent contract is per-predicate rather than one blanket rule. |
| I1 × I7 | Every SoT the predicates need (`Attributes`, `TagType`, tree nav, `EcsDom`'s `is_contenteditable`, `FormControlState`) is nameable from `elidex-ecs` + `elidex-form-core` — no predicate needs anything above the leaf. That is the check that the layering is possible at all. |
| I2 × I3 | Reader-placement is only affordable *because* there is one reader; with N readers it would be N derivation call sites and a shared-cache argument would return. |
| I2 × I4 | Deriving at read time is what makes the tri-states honest: a bit cannot represent "no validity semantics", but a function returning `Option<bool>` at read time can. |
| I2 × I5 | Read-time derivation moves cost from "every mutation" to "every match" — bounded because the matcher only calls a predicate when the selector names that pseudo-class (`matching.rs:213-214`). |
| I2 × I6 | At read time the entity may legitimately have no FCS (detached / synthetic); a push design would simply never have written a bit, so the absent case is *newly reachable* and must be specified, not discovered. |
| I2 × I7 | The reader lives in `elidex-css`, below `elidex-form` — so "derive at the reader" is exactly the constraint that forced the 0a carve; Slice 1 is where that carve pays. |
| I3 × I4 | Sole-reader means the spec fixes land at one place with no cross-consumer skew — `:disabled` and `submit.rs`'s entry-list exclusion are the one exception, which is why `submit.rs:353` is bundled (§5-M1). |
| I3 × I5 | One reader ⇒ one place to measure; the O3 measurement (§10-Q3) is a single-call-site benchmark, not a fleet audit. |
| I3 × I6 | The sole reader is also the sole place the absent contract is observable, so §8's FCS-absent tests can assert at the pseudo-class level rather than at each predicate in isolation (they do both). |
| I3 × I7 | Sole reader ⇒ exactly **one** new Cargo edge (`elidex-css → elidex-form-core`); no other crate needs the predicates in Slice 1. |
| I4 × I5 | The only spec-faithful answer with real cost is validity (constraint evaluation). It is also the only one gated on a measurement (§10-Q3) — the two axes intersect at exactly one pseudo-class pair, which is why D3 is the headline decision. |
| I4 × I6 | Spec tri-states and component absence are *different* reasons to match neither pseudo-class, and both must map onto the same `Option` return so the matcher cannot accidentally collapse them into "false" (§4.3). |
| I4 × I7 | Spec fidelity for `:read-write` needs `EcsDom`'s `is_contenteditable` (`crates/core/elidex-ecs/src/dom/mod.rs:335-352`) — in `elidex-ecs`, reachable from the leaf. Fidelity for the `option`/`optgroup` disabled branches needs `is_option_disabled`, which today lives **above** `elidex-css` (`crates/dom/elidex-dom-api/src/element/option_disabled.rs:30`) — unreachable, which is *why* that branch is a scoped-out slice boundary (§6-D) and not an omission. |
| I5 × I6 | The FCS-absent early return is also the cheapest path (a failed component get), so the absent contract costs nothing; conversely the FCS-present validity path is the only one worth measuring. |
| I5 × I7 | The leaf crate has no heavier dependency to reach for (no `dom-api` walkers), so the predicates are structurally forced to stay cheap. |
| I6 × I7 | `FormControlState` is nameable only from `elidex-form-core`, so the absent-case handling **must** be inside the predicate (the matcher cannot pattern-match on the component) — the absent contract is therefore a leaf-crate API guarantee, not a matcher convention. |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.16.3 Pseudo-classes | `:disabled` | any element that is *actually disabled* (unrestricted — **not** scope-list gated) | `match_form_pseudo_class` `"disabled"` arm → `is_effectively_disabled` | ✓ | yes — `setAttribute("disabled")`, `el.disabled=` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:enabled` | scope list (button, input, select, textarea, optgroup, option, fieldset, FACE) ∧ ¬actually-disabled — **optgroup/option/FACE OUT (§6-D)** | `"enabled"` arm → in-scope tag gate ∧ `!is_effectively_disabled` | ✓ (in-scope 5 of 8 enumerated; 3 scoped out with slot) | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:checked` | `input[type=checkbox]` ∧ checkedness | `"checked"` arm → `is_checked` → `FCS.checked` | ✓ | yes — `i.checked=`, click |
| WHATWG HTML §4.16.3 Pseudo-classes | `:checked` | `input[type=radio]` ∧ checkedness | same arm → `FCS.checked` | ✓ | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:checked` | `option` ∧ **selectedness** | same arm → `selected` **content attribute** (documented spec divergence, §6-D, slot `#11-select-value-split-sot`) | ✗ — divergence declared, not silent | yes — `select.value=`, `option.selected=` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:indeterminate` | `input[type=checkbox]` ∧ indeterminateness | `"indeterminate"` arm → `is_indeterminate` → `FCS.indeterminate` | ✓ | yes — `i.indeterminate=` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:indeterminate` | `input[type=radio]` ∧ its radio button group has no checked member | **OUT** — needs form-owner + group resolution (a *system*, §6-D, slot `#11-radio-group-indeterminate-pseudo`) | ✗ — scoped out with slot; pre-existing gap, non-regressing | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:indeterminate` | `progress` with **no** `value` content attribute | same arm → `!attrs.contains("value")` (NEW branch — today unreachable, §6-A) | ✓ | yes — `setAttribute("value")` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:required` | input that *is required* / select with `required` / textarea with `required` | `"required"` arm → `requiredness(..) == Some(true)` | ✓ | yes — `setAttribute("required")`, `el.required=` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:optional` | input **to which `required` applies** ∧ ¬required / select ¬required / textarea ¬required | `"optional"` arm → `requiredness(..) == Some(false)` | ✓ (the `None` = matches-neither case is explicit) | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:read-write` | input to which `readonly` **applies** ∧ mutable (no `readonly` attr ∧ not disabled) | `"read-write"` arm → `is_read_write` input branch | ✓ | yes — `setAttribute("readonly")`, `el.readOnly=`, `el.disabled=` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:read-write` | textarea with no `readonly` attr ∧ not disabled | same predicate, textarea branch | ✓ | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:read-write` | element that is an editing host / editable ∧ neither input nor textarea | same predicate, `_` branch → `EcsDom`'s `is_contenteditable` | ✓ (fidelity of `is_contenteditable` itself is pre-existing, §6-D) | yes — `contenteditable` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:read-only` | **all other HTML elements** (exact complement) | `"read-only"` arm → `!is_read_write` (D4) | ✓ | yes |
| WHATWG HTML §4.16.3 Pseudo-classes | `:valid` / `:invalid` | ELEMENT bullet — candidate for constraint validation ∧ (¬)satisfies its constraints | `"valid"`/`"invalid"` arms → `constraint_validity` → `is_constraint_validation_candidate` + `validate_control` | ✓ | yes — every value keystroke / `setCustomValidity` |
| WHATWG HTML §4.16.3 Pseudo-classes | `:valid` / `:invalid` | AGGREGATE bullets — `form` by form-ownership, `fieldset` by descendants | **OUT** — re-scoped umbrella Slice 3 (D3) | ✗ — scoped out, owning slice named | yes |
| WHATWG HTML §4.15 Disabled elements | "actually disabled" | button / input / select / textarea that is *disabled* | `is_effectively_disabled` tag arm | ✓ | yes |
| WHATWG HTML §4.15 Disabled elements | "actually disabled" | fieldset that is a *disabled fieldset* | same arm (unified disjunction) | ✓ | yes |
| WHATWG HTML §4.15 Disabled elements | "actually disabled" | optgroup / option / form-associated custom element | **OUT** — `is_option_disabled` is above `elidex-css` (§6-D, slot `#11-actually-disabled-option-optgroup-branches`) | ✗ — scoped out with slot; pre-existing gap | yes |
| WHATWG HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute | "a form control is disabled" | own `disabled` content attribute | `is_effectively_disabled` (attribute half) | ✓ | yes |
| WHATWG HTML §4.10.19.5 Enabling and disabling form controls: the disabled attribute | "a form control is disabled" | descendant of `fieldset[disabled]`, not in its first `legend` child | `is_fieldset_disabled` (`crates/dom/elidex-form-core/src/fieldset.rs:46-76`, existing) | ✓ | yes — `fs.disabled=`, insert/move |
| WHATWG HTML §4.10.15 The fieldset element | "disabled fieldset" | own `disabled` attr **or** disabled-fieldset ancestry | same unified disjunction as above | ✓ | yes |
| WHATWG HTML §4.10.21.1 Definitions | candidate for constraint validation | submittable element ∧ ¬barred | `is_constraint_validation_candidate` (`crates/dom/elidex-form-core/src/validation/mod.rs:21-44`) — **consumed as-is; PR-A already corrected its basis and bars** | ✓ (PR-A owns the branch enumeration) | yes |
| WHATWG HTML §4.10.21.1 Definitions | satisfies its constraints | no validity state is set | `validate_control` (`validation/mod.rs:104`) + `ValidityState::is_valid` (`:88`) | ✓ (existing algorithm reused verbatim) | yes |
| WHATWG HTML §4.10.12 The output element | (category) | `<output>` is not submittable ⇒ never a candidate (true both before and after PR-A) | the `<output>` special-case at `matching.rs:261-267` is deleted; candidacy answers instead (§6-A row 9) | ✓ | no |
| WHATWG HTML §4.10.22.4 Constructing the entry list | step 5.1 | field is **disabled** ⇒ continue (uses the §4.10.19.5 concept) | `collect_control_entry` (`crates/dom/elidex-form/src/submit.rs:349-355`) migrates `fcs.disabled` → `is_effectively_disabled` (§5-M1) | ✓ | yes — form submit |
| WHATWG HTML §4.10.18.2 Mutability | "mutable" | the `:read-write` parenthetical (no `readonly` ∧ not disabled) | `is_read_write` input/textarea branches | ✓ | yes |
| WHATWG HTML §4.10.5.3.3 The readonly attribute | apply-list | `readonly` does not apply to checkbox/radio/range/color/file/hidden/button-types | `FormControlKind::readonly_applies` (`crates/dom/elidex-form-core/src/lib.rs:178-195`, existing) | ✓ | yes |
| WHATWG HTML §4.10.5.3.4 The required attribute | apply-list | `required` does not apply to hidden/range/color/submit/image/reset/button | `FormControlKind::required_applies` (NEW, §4.2) | ✓ | yes |
| WHATWG HTML §4.10.10 The option element | selectedness | live selectedness is the `:checked` SoT | **divergence declared** — Slice 1 keeps the `selected` content-attribute read (§6-D) | ✗ — declared, slot-tracked | yes |
| WHATWG HTML §4.10.5.1.16 Radio Button state (type=radio) | radio button group | group membership for the `:indeterminate` radio branch | **OUT** (§6-D) | ✗ — scoped out with slot | yes |
| WHATWG HTML §6.8.4 Editing APIs | editing host | the `:read-write` third bullet | `EcsDom`'s `is_contenteditable` (`crates/core/elidex-ecs/src/dom/mod.rs:335-352`, existing, unchanged) | ✓ for the bullet; `plaintext-only`/`designMode` fidelity is pre-existing (§6-D) | yes |
| CSS Selectors L4 §12.1.1 The :enabled and :disabled Pseudo-classes | host-language delegation | "the host language defines" ⇒ HTML §4.16.3 governs | no separate code path | ✓ | n/a |
| CSS Selectors L4 §12.1.2 The Mutability Pseudo-classes: :read-only and :read-write | host-language delegation | ditto | no separate code path | ✓ | n/a |
| CSS Selectors L4 §12.2.1 The Selected-option Pseudo-classes: :checked, :unchecked, and :indeterminate | `:unchecked` + "indeterminate wins" precedence | Selectors-4-only additions with **no** HTML §4.16.3 counterpart | **OUT** — noted, not implemented (§6-D) | ✗ — declared out | n/a |
| CSS Selectors L4 §12.3.1 The Validity Pseudo-classes: :valid and :invalid | host-language delegation | ditto | no separate code path | ✓ | n/a |
| CSS Selectors L4 §12.3.3 The Optionality Pseudo-classes: :required and :optional | host-language delegation | ditto | no separate code path | ✓ | n/a |

**Breadth** (all citations webref-verified 2026-07-26, evidence in §0.5): K=2 specs (WHATWG HTML, CSS Selectors L4), M=37 rows → preflight verdict **SPLIT-DEFAULT (M≥30)**.

Carving PR-A out took M from 44 → 37. **That did not change the verdict and was never meant to**: the trigger for the carve was axis-orthogonality (§10-Q1a), and M=37 is still SPLIT-DEFAULT. The count is not what carries this PR — the appeal below is. Stated plainly so no reader mistakes a lower number for a resolved objection.

### §3.1 Split-decision narrative — the positive appeal for a single PR (D7)

The breadth verdict is high because the map enumerates **branches of one pseudo-class table**, not independent surfaces. The single-PR case is structural, not budgetary:

1. **The unit of deletion is one component field with one reader.** The 7 bits are `ElementState`'s low-order form half; `matching.rs` is their sole production reader (I3, artifact above). Removing the constants is what *finds* the writers — the compiler, not a grep. There is no sub-slice of "delete 3 of 7 constants" that compiles without either leaving dead bits or leaving a second matching mechanism.
2. **Any pseudo-class-wise sub-slice recreates the strangler state One-issue-one-way forbids.** "New predicate for `:checked`, bits still live for `:disabled`" means `form_element_state()`, `apply_element_state_flags`, and `state_flag_for_pseudo` all survive, and every subsequent reader/PR re-litigates "which mechanism does this pseudo-class use?" — the decision tax the discipline exists to delete.
3. **The umbrella already ruled on this** (§6, closing paragraph): "at its own mandated `/elidex-plan-review`, confirm the ElementState-cache flip is atomic against the real diff — do **not** sub-slice by pseudo-class."
4. **What genuinely could be split already has been — three times.** 0a (crate carve), 0b (reconciler completeness), and now **PR-A** (constraint-validation candidacy) were carved out precisely so Slice 1 is only the flip; Slice 2 (other effective-disabledness consumers), Slice 3 (validity aggregates + perf decision), Slice 4 (dirty checkedness), Slice 5 (push-propagation tail) stay out. §6-D scopes out five further surfaces. The carve test each time was the same: *is this a different axis that stands alone and leaves no dual mechanism behind?* Applied honestly, it keeps returning "yes" for the neighbours and "no" for the bit-deletion itself.
5. **CLAUDE.md's base case applies**: an approved umbrella's narrowly-scoped per-PR slice that has passed its own plan-review is a terminal unit. Re-splitting a terminal slice is the infinite regress the rule explicitly forecloses.

Residual risk is handled by review depth, not by splitting: §6's delta table is the review's checklist, and §8 pairs a test with every row.

### §3.2 User-input touch audit

Every touched path is an **existing** read path in the selector matcher; no new opcode, native, or dispatch surface is added. User-controllable input reaching the predicates: content attributes (`disabled`/`required`/`readonly`/`type`/`selected`/`value`/`contenteditable`) from the parser, `setAttribute`, or reflecting IDL setters; live `FormControlState` fields from IDL setters and user interaction. All already flow into `matching.rs` today — Slice 1 changes *which* SoT is consulted, not *whether* attacker-controlled data reaches the matcher.

Exposure delta:
- **Reduced write surface**: 6 ElementState-writing sites deleted, including one in `crates/script/elidex-js/src/vm/host/`-adjacent shell code (`form_input.rs`) — fewer places that can put the engine in an inconsistent state.
- **Added read cost** on the `:valid`/`:invalid` path: `validate_control` may run a user-supplied `pattern` regex. It is **matched, not compiled**, per match — `FCS.cached_pattern_regex` (`crates/dom/elidex-form-core/src/lib.rs:348-432`, field documented `Option<Option<Arc<Regex>>>`) is populated by the FCS `update_pattern` method (`crates/dom/elidex-form-core/src/lib.rs:487`) behind the `MAX_PATTERN_LENGTH` guard, so pattern-based ReDoS exposure is unchanged from today's `checkValidity()` path (§10-Q3 measures the throughput side).
- **Adjacent pre-existing behavior**: the `is_contenteditable` ancestor walk is `MAX_ANCESTOR_DEPTH`-bounded and unchanged; `is_fieldset_disabled`'s walk likewise.

---

## §4 Predicate API

### §4.1 Module placement

New module **`crates/dom/elidex-form-core/src/pseudo.rs` (NEW)** — *not* `lib.rs`, which is at 960 lines (§9). Rationale: the pseudo-class derivations are a cohesive surface (one spec section, one consumer) and deserve their own file from birth rather than being appended to the crate root and split later. `lib.rs` gains only `pub mod pseudo;` + the `pub use` re-exports.

The **`elidex-form` facade** (`crates/dom/elidex-form/src/lib.rs`) re-exports them too, per the 0a §6 contract — but note this is exactly the dual-path moment slot **`#11-form-core-facade-shrink`** was registered for (0a §6): Slice 1's first `elidex_form_core` import in `elidex-css` is the declared trigger. §11 audits it.

### §4.2 Signatures

```rust
// crates/dom/elidex-form-core/src/pseudo.rs (NEW)
//
// Pure pull derivations for the HTML §4.16.3 form pseudo-classes.
// Each reads ITS OWN source of truth (Attributes / tree / FormControlState).
// No cache, no invalidation, no `unwrap`.

/// HTML §4.15 "actually disabled", in-scope branches (§4.10.19.5 form
/// controls + §4.10.15 disabled fieldsets). Reads `Attributes` + tree —
/// NEVER `FormControlState::disabled` (D5).
pub fn is_effectively_disabled(entity: Entity, dom: &EcsDom) -> bool;

/// HTML §4.16.3 `:checked`.
pub fn is_checked(entity: Entity, dom: &EcsDom) -> bool;

/// HTML §4.16.3 `:indeterminate` (checkbox + progress branches; the
/// radio-group branch is scoped out, see the module docs).
pub fn is_indeterminate(entity: Entity, dom: &EcsDom) -> bool;

/// HTML §4.16.3 `:required` / `:optional`. TRI-STATE:
/// `None`  = outside both pseudo-classes' element sets → matches NEITHER
/// `Some(true)`  = `:required`
/// `Some(false)` = `:optional`
pub fn requiredness(entity: Entity, dom: &EcsDom) -> Option<bool>;

/// HTML §4.16.3 `:read-write`. `:read-only` is its exact complement over
/// ALL elements ("must match all other HTML elements") — the matcher
/// derives `:read-only` as `!is_read_write` (D4). Never author a
/// form-control-local `is_read_only`.
pub fn is_read_write(entity: Entity, dom: &EcsDom) -> bool;

/// HTML §4.16.3 `:valid` / `:invalid`, ELEMENT bullet only. TRI-STATE:
/// `None`  = no validity semantics (not a candidate) → matches NEITHER
/// `Some(true)`  = candidate ∧ satisfies its constraints → `:valid`
/// `Some(false)` = candidate ∧ does not satisfy → `:invalid`
/// The `<form>` / `<fieldset>` AGGREGATE bullets are Slice 3 (D3).
pub fn constraint_validity(entity: Entity, dom: &EcsDom) -> Option<bool>;
```

Plus one apply-list method, authored beside its sibling so the "which types does attribute X apply to" question keeps a single home:

```rust
// crates/dom/elidex-form-core/src/lib.rs, in `impl FormControlKind`,
// immediately after `readonly_applies` (:178-195).
/// HTML §4.10.5.3.4 — the `required` attribute's apply-list. Distinct
/// from `readonly_applies`: `required` DOES apply to Checkbox / Radio /
/// File, which `readonly` does not.
#[must_use]
pub fn required_applies(self) -> bool;
```

`FormControlKind::required_applies` (NEW), `elidex_form_core::pseudo::is_effectively_disabled` (NEW), `elidex_form_core::pseudo::constraint_validity` (NEW), `elidex_form_core::pseudo::requiredness` (NEW), `elidex_form_core::pseudo::is_read_write` (NEW).

### §4.3 Bodies (the load-bearing shapes) + the FCS-absent answer for **every** predicate

| Predicate | Body sketch | Needs FCS? | **FCS-absent answer** |
|---|---|---|---|
| `is_effectively_disabled` | `match tag { "button"\|"input"\|"select"\|"textarea"\|"fieldset" => attrs.contains("disabled") \|\| is_fieldset_disabled(entity, dom), _ => false }` — §4.10.19.5 and §4.10.15 are the **same disjunction**, which is why five tags share one arm | **no** | full answer (attribute + tree only) |
| `is_checked` | `input` ∧ type∈{checkbox,radio} → `FCS.checked`; `option` → `attrs.contains("selected")` (declared divergence); else `false` | yes (input branch) | `false` — an entity with no value model has no checkedness |
| `is_indeterminate` | `input[checkbox]` → `FCS.indeterminate`; `input[radio]` → `false` (branch scoped out); `progress` → `!attrs.contains("value")`; else `false` | yes (checkbox branch) | `false` |
| `requiredness` | `input` → `FormControlKind::from_tag_and_type_attr("input", attrs.get("type"))` (`lib.rs:232-247`) → `.required_applies()` ? `Some(attrs.contains("required"))` : `None`; `select`\|`textarea` → `Some(attrs.contains("required"))`; else `None` | **no** | full answer (attributes only) |
| `is_read_write` | `input` → `kind.readonly_applies() && !attrs.contains("readonly") && !is_effectively_disabled(..)`; `textarea` → `!attrs.contains("readonly") && !is_effectively_disabled(..)`; `_` → `dom.is_contenteditable(entity)` | **no** | full answer (attributes + tree) |
| `constraint_validity` | `let fcs = dom.world().get::<&FormControlState>(entity).ok()?;` → `is_constraint_validation_candidate(&fcs, entity, dom)` ? `Some(validate_control(&fcs).is_valid())` : `None` | yes | **`None`** — no value model ⇒ not a candidate ⇒ matches **neither** `:valid` nor `:invalid` |

**Correction to umbrella §3.1** (which specified `validity(...)` "absent→valid"): returning "valid" for an entity with no value model would assert that a thing with no constraints *satisfies* them, and it would make `:valid` fire on pre-init/synthetic elements. `None` is both spec-faithful (the §4.16.3 element bullet requires *candidate* status) and consistent with `:valid`/`:invalid` being tri-state. This changes the umbrella's I6 flip count — see §6-B.

### §4.4 Matcher side (`matching.rs`)

`match_form_pseudo_class` (`matching.rs:240-321`) is rewritten as a thin mapping from pseudo-class name to (element-set gate, predicate):

```rust
"disabled"      => is_effectively_disabled(entity, dom),          // unrestricted (§4.16.3)
"enabled"       => is_enabled_scope(entity, dom) && !is_effectively_disabled(entity, dom),
"checked"       => is_checked(entity, dom),
"indeterminate" => is_indeterminate(entity, dom),
"required"      => requiredness(entity, dom) == Some(true),
"optional"      => requiredness(entity, dom) == Some(false),
"read-write"    => is_read_write(entity, dom),
"read-only"     => !is_read_write(entity, dom),                   // D4: exact complement
"valid"         => constraint_validity(entity, dom) == Some(true),
"invalid"       => constraint_validity(entity, dom) == Some(false),
```

**Layering split, stated once**: `elidex-form-core` owns the **HTML concept** ("is this element actually disabled / checked / required / …"); `matching.rs` owns the **Selectors mapping** (which element set each pseudo-class ranges over, and the complement/`Option`-comparison shape). The element-set gate `is_enabled_scope` (NEW, matcher-local: `button`\|`input`\|`select`\|`textarea`\|`fieldset`) therefore stays in `matching.rs` — it is matcher data with one consumer. If a second consumer ever appears it moves down, by the same rule that moved the predicates.

The kept-bit path (`matching.rs:204-211`) and `state_flag_for_pseudo`'s 5 surviving arms are untouched.

---

## §5 The deletion set, and how each removal is proven safe

**Driver**: remove the 7 constants at `crates/core/elidex-ecs/src/components.rs:430-436` **first**; `cargo check --workspace --all-features` then enumerates every remaining reference. The list below is the expected result, pre-verified 2026-07-26 by the I3 grep artifact (§2) — the compile is the proof, the grep is the plan.

| ID | Site | Statement(s) deleted | What survives | Safety proof |
|---|---|---|---|---|
| **W1** | `crates/dom/elidex-form/src/init.rs:150-171` `apply_element_state_flags` | whole fn + its call at `:146` inside `finalize_control` (`:128-147`) | — | Its only effect was writing the 5 bits (DISABLED/CHECKED/REQUIRED/READ_ONLY/VALID). It was also the only site inserting an `ElementState` at FCS-creation time; that is fine — all three surviving writers (`walk.rs:711-718`, `focus/sot.rs:194-205`, `hover.rs:53-66`) and the reader (`matching.rs:204-211`) already handle component-absent via `map_or(ElementState::default(), …)`. |
| **W2** | `crates/dom/elidex-form/src/init.rs:89-94` (fieldset fold-in in `create_form_control_state`, `:63-97`) | `:91-93` only (the `if let Ok(mut es) … es.insert(DISABLED)`) | `:90` `state.disabled = true` — **stays until Slice 5** (umbrella §6) | The FCS half still feeds `submit`/`radio`/`validation` until their Slice-2/5 migrations; only the bit write dies. |
| **W3** | `crates/dom/elidex-form/src/fieldset.rs:60-68` (`disable_descendants`, `:35-71`) | `:65-67` (`es.insert(DISABLED)`) | `:63` `fcs.disabled = true` — stays until Slice 5 | Same as W2. **Also delete the test assertion `fieldset.rs:98-102`** (`fieldset_disables_descendants` asserts the bit) — retain the FCS half of that test. *(This test was missing from the scouted writer list; see §13.)* |
| **W4** | `crates/dom/elidex-form/src/radio.rs:83-85` + `:92-94` (`toggle_radio`, `:59-96`) | both `if let Ok(mut es) …` blocks | `fcs.checked` writes at `:81`/`:90` | The only writer that ever *removed* a form bit — with the pull derivation, `:checked` follows `FCS.checked` with no second write. Shrinks `radio.rs:3`'s `use elidex_ecs::{… ElementState …}`. |
| **W5** | `crates/shell/elidex-shell/src/content/form_input.rs:36-39` | the `update_element_state(dom, entity, \|s\| s.set(CHECKED, new_checked))` call | `fcs.checked` toggle at `:29-31` | `update_element_state` (`hover.rs:53-66`) keeps its HOVER callers (`hover.rs:40`/`:47`), so the helper stays. Check whether `form_input.rs`'s `DomElementState` import becomes unused. |
| **W6** | `crates/dom/elidex-form/src/clone.rs:146-151` (the #466 `:checked`/`:indeterminate` re-sync) | the whole `if kind != TextArea { … }` block | the FCS overlay at `:118-134` | With the pull derivation the clone's `:checked`/`:indeterminate` follow the *overlaid* `FCS` automatically — that is precisely what the re-sync was hand-simulating. **Coupled doc/test edits**: the explanatory comment `clone.rs:136-145`; the two test assertions `clone.rs:224-229` and `:251-255` (keep the FCS assertions, drop the bit assertions); and the clone-policy row at `crates/core/elidex-ecs/src/dom/tree_clone.rs:28`, whose current wording (retagged by #484 `f7d9b5ce`) says the init "re-runs … so the clone's attribute-derived fields and its *non-copy* `ElementState` bits stay single-sourced … then re-syncs `:checked` / `:indeterminate` to the *overlaid* values" — both clauses become false. |
| **C** | `crates/core/elidex-ecs/src/components.rs:430-436` | the 7 constants | HOVER/FOCUS/ACTIVE/LINK/VISITED (`:425-429`) | Provably dead post-W1–W6 (I3). Also update the `ElementState` doc comment at `:416-420` ("…and form-related pseudo-classes"). |
| **R1** | `crates/css/elidex-css/src/selector/matching.rs:126-133` `form_element_state` | whole fn | — | Its body *is* the bit fetch. |
| **R2** | `crates/css/elidex-css/src/selector/matching.rs:371-385` `is_form_element` | whole fn | — | Sole caller was `form_element_state:129` (verified 2026-07-26 via `grep -n is_form_element crates/css/elidex-css/src/selector/matching.rs` → 2 hits: the def and that call). Dead ⇒ "dead code は残さず削除". |
| **R3** | `crates/css/elidex-css/src/selector/matching.rs:360-365` | the `disabled`/`checked`/`required`/`indeterminate`/`valid`/`invalid` arms of `state_flag_for_pseudo` (`:353-368`) | the 5 kept-bit arms + the fn | 4 of the 6 are **already** dead today (only `:210` and the `valid`/`invalid` use at `:259` reach the fn); the other 2 die with the `:valid`/`:invalid` rewrite. |
| **R4** | `crates/css/elidex-css/src/selector/matching.rs` tag gates | `is_requirable_element` (`:139-143`), `is_checkable_element` (`:391-403`), `is_indeterminate_candidate` (`:408-417`), `is_disableable_element` (`:424-428`) | re-homed | These encode element sets that the new predicates own (`requiredness`'s `None`, `is_checked`'s tag match, …) or that `is_enabled_scope` replaces. Whether each is deleted or folded is a mechanical outcome of §4.4; the invariant is **no element-set logic exists in two places**. Their stale `§4.10.18.x` citations disappear with them (§6-C). |
| **M1** | `crates/dom/elidex-form/src/submit.rs:353` (`collect_control_entry`, `:349-355`) | `fcs.disabled` → `is_effectively_disabled(entity, dom)` | rest of the fn | **Bundled per umbrella §6/§4.1**: the §4.10.22.4 step-5.1 disabled exclusion must flip *with* `:disabled`, else a dynamically-fieldset-disabled control is greyed-but-submittable. Signature already carries `&EcsDom` + `entity` — no signature change. **Only the disabledness term changes**; the button exclusion in the same predicate is PR-A's. |
| **M2** | `crates/dom/elidex-form-core/src/validation/mod.rs:29-31` + `:43` (post-PR-A line numbers may shift by ~5) | the manual `state.disabled` / `!is_fieldset_disabled` pair → one `is_effectively_disabled(entity, dom)` call | rest of `is_constraint_validation_candidate` | **D3b (PM-decided).** Pulled forward from umbrella Slice 2 by the *identical* argument that bundled M1: Slice 1 makes `:disabled` live, and `:valid`/`:invalid` must not disagree with `:disabled` about whether the same control is disabled. Without it, a control initialised inside a `<fieldset disabled>` and then moved out reads `:enabled` but stays barred from validation. |

**Test rewrites** (D1-g): 37 selector tests exercise form pseudo-classes — 21 in `crates/css/elidex-css/src/selector/matching.rs` (all `fn`s after line 453) and 16 in `crates/css/elidex-css/src/selector/tests/matching.rs` (lines 673/682/695/706/714/726/736/748/766/777/787/807/825/838/855/868). **Verified 2026-07-26** via `awk 'NR>453 && /^    fn /' … | wc -l` → 21, and `awk '/^fn /{n=$0;l=NR;h=0} /:disabled|:enabled|:checked|:indeterminate|:required|:optional|:read-only|:read-write|:valid|:invalid|ElementState/{if(n!=""&&h==0){print l": "n;h=1}}' crates/css/elidex-css/src/selector/tests/matching.rs` → 19 rows, of which 3 (lines 457/489/502) are `:hover`/`:link`/`:visited` and stay. Of the 37, **19 seed a form bit directly** (`es.insert(ElementState::<form bit>)`: `matching.rs` 465/476/487/516/534/545/556/570/582/593/603 = 11; `tests/matching.rs` 719/741/800/813/831/843/861/872 = 8); the rest seed `ElementState::default()` or rely on absence.

The rewrite is *simpler* than the current seeding for most cases, because only `:checked` / `:indeterminate` / `:valid` / `:invalid` need a `FormControlState`:

- attribute-sourced pseudo-classes (`:disabled`/`:enabled`/`:required`/`:optional`/`:read-only`/`:read-write`) → seed `Attributes` (`attrs.set("disabled","")`), no component insert at all;
- live-state pseudo-classes → `FormControlState::from_element(tag, &attrs)` (`crates/dom/elidex-form-core/src/lib.rs:819-840`) + field write, mirroring `crates/dom/elidex-form/src/clone.rs`'s existing test helper shape.

`elidex-css`'s test seeding therefore needs `elidex-form-core` as a normal dependency (it already is one, for production) — no `[dev-dependencies]` addition.

## §6 Behavior-delta table

### §6-A Deltas that are spec-fidelity **fixes**

| # | Pseudo-class | Element / state | Today | After Slice 1 | Why (spec) | Test |
|---|---|---|---|---|---|---|
| 1 | `:checked` | `input[type=checkbox]` after `i.checked = true` (no `checked` attr) | **false** | true | §4.16.3 keys on checkedness, not the attribute | §8-V1 |
| 2 | `:indeterminate` | `input[type=checkbox]` after `i.indeterminate = true` | **false** | true | §4.16.3 indeterminateness | §8-V2 |
| 3 | `:disabled` / `:enabled` | control after `i.disabled = true` at runtime | stale | live | §4.15 + §4.10.19.5 | §8-V3 |
| 4 | `:disabled` / `:enabled` | control under a dynamically `fs.disabled = true` fieldset | **false** | true | §4.10.19.5 second branch | §8-V4 |
| 5 | `:disabled` / `:enabled` | control moved **out** of `<fieldset disabled>` | never re-enables | re-enables | ancestry is a live predicate | §8-V5 |
| 6 | `:disabled` | `<fieldset disabled>` itself | **false** (excluded by `is_disableable_element`, whose comment claims fieldset "does NOT participate") | **true** | §4.15 lists "a fieldset element that is a disabled fieldset"; §4.16.3 `:disabled` is unrestricted | §8-U6 |
| 7 | `:enabled` | non-disabled `<fieldset>` | **false** | **true** | §4.16.3 `:enabled` scope list explicitly includes `fieldset` | §8-U6 |
| 8 | `:indeterminate` | `<progress>` with no `value` attribute | **false** (bit never written) | **true** | §4.16.3 third `:indeterminate` bullet | §8-U7 |
| 9 | `:valid` | `<output>` | **true** (special-cased at `matching.rs:261-267` + the init VALID bit) | **false** (matches neither) | `<output>` is not a submittable element (§4.10.2) ⇒ never a candidate (§4.10.21.1) | §8-U8 — **flips the existing test `valid_matches_output_element`** (`crates/css/elidex-css/src/selector/tests/matching.rs:766`) |
| 10 | `:valid` | `<button type=button\|reset>` / `<input type=reset\|button>` (kinds `Button` / `ResetButton`) | **true** (init VALID bit) | **false** (matches neither) | **Slice 1's own flip**: PR-A already bars these kinds, so the pre-Slice-1 `:valid=true` comes purely from the unconditional VALID bit at `init.rs:169`. Switching the reader from the bit to the predicate is what changes the answer | §8-U9 |
| 10a | `:valid` | `<button type=submit>` **and** `<input type=submit>` (kind `SubmitButton`) | true | **true — NO delta** | PR-A made them candidates (§4.10.5.1.18 states no bar), and they satisfy their constraints vacuously, so bit and predicate agree. **This is the assertion that proves the PR-A → Slice-1 handoff is sound** | §8-U9b (non-regression) |
| 11 | `:invalid` | any control failing a constraint (e.g. `<input required>` empty) | **impossible** (INVALID has zero production writers) | true | §4.16.3 element bullet | §8-V6 |
| 12 | `:valid` | disabled / readonly-barred / `<input type=hidden>` control | **true** | **false** (matches neither) | barred ⇒ not a candidate (§4.10.21.1; hidden = §4.10.5.1.1) | §8-U10 |
| 13 | `:read-write` | `input[type=checkbox\|radio\|range\|color\|file\|hidden\|submit\|reset\|button]`, `<select>`, `<button>` | **true** | **false** (⇒ `:read-only` true) | §4.16.3 `:read-write` first bullet requires `readonly` to **apply**; `<select>`/`<button>` are neither input nor textarea, so only the editing-host bullet could match | §8-U11 |
| 14 | `:required` / `:optional` | `input[type=hidden\|range\|color\|submit\|image\|reset\|button]`, with or without `required` | `:optional` true (or `:required` true if the bit was seeded) | **neither** | §4.16.3 `:optional` requires `required` to apply; `:required` presupposes it (§4.10.5.3.4 apply-list) | §8-U12 |
| 15 | `:required` / `:optional` | after `setAttribute("required")` at runtime | stale (bit written only at init) | live | attribute is the SoT | §8-V7 |
| 16 | `:checked` | cloned `<input>` (`cloneNode`) | correct **via the W6 hand re-sync** | correct **by construction** | the overlaid `FCS` is the SoT | §8-C1 (existing `tests_clone_form_state.rs:100` VM assertion must stay green) |
| 17 | (submission) | fieldset-disabled control in the entry list | included when `FCS.disabled` is stale | excluded | §4.10.22.4 step 5.1 "field is disabled" | §8-V8 |
| 18 | `willValidate` / `checkValidity()` (JS) | control moved out of `<fieldset disabled>` after init | still barred | candidate | D3b (M2) — the submit-button candidacy flip is **PR-A's**, not this slice's | §8-U14 |

### §6-B Deltas on the **component-absent** boundary (correcting umbrella I6)

Umbrella I6 asserts that the new design "flips five pseudo-classes" for a form element with no component: `:read-only` t→f and `:enabled`/`:optional`/`:read-write`/`:valid` f→t. Two corrections, both from re-reading the actual code and the actual absent-case semantics:

1. **The boundary moves from `ElementState`-absent to `FCS`-absent.** Today's asymmetry is triggered by `form_element_state()` returning `None`, which happens when `ElementState` is missing (or the tag is outside the 8-tag `is_form_element` list). After Slice 1 the only component-absence that matters is `FormControlState` — and only for `:checked` / `:indeterminate` / `:valid` / `:invalid`. The other six pseudo-classes answer from attributes + tree regardless.
2. **`:valid` does *not* flip f→t.** Today an `<input>` with no `ElementState` falls to `matching.rs:261-267` → `name == "valid" && tag == "output"` → **false**. After Slice 1, an `<input>` with no `FCS` is not a candidate → **false**. Same answer, better reason. So the count is **four** flips, not five.

| Pseudo-class | `<input>` with **no** component | Today | After | Why |
|---|---|---|---|---|
| `:read-only` | — | **true** | false | `is_read_write` = readonly-applies ∧ no `readonly` attr ∧ not disabled = true ⇒ complement false |
| `:read-write` | — | false | **true** | ditto |
| `:enabled` | — | false | **true** | in scope ∧ not actually disabled |
| `:optional` | — | false | **true** | `requiredness` = `Some(false)` |
| `:valid` | — | false | false | **no flip** — not a candidate (correction to umbrella I6/§3.1) |
| `:invalid` | — | false | false | no flip |
| `:checked` / `:indeterminate` | — | false | false | FCS-absent ⇒ `false` |
| `:disabled` | — | false | false | no `disabled` attribute |
| `:required` | — | false | false | `requiredness` = `Some(false)` |

### §6-C Stale in-tree spec citations removed as a side effect

Not separate work — these comments are attached to code the slice deletes or rewrites, so leaving them would be shipping a known-wrong citation.

| Site | Current citation | Correct today (webref 2026-07-26) |
|---|---|---|
| `matching.rs:242-243` (`:disabled` comment) | "HTML §4.10.18.5" | §4.15 Disabled elements (`#concept-element-disabled`) + §4.10.19.5 |
| `matching.rs:249-250` (`:indeterminate`) | "HTML §4.10.18.3" | §4.16.3 Pseudo-classes |
| `matching.rs:256` (`:valid`/`:invalid` + `<output>`) | "HTML §4.10.18.7" | **§4.10.18.7 does not exist** (`heading --exact html 4.10.18.7` → no match); the claim itself is wrong — §4.10.12 + §4.10.2 say `<output>` is not a candidate |
| `matching.rs:137` / `:284` / `:299` (`:required`/`:optional`) | "HTML §4.10.15.2.4" | §4.16.3 (matching) + §4.10.5.3.4 (apply-list); §4.10.15 is "The fieldset element" |
| `matching.rs:405-407`, `:419-423` | "§4.10.18.3", "§4.10.18.5" | as above |
| `crates/dom/elidex-form-core/src/validation/mod.rs:12` | "HTML §4.10.20.3" | §4.10.21.1 Definitions (`#candidate-for-constraint-validation`) — **owned by PR-A** (which rewrites this fn); listed only so the Slice-1 reviewer does not re-flag it |

### §6-D Explicitly OUT of Slice 1 — scope boundaries, not deferred defects

Each is **pre-existing and non-regressing** (Slice 1 leaves it exactly as correct as `main`), with a cited reason and a durable `#11-*` home.

1. **`optgroup` / `option` / form-associated-custom-element branches of "actually disabled"** (§4.15 bullets 5/6/8).
   *Reason*: the option/optgroup answer needs `is_option_disabled`, which lives at `crates/dom/elidex-dom-api/src/element/option_disabled.rs:30` — in a crate **above** `elidex-css` (I4 × I7). Making it reachable is its own re-layer slice, the same species as 0a. FACE disabledness is unmodeled engine-wide.
   *Slot (NEW)*: **`#11-actually-disabled-option-optgroup-branches`**. *Trigger*: the select-surface pass, or a WPT/site exercising `optgroup:disabled`. *Date*: 2026-11.
2. **Radio-group `:indeterminate`** (§4.16.3 bullet 2 — a radio whose group contains no checked radio).
   *Reason*: needs form-owner resolution + group membership (`resolve_form_owner` / `find_radio_group_scoped` / `AncestorCache`, `crates/dom/elidex-form/src/radio.rs`) moved into the leaf. Those are a **system** (form-owner resolution walks and caches), not a pure derivation, so they fail the very test that justified 0a's DOWN set ("component data + pure derivations down, systems up"). Pre-existing gap — never implemented (INDETERMINATE's only production writer was the clone re-sync).
   *Slot (NEW)*: **`#11-radio-group-indeterminate-pseudo`**. *Trigger*: a radio-group WPT, or the form-owner-resolution re-layer. *Date*: 2026-11.
3. **`<option>` `:checked` keys off the `selected` content attribute, not live selectedness** (§4.10.10 `#concept-option-selectedness`).
   *Reason*: live selectedness is **not a reliable SoT in this engine today** — `<select>.value` / `.selectedIndex` write only the `selected` content attribute and never update `FCS.options`/`selected_index`. Switching `:checked` to "live selectedness" would read a field that is stale by construction: strictly worse than the attribute. The predicate carries an explicit divergence comment naming the unblock.
   *Slot (existing, not new)*: **`#11-select-value-split-sot`** (umbrella §10 item 1).
4. **Selectors-4 `:unchecked` and the "indeterminate wins / mutually exclusive" precedence rule** (§12.2.1).
   *Reason*: Selectors-4-only additions with no HTML §4.16.3 counterpart; adding a pseudo-class name to the matcher is a new *surface*, not part of deleting a cache. Noted so a future reader does not mistake the omission for an oversight.
   *No slot* — belongs to a Selectors-4 conformance pass, and per `feedback_defer-slot-eligibility-audit-at-create` a slot with no repeat-signal and no owning surface is noise. Flag at §10-Q5 for review to overrule.
5. **Editing-host fidelity** (`contenteditable=plaintext-only`, `document.designMode`) inside `EcsDom`'s `is_contenteditable` (`crates/core/elidex-ecs/src/dom/mod.rs:335-352`).
   *Reason*: pre-existing, untouched by Slice 1 (the `:read-write` third bullet routes through the same function it does today). **No slot** — Slice 1 neither widens nor narrows it, and it belongs to the editing surface, not the form-pseudo surface. Flag at §10-Q5.
6. **Constraint-validation candidacy (which elements are candidates at all)** — the §4.10.2 category, the §4.10.6 / §4.10.5.1.x bars, and the §4.10.22.4 step-5.1 button exclusion.
   *Reason*: **carved to PR-A** (`docs/plans/2026-07-form-submittable-category-repair.md`) as a hard prereq — axis-orthogonal to selector matching, and it carries its own slot (`#11-submitter-entry-tree-order`) and its own JS-observable `willValidate` deltas. Slice 1 **consumes** the corrected predicate and adds nothing to it.
   *No slot here* — PR-A owns the surface and its deferrals.

---

## §7 Layering check

- **Algorithm placement** ✓ — all six derivations land in `elidex-form-core` (engine-independent leaf, deps `elidex-plugin`/`elidex-ecs`/`regex`/`tracing`/`url`). Nothing is authored in `crates/script/elidex-js/src/vm/host/`; the VM is untouched by this slice except for tests.
- **New crate edge** ✓ — `elidex-css → elidex-form-core` only. Acyclic: form-core reaches neither `elidex-dom-api` nor `elidex-css` nor `elidex-script-session`. Precedent: `elidex-custom-elements` was added to `crates/css/elidex-css/Cargo.toml` for `:defined` (`matching.rs:226-234`).
- **`matching.rs` stays a matcher** ✓ — after the rewrite it contains selector-shaped logic (element-set gates, complement, `Option` comparison) and zero HTML form algorithm. The `EcsDom::*` calls that remain are marshalling-grade (`TagType`, `Attributes`, `is_contenteditable` for the editing-host bullet).
- **B1-core (`elidex-ecs`) touch** ✓ bounded — 7 constant lines + one doc comment in `components.rs`, and the clone-policy doc row in `dom/tree_clone.rs:28`. No structural `EcsDom` change. No concurrent worktree touches these (verified 2026-07-26: the 5 live worktrees are `terminal-z-c3-plan`, `terminal-z-c3a-impl`, `session-history-slice-b`, `domform-slice1`, `vm-p4-plan`).
- **Shell touch** ✓ minimal — one call deleted in `content/form_input.rs`; `update_element_state` survives for HOVER.

### §7.1 ECS-native check

- **Derived state stops being a component field.** The 7 bits were a *cache of a derivation* living on the entity; ECS-native says a component holds **facts**, and a derivation over facts belongs to a query at the point of use. `FormControlState` (the value model) and `Attributes` (the content attributes) are the facts; the pseudo-class answer is the query. Deleting the bits makes the component set honest.
- **No side-store, no registry, no observer.** The replacement is a free function over `(Entity, &EcsDom)` — the ECS idiom for "derive from components", not an OO accessor or a subscriber list.
- **`ElementState` keeps exactly the facts it owns**: HOVER/FOCUS/ACTIVE/LINK/VISITED are *interaction* state with no other representation — genuine per-entity facts, correctly a component.
- **Component data-flow integrity (Axis 2 sub-check 2b)**: the new read sites are `FormControlState` (written by the 0b-completed reconciler + the IDL setters for attribute-less live state) and `Attributes` (written by the `EcsDom::set_attribute`/`remove_attribute` chokepoint). Both write paths are wired **today** — Slice 1 adds no component whose writer does not exist. The one read whose writer *was* an ad-hoc hand-sync (`ElementState` bits) is exactly what the slice removes.

---

## §8 Test plan

Every §6 row has a test. Layer choice follows the umbrella §8 pattern: predicate unit tests in `elidex-form-core`, pseudo-class semantics in the `elidex-css` selector tests, and the mutate-then-read contracts at VM JS level (only JS can prove I2 — that `matches()` sees a mutation without a restyle).

**A. `elidex-form-core` unit tests** (new `#[cfg(test)] mod tests` in `pseudo.rs`, or `pseudo_tests.rs` if it crosses the file-size seam — §9):
- `is_effectively_disabled`: own attr; nested fieldsets; first-`<legend>` exemption (incl. the "exempt but an outer fieldset still disables" path at `fieldset.rs:66-69`); `<fieldset>` itself as subject; move-in / move-out (**no test exists today** for move-out); non-form tags → false; `<div>` inside `<fieldset disabled>` → false.
- `is_checked` / `is_indeterminate` / `requiredness` / `is_read_write` / `constraint_validity`: each against its own SoT, including the `required`/`readonly` apply-list boundaries and the `<progress>` no-`value` branch.
- **FCS-absent path for all six** (I6): every predicate returns its §4.3 documented absent answer and **never panics**. Explicitly assert `constraint_validity(..) == None` for an FCS-less `<input>` (the umbrella-correction, §6-B).
- `FormControlKind::required_applies`: full variant sweep, asserted **against** `readonly_applies` so the two apply-lists cannot silently converge.

**B. `elidex-css` selector tests** (the 37 rewritten tests, §5): re-seeded per §5's split (attributes for attribute-sourced pseudo-classes, `FormControlState::from_element` for live-state ones). New coverage for each §6-A delta that is observable without JS: rows 6, 7, 8, 9, 10, 12, 13, 14 (§8-U6…U12). Row 9's existing test `valid_matches_output_element` (`crates/css/elidex-css/src/selector/tests/matching.rs:766`) is **inverted with a comment naming §4.10.2 + §4.10.12** so the flip reads as intentional.

**C. VM JS-level regressions** (umbrella §8 — the contracts broken on `main`):
- **V1** `i.checked = true; i.matches(':checked')` → true (currently **false**).
- **V2** `i.indeterminate = true; i.matches(':indeterminate')` → true (currently **false**).
- **V3** `i.disabled = true` → `:disabled` true, `:enabled` false, `:read-write` false, `:read-only` true.
- **V4** `fs.disabled = true` (dynamic) → descendant `input.matches(':disabled')` → true (currently **false**).
- **V5** move a control **out** of `<fieldset disabled>` → `:enabled` again (currently **never re-enables**).
- **V5b** `input.removeAttribute('disabled')` inside `<fieldset disabled>` → still `:disabled` (currently **clobbered to enabled** via `reconciler.rs`'s bare assignment).
- **V6** `i.required = true; i.value = ''` → `:invalid` true, `:valid` false (currently `:invalid` is **impossible to match**).
- **V7** `i.setAttribute('required','')` → `:required` true, `:optional` false.
- **V8** entry list **excludes** a dynamically fieldset-disabled control (§4.10.22.4 — M1's proof).
- **U9b** (non-regression, §6-A row 10a) `<button type=submit>:valid` and `<input type=submit>:valid` stay **true** across the bit→predicate switch — the assertion that proves the PR-A → Slice-1 handoff is sound. (PR-A separately asserts their `willValidate`; this one is the CSS-side counterpart.)
- **U14** a control moved out of `<fieldset disabled>` has `willValidate` → true (D3b).
- **C1** the existing clone assertions stay green **without** the W6 re-sync: `crates/script/elidex-js/src/vm/tests/tests_clone_form_state.rs:100` (`String(c.indeterminate) + ',' + String(c.matches(':indeterminate'))`) is the load-bearing one — it is the #466 contract, and it passing *because the derivation is live* rather than because a hand re-sync ran is the cleanest possible proof that W6 was redundant.
- **Reset** `form.reset()` → `:checked` follows `defaultChecked` (0b supplied the `default_checked` reconciliation; full dirty-checkedness interaction stays Slice 4).

**D. Non-regression guards**: `:hover`/`:focus`/`:active`/`:link`/`:visited` unaffected (`elidex-style` `set_link_state` + focus + hover paths untouched); `<div>:read-only` still true (§6-B's most easily broken invariant — D4 exists to protect it).

**Gate**: `cargo test -p elidex-form-core -p elidex-form -p elidex-css -p elidex-js -p elidex-shell --all-features`, then `mise run ci`, then `/elidex-review` + `/code-review`, then `/external-converge` (edge-dense per `feedback_gate-miss-on-edge-dense-escalate-to-converge` — plan-review is mandatory here, so converge from round 1).

---

## §9 File / line-count budget + the 1000-line discipline

Counts are `wc -l` at `f7d9b5ce`, **verified 2026-07-26**.

| File | Now | After (est.) | Touch-time verdict |
|---|---|---|---|
| `crates/core/elidex-ecs/src/components.rs` | **1037** (already >1000) | ~1029 | **Net deletion — no split needed.** The discipline targets *growth into* unbounded files; Slice 1 removes 7 constant lines and edits one doc comment. `components.rs` is also a **flat component-declaration table** — the discipline's explicitly non-splittable shape (no real cohesion seam; every item is an independent `pub struct`/`impl`). Splitting it would be a speculative refactor bundled into a feature PR, which the same discipline forbids. Recorded here so plan-review does not read the >1000 count as an unaddressed debt. |
| `crates/css/elidex-css/src/selector/matching.rs` | 671 | ~600–650 | Production shrinks (5 helpers + 1 fetch deleted, arms simplified); tests roughly flat (re-seeded, not multiplied). Comfortably bounded. |
| `crates/dom/elidex-form-core/src/lib.rs` | **960** | ~978 | `required_applies` (~18 lines incl. doc) is the only addition — the predicates go to `pseudo.rs` precisely to avoid pushing this file over. Still <1000, so no obligation. **Contingency**: if the actual delta would cross 1000, carve `FormControlKind` + its inherent impl (`lib.rs:89-326`) into `kind.rs` as a **standalone prereq commit before** the feature work (the seam 0b §9 already named), never bundled. |
| `crates/dom/elidex-form-core/src/pseudo.rs` (NEW) | — | ~200 production + ~250 tests | If tests push it past 1000, split to `pseudo_tests.rs` via the crate's existing `#[cfg(test)] #[path = "…"] mod tests;` idiom (`lib_tests.rs` / `input_tests.rs` precedent) — decide at impl from the real count, not speculatively. |
| `crates/dom/elidex-form/src/init.rs` / `fieldset.rs` / `radio.rs` / `clone.rs` / `submit.rs` | 295 / 189 / 500 / 304 / 546 | all shrink | net deletion |
| `crates/shell/elidex-shell/src/content/form_input.rs` | 318 | ~314 | net deletion |
| `crates/css/elidex-css/src/selector/tests/matching.rs` | 876 | ~900 | re-seeding, roughly flat; watch the 1000 line |

**Cargo**: `crates/css/elidex-css/Cargo.toml` gains `elidex-form-core.workspace = true` (current deps: `elidex-plugin`, `elidex-ecs`, `cssparser`, `elidex-custom-elements`).

---

## §10 Decisions + open questions for `/elidex-plan-review`

**How to read this section.** Q1 is the headline **decision**. Q1a/Q2/Q3/Q4/Q5/Q6/Q7 are **PM-decided, open for review challenge** — they are stated as settled design with the rejected alternative named, so the review scrutinizes a committed design rather than picking from a menu. Overturning any of them is a legitimate review outcome; leaving one unexamined is not.

- **Q1 (HEADLINE) — D3: delete VALID/INVALID in Slice 1 and wire the `:valid`/`:invalid` ELEMENT bullet now?**
  **Decision: yes.** Rationale: keeping 2 of 7 bits keeps `apply_element_state_flags`, the init-time `ElementState` insert, `form_element_state()`, and `state_flag_for_pseudo`'s bit arms alive **for two pseudo-classes** — the "new seam + N legacy impls" strangler One-issue-one-way explicitly forbids, in the very PR whose purpose is to collapse the decision surface. The umbrella's own §1.2 names "`:valid` matches every form control forever; `:invalid` can never match" as a **live cache-caused bug**, so a slice that deletes the cache and preserves that bug is self-contradictory. The pure functions already exist in the leaf (`validate_control` `validation/mod.rs:104`, `is_constraint_validation_candidate` `:21-44`), so wiring is a call, not an algorithm.
  **Rejected (i)**: keep the 2 bits until Slice 3 → the strangler state above.
  **Rejected (ii)**: delete the bits but give `:valid`/`:invalid` an interim derivation that reproduces today's answers (always-valid) → ships a knowingly-wrong canonical predicate when the correct pure functions are already sitting one call away = "TODO 先送り禁止" in its purest form.
  **Re-scopes umbrella Slice 3** to "the `<form>`/`<fieldset>` **aggregate** bullets + the O3 perf decision"; the Slice-3 row and the §5 coverage-map row in the umbrella are edited by this PR (§12).

- **Q1a — RESOLVED by carve, not by decision inside this slice.** The draft of this memo proposed fixing the constraint-validation candidacy basis here (`FormControlKind`'s `is_submittable` is not the §4.10.2 category, so wiring `:valid`/`:invalid` on it would regress `<button type=submit>` true→false). That repair is now **PR-A**, a hard prereq: `docs/plans/2026-07-form-submittable-category-repair.md`. Trigger for the carve was **axis-orthogonality**, not size — it drags in §4.10.2 / §4.10.6 / §4.10.5.1.18/20/21 / §4.10.22.4 step 5.1, the submitter-append factoring, entry ordering, image buttons, and a JS-observable `willValidate` flip, none of which is selector matching; the corroborating signal was that it generated a third defer slot and put this keystone at the ≤3 cap with zero headroom.
  **What Slice 1 keeps**: it *consumes* `is_constraint_validation_candidate` as-is and asserts the handoff (§6-A rows 10 / 10a, §8-U9b). **Review should challenge**: whether Slice 1 truly adds nothing to the candidacy surface — if any reviewer finds a candidacy branch this slice still has to touch, the carve boundary is wrong.

- **Q2 — D3b: pull the `validation/mod.rs` disabledness collapse forward from umbrella Slice 2. PM-DECIDED; open for review challenge.**
  `is_constraint_validation_candidate` checks `state.disabled` (`:29`) *and* `!is_fieldset_disabled(..)` (`:43`). `FCS.disabled` carries the init-time fieldset fold-in (`init.rs:89-90`) and is never re-derived on move, so a control initialised inside `<fieldset disabled>` and then moved out stays barred. After Slice 1 that control reads `:enabled` (live pull) while `:valid`/`:invalid` report "no validity semantics" — **two pseudo-classes in the same PR disagreeing about the same fact**.
  **Decision**: collapse both lines onto `is_effectively_disabled(entity, dom)` here, by the *identical* argument the umbrella used to bundle `submit.rs:353` ("must flip *with* `:disabled`"). One line, intra-crate. Umbrella Slice 2 narrows to `radio.rs:64` + `init.rs:30/56/89`.
  **Rejected**: leave it and record the incoherence as a named transient — the umbrella's transient-incoherence budget was spent on *cross-lane* consumers (focus/shell), not on two CSS pseudo-classes in one file.

- **Q3 — O3 (umbrella) perf. PM-DECIDED (bench home + gate framing); open for review challenge.**
  **Bounding argument**: (a) the predicate runs **only** when a selector names `:valid`/`:invalid` — `match_pseudo_class` (`matching.rs:190-237`) dispatches by name, so pages without those selectors pay zero; (b) the `pattern` regex is **matched, not compiled** — `FCS.cached_pattern_regex` holds a precompiled `Arc<Regex>` (`lib.rs:348-432`), maintained by the FCS `update_pattern` method (`lib.rs:487`); (c) `ValidityState::default()` allocates nothing (`custom_error_message` is only assigned when a custom message exists, `validation/mod.rs:196-199`).
  **Bench home**: extend `crates/css/elidex-style/benches/style_resolve.rs` (100 lines) — it already drives selector matching end-to-end. A targeted `elidex-css` microbench would measure the predicate rather than the question, and `crates/css/elidex-css/benches/css_parse.rs` is parse-only.
  **Hard gate (non-regression)**: the **existing** `style_resolve` fixtures — which contain **no** form pseudo-classes — must not regress **>10%**. That is the real risk (a common path made slower for nothing) and it is a fair before/after comparison of the same work.
  **Informational (not a gate)**: a **new** form-heavy fixture (N inputs × a sheet with `:valid`/`:invalid`/`:disabled`/`:checked`), before/after recorded in the PR body. It is *not* a gate because it compares the cost of today's **wrong** answer (a bit test) against tomorrow's **correct** one (a constraint evaluation) — those are different computations, so no threshold over them can decide anything. Its purpose is to be the **input to Slice 3's** cached-vs-derived decision.
  **Explicit Slice-3 trigger**: if the form-heavy fixture shows the `:valid`/`:invalid` path costing **more than the whole rest of selector matching on that fixture** (i.e. validity dominates rather than contributes), Slice 3 must design a derived-and-cached validity with a **single invalidation owner** sitting *behind* `constraint_validity` — never a second answer beside it.
  **Slice 1 lands either way.** The alternative to the derivation is not a cheaper right answer; it is a permanently wrong one (`:invalid` unmatched forever). Perf can motivate a cache *behind* the canonical predicate; it cannot motivate keeping the cache *instead of* it.

- **Q4 — D4 correction to umbrella §3.1 (`:read-only`). PM-DECIDED; open for review challenge.** The umbrella sketches `is_read_only` as "`readonly` attr OR disabled", which would make a plain `<div>` **not** `:read-only` — breaking behavior that is **correct today** (`matching.rs:305-311`'s non-form fallback) and required by §4.16.3 ("The `:read-only` pseudo-class must match all other HTML elements"). Slice 1 authors `is_read_write` as the only predicate and derives `:read-only` as its exact complement. The umbrella §3.1/§5 rows are edited accordingly by this PR (§12).

- **Q5 — the two no-slot boundaries. PM-DECIDED (no slot); open for review challenge.** §6-D items 4 (Selectors-4 `:unchecked` + precedence) and 5 (editing-host fidelity) are recorded **without** a `#11-*` slot, per `feedback_defer-slot-eligibility-audit-at-create` (no repeat-signal, no owning surface, and item 5 is untouched by this slice). Review may overrule by naming owners.

- **Q6 — `<progress>:indeterminate` (§6-A row 8) is in scope. PM-DECIDED; open for review challenge.** It is a one-line pure attribute read (`!attrs.contains("value")`) inside a predicate the slice authors anyway, so excluding it would mean deliberately writing an incomplete canonical predicate (D2). Review may reclassify it as a §6-D boundary.

- **Q7 — `is_enabled_scope` placement. PM-DECIDED (stays in `matching.rs`); open for review challenge.** Element-set gates stay in `matching.rs` (matcher data, one consumer) while the HTML concepts go down (§4.4). Confirm this split, or move the scope lists into `pseudo.rs` too. Decision: keep them up — they are Selectors-side data, and moving them down would put "which elements does `:enabled` range over" in a crate that has no notion of selectors.

---

## §11 Defer slots + per-PR ≤3 audit

| Slot | Status | 4-question audit (`feedback_defer-slot-eligibility-audit-at-create`) |
|---|---|---|
| **`#11-actually-disabled-option-optgroup-branches`** | **NEW** | (1) Real platform gap? yes — §4.15 bullets 5/6/8 unimplemented. (2) Blocked by structure? yes — `is_option_disabled` is above `elidex-css`; needs a re-layer slice. (3) Non-regressing to defer? yes — pre-existing, untouched. (4) Durable home? `project_open-defer-slots.md` L3 cluster at ship. |
| **`#11-radio-group-indeterminate-pseudo`** | **NEW** | (1) yes — §4.16.3 `:indeterminate` bullet 2. (2) yes — needs form-owner/group *systems* in the leaf. (3) yes — never implemented. (4) same. |
| `#11-select-value-split-sot` | **pre-existing** (umbrella §10 item 1) | referenced as the `<option>` `:checked` unblock; not a new slot. |
| `#11-form-core-facade-shrink` | **pre-existing** (0a §6) | **triggered by this PR** — Slice 1's first `elidex_form_core` import in `elidex-css` is its declared trigger. Disposition proposed: **keep the facade for this slice** — the 5 consumers 0a §3-I3 enumerated (`elidex-render`, `elidex-shell`, `elidex-layout-block`, `elidex-js`, `elidex-a11y`; verified 2026-07-26) still import `elidex_form::X` — and record that the dual path is now live. Review may instead demand the shrink here; I recommend not, because it would bundle a 5-crate import sweep into the keystone. |
| `#11-input-dirty-checkedness-flag` | pre-existing (umbrella Slice 4) | untouched. |
| `#11-fieldset-disabled-dynamic-insert` | pre-existing | **dissolves** with Slice 1 (umbrella §11): with pull derivation there is no propagation to schedule for the CSS-visible half. Close at ship *only if* review agrees the remaining `FCS.disabled` half is Slice 5's. |
| `#11-input-live-pseudo-state-elementstate-reconciliation` | pre-existing | **CLOSED by this slice** (the umbrella's Slice-1 target). |

**Per-PR budget**: **2 new slots** — `#11-actually-disabled-option-optgroup-branches`, `#11-radio-group-indeterminate-pseudo` — within the ≤3 cap **with one slot of headroom**, restored by carving PR-A (which now carries `#11-submitter-entry-tree-order` in its own budget). `#11-select-value-split-sot` is *referenced*, not created. The headroom matters: if review overrules Q5's two no-slot boundaries, one can be absorbed without breaching the cap. Registration target = `project_open-defer-slots.md` (L3 cluster) **at ship**, per `reference_spawn-task-chips-not-durable` — a committed plan note alone ages out.

---

## §12 Coordination + the umbrella doc-edits this PR carries

**Worktrees / branches, verified 2026-07-26** (`git worktree list`; `git diff --stat origin/main...origin/vm-input-value-as-date`):

| Lane | Actual state | Overlap with Slice 1 | Ordering rule |
|---|---|---|---|
| **PR-A** (`docs/plans/2026-07-form-submittable-category-repair.md`, same worktree) | plan-memo, not implemented | Both touch `crates/dom/elidex-form-core/src/lib.rs` (PR-A: `is_submittable`; Slice 1: `required_applies`) and `validation/mod.rs` (PR-A: the basis at `:26` + bars at `:32`; Slice 1: the disabledness collapse at `:29-31`/`:43`). Adjacent lines in the same two functions. | **PR-A lands first — hard prereq** (§0). Slice 1 rebases on it; expect ~5-line offsets in `validation/mod.rs` citations. |
| `vm-input-value-as-date` (`#11-input-value-as-date`, L3) | **plan-only**: a single commit `75a0cebc` adding `docs/plans/2026-07-input-value-as-date.md` (230 lines). It touches **no** `.rs` file. | **none today.** Its *future* implementation will touch `crates/dom/elidex-form-core/src/input.rs` + `crates/dom/elidex-form-core/src/datetime.rs`; Slice 1 touches neither (it adds `pseudo.rs` and ~18 lines to `lib.rs`). | No ordering constraint. Whichever implements second rebases. **This corrects the scouted premise** that the branch currently collides on `input.rs`/`datetime.rs` — see §13. |
| L1 CSS-style lane (`#11-style-shorthand-expand`) | No active worktree. The shorthand work lives under `elidex-css`'s `src/declaration/` directory (`box_model.rs`, `flex.rs`, `font.rs`, `grid.rs`, `grid_shorthand.rs`, `misc.rs`, `cssom.rs`) — **there is no `shorthand.rs` at the `elidex-css` crate root** (verified 2026-07-26 via `ls crates/css/elidex-css/src/`). | Same **crate**, disjoint **modules**. Slice 1 touches `crates/css/elidex-css/Cargo.toml` + the `src/selector/` subtree. | Only `Cargo.toml` can conflict, and only as a one-line dependency addition. Rebase-resolvable; no logical ordering. |
| Layout lane (`terminal-z-c3a-impl`) | 7 unpushed commits in `/Users/kazuaki/repos/send.sh/elidex-wt-c3a-impl`; touches `elidex-ecs` `dom/geometry.rs` + layout readers. | `elidex-ecs`, different files (`components.rs` vs `dom/`). | None. |
| B1-core | `crates/core/elidex-ecs/src/dom/tree_clone.rs:28` was just retagged by #484 (`f7d9b5ce`). Slice 1 must edit that row again (W6). | direct | Slice 1 is on top of #484, so it edits the **current** wording — re-read the row at impl rather than applying a pre-written patch. |

**Umbrella doc-edits carried in this PR** (the umbrella is committed on `main`, so "fix at umbrella landing" is foreclosed — same precedent as 0b §11):
1. **§3.1** — `is_read_only(entity, dom)` → `is_read_write(entity, dom)` with the complement note (D4/Q4).
2. **§3.1** — the `validity(...)` absent-case "absent→valid" → "absent → `None`, matches neither" (§4.3).
3. **I6 / §3.1 / §8** — "flips five pseudo-classes" → **four**, with `:valid`'s no-flip reason (§6-B).
4. **§6 Slice 1 row** — writer list corrected from four sites to **six** (W2 `init.rs:89-94` and W3 `fieldset.rs:60-68` were missing; W6 was attributed to `elidex-ecs` but lives in `crates/dom/elidex-form/src/clone.rs`), and `clone.rs:147-149` → `clone.rs:146-151`.
5. **§6 Slice 3 row + §5 coverage row** — re-scoped to the `<form>`/`<fieldset>` **aggregate** bullets + the O3 decision (D3/Q1).
6. **§6 Slice 2 row** — narrowed to `radio.rs:64` + `init.rs:30/56/89` (Q2 decided).
7. **I1** — the `matches()` chain path `element/child_node/selectors.rs:38` → `crates/dom/elidex-dom-api/src/child_node/selectors.rs` (no `element/` segment).

(The umbrella §6 **prereq row for PR-A** is carried by **PR-A's own PR**, not this one — see that memo's §11.)
8. **§4.1 audit table, `submit.rs:353` row** — the row stays as the disabledness swap. (An earlier draft paired it with a button exclusion; that half is now PR-A's.)

---

## §13 Where the scouted ground truth was wrong (recorded so the corrections are auditable)

1. **The writer list is six sites — and the sixth had a test too.** `crates/dom/elidex-form/src/fieldset.rs:98-102` (`fieldset_disables_descendants`) asserts `ElementState::DISABLED` and must be edited with W3. Likewise `crates/dom/elidex-form/src/clone.rs:224-229` / `:251-255` for W6.
2. **`origin/vm-input-value-as-date` is plan-only** (one commit, one `.md`). It does not touch `form-core/input.rs` or `datetime.rs` today, so there is no present ordering constraint (§12).
3. **There is no `shorthand.rs` at the `elidex-css` crate root.** The L1 shorthand surface is `elidex-css`'s `src/declaration/` subtree (§12).
4. **`:valid` does not flip f→t on the component-absent path** — the umbrella's five-flip claim is four (§6-B).
5. **`is_disableable_element`'s premise is wrong, not just narrow.** Its comment asserts `<fieldset>` "does NOT participate in the `:enabled`/`:disabled` pseudo-classes"; §4.16.3's `:enabled` scope list names `fieldset` explicitly and §4.15 lists disabled fieldsets among "actually disabled" elements (§6-A rows 6-7).
6. **`<progress>:indeterminate` is a live spec branch** the scouted decision set omitted (§6-A row 8, Q6).
7. **Reusing `is_constraint_validation_candidate` verbatim would regress `<button type=submit>:valid`** — `FormControlKind`'s `is_submittable()` is not the §4.10.2 category despite its doc citation (Q1a).
8. **HTML §6.8.4's title is "Editing APIs"**, not "editing host"; `#editing-host` is a dfn *inside* it (§0.5).
9. **§4.10.18.7 does not exist** — `matching.rs:256` cites a section that `webref heading --exact html 4.10.18.7` cannot resolve, and the claim it supports (`<output>` is always `:valid`) is itself wrong (§6-C, §6-A row 9).

**Round 3 — the D3a carve.**

15. **D3a is no longer part of this slice.** The constraint-validation candidacy repair — §4.10.2 category, §4.10.6 / §4.10.5.1.18/20/21 bars, §4.10.22.4 step-5.1 button exclusion, the submitter-append factoring, entry ordering, image buttons, and the JS-observable `willValidate` flip — moved to **PR-A** (`docs/plans/2026-07-form-submittable-category-repair.md`), a hard prereq. **Trigger = axis-orthogonality, not the preflight M count**: none of that cluster is selector matching, and it stands alone as a correctness improvement with no dual mechanism in the PR-A → Slice-1 window (the same carve test 0a and 0b passed). The corroborating signal was the third defer slot (`#11-submitter-entry-tree-order`), which had put this keystone at the ≤3 cap with zero headroom. M dropped 44 → 37 and the verdict is **still SPLIT-DEFAULT** — §3.1's appeal, not the count, is what carries this PR (§3).

**Round 2 — corrections to the PM dispositions themselves** (both items the PM explicitly said not to take on trust):

10. **The button input-state section numbers are NOT 4.10.5.1.19/20/21.** Looked up 2026-07-26 (`heading --exact html 4.10.5.1.<n>`): **§4.10.5.1.18 = Submit Button state (type=submit)**, **§4.10.5.1.19 = Image Button state (type=image)**, **§4.10.5.1.20 = Reset Button state (type=reset)**, **§4.10.5.1.21 = Button state (type=button)**. A "19/20/21 = submit/reset/button" reading is off-by-one and lands on Image Button. Also: the existing `<input type=hidden>` bar is **§4.10.5.1.1 Hidden state (type=hidden)**.
11. **`<input type=submit>` is NOT barred from constraint validation, so no `TagType` read is needed.** `body html 'submit-button-state-(type=submit)' | grep -i 'barred from constraint'` returns **no match** — §4.10.5.1.18 carries only a `formnovalidate` note (about *triggering* validation at submission, not about barring). Same for §4.10.5.1.19 Image Button. Only §4.10.5.1.20 / §4.10.5.1.21 carry "Constraint validation: The element is barred from constraint validation." Consequently `FormControlKind::SubmitButton` — which covers `<button type=submit>` **and** `<input type=submit>` — is a candidate in **both** tags, and `ResetButton`/`Button` are barred in **both**. The instruction's premise ("`FormControlKind::SubmitButton` cannot distinguish … so the fn uses its existing `entity`+`dom` to read `TagType`") is therefore moot: the bar is a two-variant kind check (now **PR-A §4.3**). This also corrects **my own** Q1a draft, which asserted the same false disambiguation need.
12. **`<input type=submit>:valid` is a NON-delta, not a delta.** §6-A row 10 originally lumped `<input type=submit>` in with reset/button as "barred ⇒ flips true→false". Under the verified barring set it stays **true** both before and after — split out as row 10a with a dedicated non-regression assertion (§8-U9b), because it is the assertion that proves the repair (now **PR-A**) fixed the basis rather than merely widening it.
13. **`submit_tests.rs:446` is not a caller.** The PM's `grep -rn is_submittable` hit list is right on substance; the 14-hit grep also matches `fn hidden_input_is_submittable()` at `crates/dom/elidex-form/src/submit_tests.rs:446`, which is a **test-function name**, not a call to the method (now **PR-A §4.1**).
14. **D3a part 2 surfaces a real, previously-unnamed spec divergence.** §4.10.22.4 step 5.1 keeps the *submitter* button's entry in **tree order**; elidex filters all buttons out and appends the submitter at `crates/dom/elidex-form/src/submit.rs:157-172`. Membership is identical (so parts 1+2 are behavior-preserving, as the PM predicted), but **ordering** is not. Newly slotted `#11-submitter-entry-tree-order` — which at the time put this PR **at the ≤3 defer cap** with no headroom, one of the signals that triggered the carve (item 15). The slot now lives in **PR-A §10**.
