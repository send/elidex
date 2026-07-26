# Plan — repair `is_submittable()` to be the real §4.10.2 "submittable element" category (form derived-state prereq PR-A)

## §0 Status

**Parent**: `docs/plans/2026-07-form-derived-state-reconciliation.md` (umbrella). **Not** one of the umbrella's numbered slices — a **prereq carved out of Slice 1** (§0.1).
**Blocks**: `docs/plans/2026-07-form-elementstate-cache-deletion-slice-1.md` (the keystone), which lives in the sibling worktree `/Users/kazuaki/repos/send.sh/elidex-wt-slice1` on branch `domform-slice1`.
**Lane**: DOM/form (L3). **Branch**: `domform-submittable-category`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-submittable`.
**Base**: `f7d9b5ce` (`origin/main`).
**HARD PREREQ**: **PR-A0** — `docs/plans/2026-07-constraint-validation-citation-sweep.md` (doc-only citation sweep, same worktree/branch, lands FIRST). PR-A's parts 3 and 4 land comments on the numbers PR-A0 corrects; without it PR-A would author new prose onto stale citations.
**Status**: plan-memo, **`/elidex-plan-review` R1 applied** (0 CRIT / 10 IMP / 22 MIN / 2 FP — all 10 IMPs fixed; no accept-as-is tier). Re-review before implementation.
**Nature**: a **spec-fidelity repair of one shared predicate**, plus the citation sweep and the datalist bar that belong to the same function and the same spec cluster.
- **Entry-list construction is NOT preserved** — the datalist bar (§4.10.22.4 step 5.1, first bullet) removes controls inside `<datalist>` from the entry list. That is the only change; every other control's entry is unaffected (§4.3).
- **A form-level algorithm moves down** — §4.10.21.2 steps 1-4 are hoisted into `elidex-form` (§4.5), fixing two live bugs in the process.
- **Submission *outcome* is NOT preserved**: after parts 1+3, `requestSubmit()` newly aborts when a submit button carries a custom validity error (§5.1 row 6). The earlier draft's blanket "behavior-preserving on form submission" was wrong and is corrected here.

### §0.1 Why this is its own PR

Slice 1 is a selector-matching PR: delete the `ElementState` form-bit cache, derive the pseudo-classes at `matching.rs`. This repair is **axis-orthogonal** — §4.10.2 categories, the §4.10.6 / §4.10.5.1.x / §4.10.8 bars, §4.10.22.4 step 5.1, the submitter-append factoring, entry ordering, image buttons, and a JS-observable `willValidate` / `requestSubmit` flip. None of it is CSS.

It passes the same carve test 0a and 0b passed: **self-contained, correctness-improving on its own, no dead code or dual mechanism in the PR-A → Slice-1 window.** §7 applies that same test again and carves *out* the form-level-validation hoist rather than bundling it.

---

## §0.5 Spec citation table

Every § ↔ title pair looked up with `.claude/tools/webref` on **2026-07-26**. Nothing is quoted from memory — three numbers a scout (and the R1 brief) would guess are wrong; see ⚠.

| Cite | § | Exact title | Anchor | webref command → result |
|---|---|---|---|---|
| submittable elements (the category `is_submittable` must become) | HTML §4.10.2 | Categories | `#category-submit` | `dfn html 'submittable element'` |
| **"submit button"** — the concept itself | HTML §4.10.2 | Categories | `#concept-submit-button` | `dfn html 'submit button'` → the *dfn* is in §4.10.2; §4.10.6 holds the button `type` **state** |
| the `<button>` submit-button rule + its bar | HTML §4.10.6 | The button element | `#attr-button-type-submit-state` | `body html attr-button-type-submit-state` (quoted in full, §4.4) |
| candidate for constraint validation = submittable ∧ ¬barred | HTML §4.10.21.1 | Definitions | `#candidate-for-constraint-validation` | `dfn html 'candidate for constraint validation'` |
| **datalist-ancestor bar** | HTML §4.10.8 | The datalist element | `#the-datalist-element` | `body html the-datalist-element` → "Constraint validation: If an element has a datalist element ancestor, it is barred from constraint validation." |
| `<input type=submit>` — **NOT barred** | HTML §4.10.5.1.18 | Submit Button state (type=submit) | `#submit-button-state-(type=submit)` | `body html '…' \| grep -i 'barred from constraint'` → **no match** (only a `formnovalidate` note) |
| `<input type=image>` — **NOT barred either** | HTML §4.10.5.1.19 | Image Button state (type=image) | `#image-button-state-(type=image)` | same grep → **no match** |
| `<input type=reset>` is barred | HTML §4.10.5.1.20 | Reset Button state (type=reset) | `#reset-button-state-(type=reset)` | `body html '…'` → "Constraint validation: The element is barred from constraint validation." |
| `<input type=button>` is barred | HTML §4.10.5.1.21 | Button state (type=button) | `#button-state-(type=button)` | same statement |
| `<input type=hidden>` is barred | HTML §4.10.5.1.1 | Hidden state (type=hidden) | `#hidden-state-(type=hidden)` | `body html '…'` → "…it is barred from constraint validation." |
| entry-list step 5.1 | HTML §4.10.22.4 | Constructing the entry list | `#constructing-form-data-set` | `body html constructing-form-data-set` (quoted, §4.3) |
| **control-level** CVA: `willValidate` / `checkValidity()` / `reportValidity()` / `validationMessage` / `setCustomValidity()` / `ValidityState` | HTML §4.10.21.3 | The constraint validation API | `#dom-cva-willvalidate` etc. | `dfn html 'willValidate'` / `'checkValidity'` / `'validationMessage'` / `'ValidityState'` — all resolve here |
| **form-level** `checkValidity()` / `reportValidity()` / `requestSubmit()` | HTML §4.10.3 | The form element | `#dom-form-checkvalidity` / `#dom-form-reportvalidity` / `#dom-form-requestsubmit` | `dfn html 'checkValidity'` / `'requestSubmit'` → the `for=HTMLFormElement` overloads live here |
| association of controls and forms (the `form=` IDREF walk) | HTML §4.10.18.3 | Association of controls and forms | `#association-of-controls-and-forms` | `heading --exact html 4.10.18.3` |
| `minlength` (input) | HTML §4.10.5.3.1 | The maxlength and minlength attributes | `#attr-input-minlength` | `dfn html 'minlength'` |
| textarea does not support `pattern` | HTML §4.10.11 | The textarea element | `#the-textarea-element` | `heading --exact html 4.10.11` (the renumber-sweep memo's queued target) |

⚠ **Three number traps, all verified rather than assumed:**
1. **The button input-states are not consecutive**: §4.10.5.1.18 = Submit Button, **§4.10.5.1.19 = Image Button**, §4.10.5.1.20 = Reset Button, §4.10.5.1.21 = Button.
2. **"Submit button" is defined in §4.10.2**, not §4.10.6. §4.10.6 defines the `type` *state* and states the bar.
3. **§4.10.20.x, §4.10.14.2, §4.10.15.2, §4.10.15.3, §4.10.7.3.1 and §4.10.21.4 do not resolve to what the tree claims** — `heading --exact html <n>` returns *no match* for the first five, and §4.10.21.4 is **"Security"**, not `requestSubmit`. That is **PR-A0**'s sweep, not PR-A's.

---

## §1 Ideal anchor — one category, each consumer applying its own rule

§4.10.2 defines **one** "submittable elements" category (button, input, select, textarea, form-associated custom element). Each consumer then applies its own additional rule: §4.10.22.4 step 5.1 for the entry list; §4.10.21.1 + the per-element "Constraint validation:" statements for validation.

`FormControlKind`'s `is_submittable` (`crates/dom/elidex-form-core/src/lib.rs:200-224`) documents itself as "submittable element — HTML §4.10.2 Categories" but its set is **not** that category: it omits `SubmitButton` / `ResetButton` / `Button`. It is really the *unconditional entry-list contributor* set — one consumer's rule baked into the shared predicate, then reused by a second consumer that needs a different rule.

**What PR-A actually restores — stated narrowly.** The spec's factoring is restored **on the validation consumer**: `is_constraint_validation_candidate` goes from "reads a mis-set predicate" to "reads the category, then applies its own §4.10.6 / §4.10.5.1.x / §4.10.8 bars." On the **submission** consumer it is *not* fully restored: elidex excludes **all** buttons in `collect_control_entry` and re-adds the submitter in a separate append pass (§4.3), which equals step 5.1's membership only in combination — not because it implements step 5.1. The new comment must say that, and the ordering divergence it implies is slotted (§10).

---

## §2 Coupled invariants

**Edge-density judgment.** PR-A has no algorithmic novelty — the spec states every branch — but it qualifies on the shape that reliably leaks in a prose-only plan: **widening a shared predicate forces every consumer's compensating rule to be re-derived simultaneously, in opposite directions.** R1 added a second such shape (I5): the same widening reaches a *third* consumer nobody enumerated. §2 is short by design, not padded.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Category integrity** — after part 1, `is_submittable` means exactly §4.10.2. | No consumer may lean on a second meaning. Doc rewritten; both tests that encode the old set inverted, not "made green". |
| **I2** | **Entry-list change is exactly the datalist bar** — the button exclusion must be a no-op; the datalist exclusion must be the only delta. | Part 2's button exclusion must cover exactly the three kinds part 1 added (no-op), while its datalist exclusion is a deliberate §4.10.22.4 step-5.1 fix. Both must be reasoned at the **`collect_control_entry`** level (consumer-independent), because several call paths depend on it (I5). |
| **I3** | **Candidacy changes only in the spec-prescribed direction** — `SubmitButton` becomes a candidate; `ResetButton`/`Button` do not; datalist descendants become barred. | Bars derived from each element/state's "Constraint validation:" statement, never guessed from a kind's name. §4.4's table is the derivation. |
| **I4** | **Fixture truth** — two tests currently *assert the wrong set* (`lib_tests.rs:415-417`, `validation/tests.rs:736-745`). | They must be inverted **with the citation that makes the new answer checkable**, and the second one's *comment* (which states the false premise in prose) rewritten. |
| **I5** | **Three consumers, one of which gates submission** — `collect_control_entry` feeds `build_form_submission`, `vm/host/form_data.rs:255`, and (via the former) shell `content/form_input.rs:151`; separately, candidacy gates `native_form_request_submit`'s abort. | Entry-list preservation must hold for all three, **and** the candidacy change has an observable submission consequence that entry-list reasoning cannot see (§5.1 row 6). This is the invariant R1 found missing. |

**Pairwise intersections (all 10):**

| Pair | Intersection |
|---|---|
| I1 × I2 | Widening the category *removes* submission's free exclusion — so the exclusion I2 needs must be restated in the same commit, or entry lists change. |
| I1 × I3 | The same widening *exposes* validation to three new kinds — so the bars must be added in the same commit. **Opposite consequences of one edit**: why parts 1/2/3 cannot be split across PRs. |
| I1 × I4 | The tests encode the *old* category, so I1's redefinition is only real once they are inverted; a green suite against the old assertions would mean part 1 did not land. |
| I1 × I5 | A predicate consumed by three paths cannot be re-defined by inspecting one; the enumeration is a precondition for I1, not a courtesy. |
| I2 × I3 | Submission and validation want opposite treatment of the same three kinds. Keeping both correct is only possible once the shared predicate stops answering both questions — **this pair is the argument for repairing the definition rather than shimming one consumer**. |
| I2 × I4 | I2's claim is unfalsifiable without a fixture containing a submitter *and* other named controls; §6-S1 exists to make it checkable rather than asserted. |
| I2 × I5 | Preservation must be proved where the filter lives (`collect_control_entry`), not at one caller — otherwise the `new FormData(form)` path (which never appends a submitter) is silently out of scope. |
| I3 × I4 | `validation/tests.rs:736-745` asserts a default `<button>` is not a candidate *and* states the wrong reason ("Button kinds are not submittable"); inverting the assertion alone leaves the false premise for the next reader. |
| I3 × I5 | Candidacy is not only a getter: `native_form_request_submit` (`html_form_proto.rs:546-556`) **aborts submission** on validation failure, so a candidacy change is a submission change. The delta table must carry it. |
| I4 × I5 | §5.4's test audit must be derived **by consumer path**, not by API-name grep — a name grep structurally cannot reach `tests_form_submit.rs`, which puts a real `<button>` in a form and calls `requestSubmit`. |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.2 Categories | submittable elements | button, input, select, textarea | **part 1** — `is_submittable` (`crates/dom/elidex-form-core/src/lib.rs:200-224`) becomes this category | ✓ for the four modelled tags | no |
| WHATWG HTML §4.10.2 Categories | submittable elements | form-associated custom elements | **not modelled** — no `FormControlKind` variant exists | ✗ — known incompleteness, **no slot** per §9-Q5 (FACE is a whole surface, not a gap in this predicate) | no |
| WHATWG HTML §4.10.2 Categories | "submit button" concept — **anchor** `#concept-submit-button`; the disjunction's **prose** is stated in §4.10.6 | disjunction: Auto state ∧ no `command`/`commandfor` ∧ parent not `<select>` — **or** Submit Button state | consumed by part 3's bar and by `is_submit_button` (`crates/dom/elidex-form/src/submit.rs:283`); §4.4 states the modelling premise | ✓ under the stated premise | yes — `button.type=` |
| WHATWG HTML §4.10.22.4 Constructing the entry list | step 5.1 | field **is a button but is not submitter** ⇒ continue | **part 2** — explicit exclusion via `FormControlKind::is_button` (NEW) in `collect_control_entry` (`crates/dom/elidex-form/src/submit.rs:349-355`) | ✓ for *membership*; elidex reaches it as filter-all + append-submitter, not as step 5.1 (§1, §4.3) | yes — form submit |
| WHATWG HTML §4.10.22.4 Constructing the entry list | step 5.1 | the **submitter**'s entry is created **in tree order** | **OUT** — elidex appends it (`submit.rs:158-180`) | ✗ — slot `#11-submitter-entry-tree-order` (§10) | yes |
| WHATWG HTML §4.10.22.4 Constructing the entry list | step 5.2 | image-button submitter ⇒ `name.x` / `name.y` entries | **OUT** — `from_type_str` (`crates/dom/elidex-form-core/src/lib.rs:253-282`) has no `"image"` arm | ✗ — existing slot `#11-input-image-state` | yes |
| WHATWG HTML §4.10.22.4 Constructing the entry list | step 5.1 | field **is disabled** ⇒ continue | **untouched** — stays `fcs.disabled`; migrates to the live predicate in Slice 1 | n/a (out) | yes |
| WHATWG HTML §4.10.21.1 Definitions | candidate for constraint validation | submittable ∧ ¬barred | **part 3** — `is_constraint_validation_candidate` (`crates/dom/elidex-form-core/src/validation/mod.rs:21-44`) | ✓ | yes |
| WHATWG HTML §4.10.6 The button element | constraint validation | a `<button>` that is **not** a submit button is barred | part 3 — bars kinds `ResetButton` / `Button` | ✓ | yes — `button.type=` |
| WHATWG HTML §4.10.8 The datalist element | constraint validation | an element with a `<datalist>` **ancestor** is barred | **part 3 (NEW)** — bounded ancestor walk in the same predicate | ✓ | yes — DOM insert/move |
| WHATWG HTML §4.10.5.1.18 Submit Button state (type=submit) | constraint validation | **no bar** — a submit button IS a candidate (both tags) | part 3 leaves `SubmitButton` unbarred — the delta this PR exists to produce | ✓ | yes |
| WHATWG HTML §4.10.5.1.19 Image Button state (type=image) | constraint validation | **no bar** — an image button is a candidate | **pre-existing gap**: `<input type=image>` maps to `TextInput`, so it is a candidate running *text* constraints | ✗ — `#11-input-image-state` | yes |
| WHATWG HTML §4.10.5.1.20 Reset Button state (type=reset) | constraint validation | `<input type=reset>` barred | `ResetButton` bar (both tags map to that kind) | ✓ | yes |
| WHATWG HTML §4.10.5.1.21 Button state (type=button) | constraint validation | `<input type=button>` barred | `Button` bar (both tags) | ✓ | yes |
| WHATWG HTML §4.10.5.1.1 Hidden state (type=hidden) | constraint validation | `<input type=hidden>` barred | existing branch `validation/mod.rs:32`, cite added by PR-A0 | ✓ | yes |
| WHATWG HTML §4.10.21.3 The constraint validation API | `willValidate` / `checkValidity()` / `validationMessage` / `setCustomValidity()` | control-level surface reading candidacy | no logic change; the newly-reachable `customError` on submit buttons is a consequence (§5) | ✓ | yes — `btn.setCustomValidity('x')` |
| WHATWG HTML §4.10.3 The form element | `requestSubmit()` | the entry point; its validation step is §4.10.22.3 step 5.4 → §4.10.21.2 | `native_form_request_submit` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:546-556`) — **no logic change in PR-A, real behavior delta** (§5.1 row 6) | ✓ incl. the `novalidate`/`formnovalidate` escape at `:548-549` | yes — `f.requestSubmit()` |
| WHATWG HTML §4.10.21.2 Constraint validation | statically validate, steps 1-4 | walk → candidacy → satisfies-constraints → invalid list | **part 4** — `elidex_form::statically_validate_constraints` (NEW), `crates/dom/elidex-form/src/submit.rs` beside `reset_form` (`:464`) | ✓ | yes — form submit |
| WHATWG HTML §4.10.21.2 Constraint validation | statically validate, steps 5-7 | fire cancelable `invalid`; collect **unhandled** invalid controls; return them | **part 4** — `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) keeps only these; fixes the interleave + discarded-cancelable bugs (§5.1 rows 12-13) | ✓ | yes |
| WHATWG HTML §4.10.22.3 Form submission algorithm | step 5.4 | `novalidate` / `formnovalidate` skip | **part 4 (judgment)** — `no_validate_state` export if small (§4.5, §9-Q8) | ✓ | yes |
| WHATWG HTML §4.10.18.3 Association of controls and forms | `form=` IDREF association | the cross-tree collection half of `collect_form_data` | citation repair at `submit.rs:322` — **PR-A0** | n/a (doc-only) | no |

**Dependency worth naming**: rows 6-7 (`run_form_check_validity` reaching buttons at all) hold only because `CollectionFilter::FormControls` resolves via `matches_tag_ascii_ci_listed` (`crates/dom/elidex-dom-api/src/live_collection/mod.rs:512`, doc at `:549`: the HTML §4.10.2 *listed* elements), which includes `"button"`. If that filter is ever narrowed, row 6 collapses — part 4's `statically_validate_constraints` inherits the same dependency via `collect_form_entities`.

**Breadth**: K=1 spec (WHATWG HTML), M=21 rows → preflight verdict **SPLIT-RECOMMENDED (M≥20)**.

**Single-PR appeal** (the count rose from 18 because part 4 added the §4.10.21.2 / §4.10.22.3 rows, not because scope diffused): the rows enumerate branches of **one spec cluster reached by one predicate**. Part 4 is not a second axis — `statically_validate_constraints` is *the* consumer of `is_constraint_validation_candidate` at form scope, so hoisting it and repairing the predicate are the same edit seen from two altitudes; splitting them would ship a hoist that still calls a mis-set predicate, or a repaired predicate whose only form-level caller stays in `vm/host/`. What genuinely was separable **has been separated three times**: PR-A0 (citations), the shell facet (slot), and Slice 1 (the CSS reader). §9-Q10 asks review to challenge exactly this.

### §3.1 User-input touch audit

No new opcode, native, or dispatch path. The user-controllable input that selects a branch is the **`type` content attribute**, which reaches `FCS.kind` through an existing chokepoint — traced, not asserted:

`html_button_proto.rs:324` (the `button.type=` IDL setter) → `element_attrs.rs:137-153` (generic `attr_set` marshalling) → `EcsDom::set_attribute` → `reconciler.rs:128-164` (`handle_attribute_change`, whose `type` arm re-derives `FCS.kind` via `FormControlKind::from_tag_and_type_attr`). No new writer; PR-A only changes how the *derived* kind is classified downstream.

Exposure delta:
- **Entry list**: none (§4.3).
- **Validation**: three kinds newly enter `validate_control`, all landing in its **no-constraint arm** (`crates/dom/elidex-form-core/src/validation/mod.rs:178-186`: `SubmitButton | ResetButton | Button | Color | File | Hidden | Output | Meter | Progress => {}`), so no value constraint runs and no regex executes. Only `customError` becomes reachable, from a script-supplied string already stored in `FCS.custom_validity_message`.
- **Datalist bar**: a bounded ancestor walk, same shape and same `MAX_ANCESTOR_DEPTH` cap as `is_fieldset_disabled` (`crates/dom/elidex-form-core/src/fieldset.rs:46-76`). It *narrows* the validated set.
- **Submission**: newly abortable — §5.1 row 6. The one genuinely new user-visible pathway.

---

## §4 The edit set

### §4.1 Part 1 — `is_submittable()` becomes the §4.10.2 category

**Site**: `crates/dom/elidex-form-core/src/lib.rs:200-224` (+ doc at `:197-199`). Add `SubmitButton`, `ResetButton`, `Button`; `Output` / `Meter` / `Progress` stay out. Rewrite the doc to say it is the *category*, naming both consumers' extra rules so the conflation cannot recur.

**Blast radius — 2 production callers + 2 tests + 1 doc mention**, verified 2026-07-26 via `grep -rn "is_submittable" crates/ --include='*.rs'` (14 hits):

| Site | Kind | Action |
|---|---|---|
| `crates/dom/elidex-form/src/submit.rs:353` | production | part 2 |
| `crates/dom/elidex-form-core/src/validation/mod.rs:26` | production | part 3 |
| `crates/dom/elidex-form-core/src/lib_tests.rs:411-419` (`is_submittable_kinds`) | test | **inverted** — `:415-417` assert `!SubmitButton`/`!ResetButton`/`!Button` today |
| `crates/dom/elidex-form-core/src/validation/tests.rs:736-745` (`candidate_button_not_submittable`) | test | **inverted + renamed + comment rewritten** (§4.4) |
| `crates/script/elidex-js/src/vm/host/validity_state.rs:154` | doc comment (bar list) | §4.5 — prose *and* number |
| `crates/dom/elidex-form/src/submit_tests.rs:446` | — | `fn hidden_input_is_submittable()` is a **test-function name**, not a call. Not a caller. |

### §4.2 New shared predicate — `FormControlKind::is_button()` (NEW)

Part 2 and part 3 both need "is this kind a button kind", and `submit.rs:283` `is_submit_button` is a third variant of the same question. Author one predicate in form-core beside `is_submittable`:

```rust
/// HTML §4.10.22.4 step 5.1 ("field is a button") — the three button
/// kinds, i.e. `<button>` in any `type` state and `<input>` in the
/// Submit / Reset / Button states. NOT the §4.10.2 "submit button"
/// concept, which is narrower (see `is_submit_button`).
#[must_use]
pub fn is_button(self) -> bool {
    matches!(self, Self::SubmitButton | Self::ResetButton | Self::Button)
}
```

`FormControlKind::is_button` (NEW). Called from `collect_control_entry` (part 2) and from part 3's bar. **Audit item**: `is_submit_button` (`crates/dom/elidex-form/src/submit.rs:283`) stays where it is — it takes `(dom, entity)` and answers the §4.10.2 *submit-button* concept for `requestSubmit`'s step-2.1 type check, a different question from `is_button`. §9-Q6 asks review whether the two should be co-homed.

### §4.3 Part 2 — `collect_control_entry` states the button exclusion

**Site**: `crates/dom/elidex-form/src/submit.rs:349-355`. Today the exclusion arrives for free from the wrong predicate. Step 5.1 verbatim (`body html constructing-form-data-set`):

> * field has a datalist element ancestor;
> * field is disabled;
> * **field is a button but it is not submitter**;
> * field is an input element whose type attribute is in the Checkbox state and whose checkedness is false; or
> * field is an input element whose type attribute is in the Radio Button state and whose checkedness is false,
>
> then continue.

**Comment wording (per §1)**: the new comment must describe elidex's **filter-all-buttons + append-submitter approximation**, not claim to implement step 5.1. Suggested: *"§4.10.22.4 step 5.1 excludes a button unless it is the submitter. `collect_control_entry` has no submitter parameter, so elidex excludes every button here and re-adds the submitter in `build_form_submission`'s append pass (`:158-180`). Membership matches; **ordering does not** — slot `#11-submitter-entry-tree-order`."*

**Step 5.1's FIRST bullet must land here too.** The quoted step bars a field with a **datalist element ancestor** *and* a non-submitter button. Part 3 implements the datalist bar for *validation*; without the same bar here, a named `<input>` inside `<datalist>` still contributes an entry. Both bullets go into `collect_control_entry`, sharing the one `is_html_datalist` walk (§4.4).

**Entry-list delta — stated honestly (this replaces an earlier byte-identity claim, which was wrong).** Reasoned at the filter, so it holds for every consumer (I2 × I5):
- **Button exclusion: a no-op.** `collect_control_entry` takes no `submitter`, so it excludes all three button kinds — exactly the set the old `is_submittable` excluded. Unchanged output.
- **Datalist exclusion: a deliberate change.** Named controls inside a `<datalist>` stop contributing entries. That is what §4.10.22.4 step 5.1 requires; it is the *only* entry-list delta (§5.1 row 11, test §6-S3).

**The consumer chain**, verified 2026-07-26 via `grep -rn "collect_form_data\|build_form_submission" crates/ --include='*.rs'`:

1. **`build_form_submission`** (`crates/dom/elidex-form/src/submit.rs:136`) — calls `collect_form_data` at `:156`, then appends the submitter at **`:158-180`**, gated `fcs.kind == FormControlKind::SubmitButton && !fcs.name.is_empty()` (`:160`), reading the `value` **content attribute** (`:170-172`). Untouched. *(The earlier draft called this `submit_form`; **no such function exists**.)*
2. **`vm/host/form_data.rs:255`** (`populate_form_data_from_form`, `:245-266`) — calls `collect_form_data` **directly** and appends **no** submitter. It has **two** callers, and they are not equivalent:
   - `native_form_data_constructor` — `new FormData(form)`, submitter genuinely null. **Correct.**
   - `build_form_data_from_form` (`form_data.rs:281`) — backs the **`FormDataEvent.formData` payload** dispatched at `html_form_proto.rs:570`. Per §4.10.22.3 the entry list there is constructed **with** the submitter, so step 5.1 admits it — and elidex's payload omits it. Observable via `f.addEventListener('formdata', e => …)` after `requestSubmit(b)`. **Pre-existing membership gap**, not introduced here; folded into `#11-submitter-entry-tree-order`, whose scope §10 widens from ordering-only to *"submitter representation in the entry list — placement and formdata-payload membership"* (decided explicitly rather than left implicit).
3. **shell `content/form_input.rs:151`** — calls `build_form_submission` (the click-submit path), so it inherits consumer 1 rather than being a peer.

### §4.4 Part 3 — the §4.10.6 / §4.10.5.1.x / §4.10.8 bars

**Site**: `crates/dom/elidex-form-core/src/validation/mod.rs:26`.

**The §4.10.6 antecedent, quoted in full** (`body html attr-button-type-submit-state`):

> A button element is said to be a submit button if **any** of the following are true:
> * the `type` attribute is in the Auto state, both the `command` and `commandfor` content attributes are not present, and the parent node is not a `select` element; **or**
> * the `type` attribute is in the Submit Button state.
>
> Constraint validation: If the element is not a submit button, the element is barred from constraint validation.

Note the shape: a **disjunction of two bullets**, the *first* of which carries three conjuncts. `<button type=submit>` matches bullet 2 unconditionally; only the **Auto state** (the attribute's missing/invalid value default) goes through bullet 1.

**Modelling premise for the two-variant collapse — stated so review can challenge it.** elidex maps a bare `<button>` (Auto state) to `FormControlKind::SubmitButton` via `from_tag_and_type_attr` (`crates/dom/elidex-form-core/src/lib.rs:232-247`). That equals bullet 1 **iff** its other two conjuncts can never fail, which holds because:
- `command` / `commandfor` are **unmodelled engine-wide** — verified 2026-07-26 via `grep -rn "commandfor\|\"command\"" crates/ --include='*.rs'` → **0 hits**; and
- `<button>` inside `<select>` is unmodelled (`from_element` classifies `"button"` with no parent check; §4.10.6's "first element child of a select ⇒ inert" rule is likewise absent).

Under that premise, Auto-state `<button>` ≡ submit button, so the bar reduces to two kinds. **No slot** — consistent with §9-Q5's FACE treatment: these are unmodelled *surfaces* (`command`/`commandfor` invokers; `<button>`-in-`<select>`), not gaps in this predicate. Flagged for review challenge at §9-Q4.

| Kind | as `<button …>` | as `<input type=…>` | Barred? |
|---|---|---|---|
| `SubmitButton` | §4.10.6 — a submit button is not barred | §4.10.5.1.18 — no barring statement | **NO — candidate** |
| `ResetButton` | §4.10.6 — not a submit button ⇒ barred | §4.10.5.1.20 — barred | **YES** |
| `Button` | §4.10.6 — not a submit button ⇒ barred | §4.10.5.1.21 — barred | **YES** |

**No `TagType` read is required** — the natural assumption that `SubmitButton` must be disambiguated between `<button type=submit>` (candidate) and `<input type=submit>` (assumed barred) is **false**: §4.10.5.1.18 states no bar, so both are candidates.

```rust
// §4.10.6 (a <button> that is not a submit button) and §4.10.5.1.20 /
// §4.10.5.1.21 (<input type=reset|button>) bar these two kinds under
// both tags. A submit button — <button type=submit> AND <input
// type=submit> (§4.10.5.1.18 states no bar) — is a candidate.
// Authored as the barred SET, not as `is_button() && != SubmitButton`:
// a subtraction would be a third spelling of submit-button-ness (§4.2).
if matches!(state.kind, FormControlKind::ResetButton | FormControlKind::Button) {
    return false;
}
```

**Barred set, not subtraction.** Three spellings of "is this a submit button" would otherwise coexist: `is_submit_button` (`crates/dom/elidex-form/src/submit.rs:283`), the inline `fcs.kind == FormControlKind::SubmitButton` at `submit.rs:160`, and a subtraction here. Part 3 states the barred set directly, and part 2 folds `submit.rs:160` onto the same predicate as `is_submit_button` so exactly **two** spellings remain — the §4.10.2 *submit-button* concept (one predicate, two call sites) and the step-5.1 *button* concept (`is_button`).

**The datalist bar — one walk, one identity test, two consumers.**

`crates/dom/elidex-form-core/src/input.rs:781-798` (`matches_datalist_with_id`) **already** answers "is this an HTML `<datalist>`": ASCII-case-insensitive tag match **plus** `dom.is_html_namespace(entity)` (`crates/core/elidex-ecs/src/dom/mod.rs:816`), with a dedicated regression test at `crates/dom/elidex-form-core/src/input_tests.rs:375-386` (`resolve_input_list_rejects_foreign_namespace_datalist`). Mirroring `is_fieldset_disabled`'s namespace-blind `t.0 == "fieldset"` shape would **re-derive that identity test wrongly**: an SVG `<datalist>` ancestor would bar, and `createElement('DATALIST')` would slip — the exact false positive form-core already has a test against.

So: extract **one** `is_html_datalist(dom, entity) -> bool` (NEW) in form-core and route **all three** spellings through it — `matches_datalist_with_id` (`input.rs:781`), `find_options_container` (the other datalist identity site in `input.rs`), and the new ancestor walk. One-issue-one-way; the existing namespace regression test then covers the new consumer for free.

**One merged ancestry walk, not two.** Candidacy already runs a `MAX_ANCESTOR_DEPTH` walk for `is_fieldset_disabled`, and this adds a second — on a hot path (`validity_state.rs:175`/`:405`/`:447`/`:469`, plus once per control inside `statically_validate_constraints`). `is_fieldset_disabled` has an outside caller (`crates/dom/elidex-form/src/init.rs:89`) so it cannot absorb the datalist test. Resolution: a **private single-pass walk inside `is_constraint_validation_candidate`** that tests each ancestor for both conditions and short-circuits, leaving the public `is_fieldset_disabled` untouched for its outside caller. §9-Q9 asks review to confirm the duplication of the fieldset logic inside that private walk is the right trade against two passes.

`validation/tests.rs:736-745` `candidate_button_not_submittable` is **inverted, renamed, and its comment rewritten** — it asserts a default `<button>` (⇒ `SubmitButton`) is not a candidate *and* states the false premise "Button kinds are not submittable" (I3 × I4).

### §4.5 Part 4 — hoist the form-level validation algorithm into `elidex-form`

**Why in PR-A rather than slotted.** The Layering mandate's instruction when no engine-independent home exists is **無ければそちらを拡張** — *create* it — not "defer because there is nowhere to put it". And the umbrella already ratified this exact shape for the sibling case: `#11-shell-effective-disabled-predicate-adoption` exports the predicate from the engine-independent crate **in-lane now** and defers only the cross-lane shell adoption. PR-A does the same: hoist here, defer only the shell facet (§10).

**No new crate edge, no new idiom.** `elidex-form` already depends on `elidex-dom-api` and already uses `LiveCollection` / `CollectionFilter` (`crates/dom/elidex-form/src/select/mod.rs:65`, `:114`, `:146`, `:171`). `reset_form` (`crates/dom/elidex-form/src/submit.rs:464`) is the crate's documented precedent for the shape: the engine-independent algorithm computes over `collect_form_entities` (`submit.rs:531`) and the caller fires the events.

**New export** — HTML §4.10.21.2 `#statically-validate-the-constraints` **steps 1-4**, authored beside `reset_form`:

```rust
/// HTML §4.10.21.2 "statically validate the constraints", steps 1-4:
/// walk the form's submittable controls in tree order, skip
/// non-candidates and controls that satisfy their constraints, and
/// return the invalid ones. PURE — steps 5-7 (firing `invalid`,
/// collecting the *unhandled* controls) belong to the caller, which
/// owns event dispatch. Empty result == the spec's positive result.
pub fn statically_validate_constraints(dom: &EcsDom, form: Entity) -> Vec<Entity>;
```

`elidex_form::statically_validate_constraints` (NEW), reusing `collect_form_entities` for step 1.

**Caller rewrite**: `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) calls it and keeps only steps 5-7.

**This fixes a live bug — two of them.** Today's loop (`:629-652`) validates control *N* and **immediately dispatches `invalid` on it** before validating *N+1*, and discards the dispatch result (`let _ = super::event_target_dispatch::dispatch_simple_event(…)?`). Against the spec prose (§0.5):
1. **Interleaving.** Steps 3 and 6 are *separate* loops: every control is validated (step 3) before any `invalid` fires (step 6). Today an `invalid` listener firing on control *N* can mutate control *N+1* — or insert/remove controls — before *N+1* is validated. §6-U8 asserts the ordering.
2. **The cancelable result is discarded.** Step 6.1 fires `invalid` **cancelable**, and step 6.2 keeps only the *non*-canceled controls as "unhandled invalid controls" (step 7's return). elidex drops `notCanceled`, so **"unhandled invalid controls" does not exist in the engine** — nothing downstream can distinguish a script that claimed responsibility from one that did not. Part 4 threads the result; the immediate consumer is `native_form_request_submit`'s abort, and the concept is what §4.10.21.2's interactive-validation step consumes.

**Second export, if it stays small**: `no_validate_state(dom, form, submitter) -> bool` (§4.10.22.3 step 5.4 — form `novalidate` OR submitter `formnovalidate`), replacing the inline read at `html_form_proto.rs:546-551`. Precedent: `build_form_submission` already resolves the submitter's `formmethod` / `formaction` overrides engine-independently. **Judgment at impl** (§9-Q8): export it if it is the two-attribute read it appears to be; leave it inline if the submitter-resolution it needs pulls more of the submission algorithm across the boundary than it removes.

**Citation note**: PR-A0 renumbers the comments in these files; part 4 lands **on top of** the corrected numbers and adds only the prose describing its own new behavior.

**Reciprocal rows from PR-A0 §4.6** (so neither memo can silently drop a site):

| Site | Owner | Why |
|---|---|---|
| `crates/dom/elidex-form/src/submit.rs:276-279` (the false `<input type=image>` → `SubmitButton` claim) | **PR-A0** | it is a citation/prose repair, not PR-A behavior; previously double-claimed |
| `crates/script/elidex-js/src/vm/host/validity_state.rs:152` (§ number) + `:154` (bar list, incl. the **missing `readonly` bar**) | **PR-A0** | pre-existing prose defect; PR-A only adds bars arising from *its own* new behavior |
| `crates/dom/elidex-form-core/src/validation/mod.rs:12` — the § number | **PR-A0** | — |
| `crates/dom/elidex-form-core/src/validation/mod.rs:12` — **the new-bar prose** (non-submit buttons, datalist ancestor) | **PR-A** | describes part 3's behavior, which does not exist until this PR |
| the `§4.10.20.x` / `§4.10.21.3` / `§4.10.21.4` / `§4.10.21.5` families in `validation/mod.rs`, `submit.rs`, `validity_state.rs`, `html_form_proto.rs` | **PR-A0 — already swept** (landed on this branch) | PR-A rebases onto the corrected numbers and must not re-cite the old ones; its own comments cite §4.10.21.1 / §4.10.21.2 / §4.10.21.3 / §4.10.22.3 as §0.5 records |

## §5 Behavior deltas

### §5.1 What changes

| # | Surface | Element | Today | After | Why | Test |
|---|---|---|---|---|---|---|
| 1 | `willValidate` (JS) | `<button>` / `<button type=submit>` | **false** | **true** | `SubmitButton` was missing from the category (§4.10.2 + §4.10.6) | §6-U1 |
| 2 | `willValidate` (JS) | **`<input type=submit>`** | **false** | **true** | same kind; §4.10.5.1.18 states no bar | §6-U1 |
| 3 | `checkValidity()` / `reportValidity()` (JS) | those elements | `true` (vacuously — barred controls short-circuit) | `true` **unless** `setCustomValidity(nonEmpty)` | now candidates; `validate_control`'s no-constraint arm leaves only `customError` (§4.10.21.3) | §6-U2 |
| 4 | `validationMessage` (JS) | those elements | `""` | the custom message when set | same candidacy gate (`validity_state.rs:407`) | §6-U2 |
| 5 | `validity` (JS) | those elements | all-false stub (`validity_state.rs:175`) | computed — still all-false unless `customError` | same | §6-U3 |
| **6** | **`form.requestSubmit()`** | a form whose submit button carries a custom error | submits | **aborts** — no `submit` event, no `formdata` event, no navigation, **but the button DOES receive an `invalid` event** (`run_form_check_validity:641-650` dispatches on every failing control before returning false). Not silent | `native_form_request_submit` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:546-556`) runs `run_form_check_validity` and returns early on failure. **The submission-outcome delta the earlier draft missed.** Escapes: form `novalidate` / submitter `formnovalidate` (`:548-549`) | §6-U4, §6-U4b |
| 7 | `form.checkValidity()` / `reportValidity()` | form containing a submit button | button skipped | button evaluated; result unchanged unless it carries a custom error | `run_form_check_validity` (`html_form_proto.rs:620-654`) filters on the same predicate | §6-U5 |
| 8 | `willValidate` etc. | `<button type=reset\|button>` / `<input type=reset\|button>` | false | **false — unchanged** | part 1 alone would have made them candidates; part 3's bar restores it. **The assertion proving parts 1 and 3 landed together** | §6-U6 |
| **9** | `willValidate` / `checkValidity()` | any control with a `<datalist>` ancestor | **true** (bar unimplemented) | **false** | §4.10.8 | §6-U7 |
| 10 | form submission entry list | any form, **buttons only** | — | **unchanged** | §4.3 button bullet is a no-op | §6-S1, §6-S2 |
| **11** | form submission entry list | a **named control inside `<datalist>`** | contributes an entry | **excluded** | §4.10.22.4 step 5.1, first bullet (§4.3) | §6-S3 |
| **12** | `invalid` event ordering | a form with ≥2 failing controls | `invalid` on control *N* fires **before** control *N+1* is validated | **all validation completes first**, then `invalid` fires in tree order | §4.10.21.2 steps 3 and 6 are separate loops (§4.5) | §6-U8 |
| **13** | "unhandled invalid controls" | any failing control whose `invalid` listener calls `preventDefault()` | the concept does not exist — the cancelable result is discarded | threaded through steps 6-7 | §4.10.21.2 steps 6.1-6.2, 7 (§4.5) | §6-U9 |

### §5.2 Submission-blocking impact — checked, not assumed

`validate_control` (`crates/dom/elidex-form-core/src/validation/mod.rs:104-207`) routes `SubmitButton`/`ResetButton`/`Button` into an explicit no-constraint arm alongside `Color`/`File`/`Hidden`/`Output`/`Meter`/`Progress`, so no `valueMissing` / `tooLong` / pattern / range / step flag can fire on a button. The custom-validity block (`:196-204`) sits **outside** the per-kind match, so `customError` becomes reachable — spec-correct, since §4.10.21.3 lists `HTMLButtonElement` among `setCustomValidity`'s interfaces.

The consequence is **not** confined to getters: because `native_form_request_submit` aborts on validation failure, a script doing `btn.setCustomValidity('x'); form.requestSubmit()` now gets **no submission at all** where today it got one (row 6). It is **not silent** — `run_form_check_validity:641-650` dispatches a cancelable `invalid` event on the button before returning false, so a listener observes the failure and (after part 4) can claim responsibility for it. That is spec-correct interactive validation, but it is a real behavior change and belongs in the PR description, not only in this memo.

### §5.3 `:valid` / `:invalid` are unaffected by PR-A — and the divergence that creates

Pre-Slice-1, `matching.rs` answers `:valid`/`:invalid` from the `ElementState::VALID` bit stamped unconditionally at `crates/dom/elidex-form/src/init.rs:169`, reached via `form_element_state()` (`crates/css/elidex-css/src/selector/matching.rs:126-133`). Verified 2026-07-26 via `grep -rn "is_constraint_validation_candidate" crates/ --include='*.rs'`: the only **non-test** call sites are `crates/script/elidex-js/src/vm/host/validity_state.rs` (`:175`, `:405`, `:447`, `:469`) and `crates/script/elidex-js/src/vm/host/html_form_proto.rs:636` — **no `elidex-css` site appears**. So PR-A changes no CSS answer.

**But that is not a clean bill.** `<button>` **is** in `matching.rs`'s `is_form_element` list (`:371-385`), and every FCS-bearing control gets the `VALID` bit at init. So after PR-A a submit button carrying a custom error reports `validity.valid === false` in JS **while still matching `:valid` in CSS** — a newly-populated instance of exactly the JS/CSS divergence Slice 1 exists to delete. It is bounded (PR-A → Slice-1 window only) and it is not a regression (the CSS side is equally wrong for every control today), so it is a further argument for the prereq ordering rather than against it. Cross-referenced from §11.

### §5.4 Existing tests — re-derived **by consumer path**, not by API-name grep

An API-name grep (`willValidate|checkValidity|…`) structurally cannot reach submission tests that exercise candidacy *through* `requestSubmit`. Re-derived by walking each consumer from §4.3 plus the candidacy gate:

| Consumer path | Test file | Contains a button? | Impact |
|---|---|---|---|
| control-level CVA getters | `tests_validity_state.rs` | `:222-226` `button_validity_returns_object` asserts only `typeof b.validity === 'object'` | **none** |
| form-level `checkValidity` | `tests_html_form_proto.rs:199-275` (6 cases) | **no** — every fixture is `<form>` + `<input>` (+ one `<fieldset disabled>`) | **none** |
| `invalid` event dispatch | `tests_events_misc.rs:872-890` | no — `<form>` + one required `<input>` | **none** |
| **`requestSubmit` / submit-event** | **`tests_form_submit.rs`** — **12 `button` hits**, incl. **E-3 at `:117-128`** (`createElement('button')` appended to a form, `f.requestSubmit(b)`) | **yes** | **none by outcome** — the button carries no custom validity, so `run_form_check_validity` still returns `true` and the submit event fires. **But this is the file a name-grep misses**, and row 6's new abort path lands here; §6-U4 is added *beside* E-3 |
| `new FormData(form)` / formdata payload | `tests_form_data.rs` — **0 `button` hits** | no | **none** |
| entry-list unit tests | `submit_tests.rs` — `:307` `buttons_not_submitted`, `:504` `submitter_entry_included` | yes | **none** — both must stay green; they are the pre-existing guard for §4.3 |
| form-core candidacy units | `validation/tests.rs:736-745`, `lib_tests.rs:411-419` | yes | **both inverted** (I4) |
| mixin install / brand checks | `tests_html_t2d_protos.rs:940-960`, `:1200-1206` | `<output>` only — stays out of the category | **none** |

**Only the two form-core unit tests change.** Everything else stays green; the additions are new cases, not rewrites.

---

## §6 Test plan

- **F1** (part 1) — `is_submittable_kinds` (`crates/dom/elidex-form-core/src/lib_tests.rs:411-419`): invert `:415-417` with the §4.10.2 cite; keep `Output`/`Meter` negative and add `Progress`.
- **F2** (part 3) — replace `candidate_button_not_submittable` (`crates/dom/elidex-form-core/src/validation/tests.rs:736-745`) with a per-kind set, each arm carrying its section cite: candidates = `<button>`, `<button type=submit>`, `<input type=submit>`; barred = `<button type=reset>`, `<button type=button>`, `<input type=reset>`, `<input type=button>`. **Rewrite the comment** — the current one states the false premise.
- **F3** (part 3, datalist) — a control inside `<datalist>` is not a candidate; the same control outside it is; nested/deep ancestry; a `<datalist>` *sibling* does not bar.
- **F4** (§4.2) — `FormControlKind::is_button` (NEW) over the full variant set, asserted **against** `is_submittable` so the two cannot silently converge.
- **S1** (I2 proof, submitter-**first**) — in `crates/dom/elidex-form/src/submit_tests.rs`: a form whose named `<button type=submit>` submitter appears **before** other named controls. Assert **membership and count**, order-insensitively (submitter appears exactly once; every named non-button control appears). Deliberately *not* order-asserting — the ordering divergence is slotted (§10), and an order-asserting fixture would encode it as intended. Submitter-first also distinguishes this from the existing `submit_tests.rs:504` `submitter_entry_included` (submitter-only, order-trivial) and `:307` `buttons_not_submitted`.
- **S2** (I2 × I5) — the same assertion through the `vm/host/form_data.rs` path: `new FormData(form)` over a form containing a non-submitter button yields no button entry.
- **U1** (rows 1-2) — `<button>` / `<button type=submit>` / `<input type=submit>` → `willValidate === true`.
- **U2** (rows 3-4) — `<input type=submit>.checkValidity()` → `true`; after `setCustomValidity('x')` → `false` and `validationMessage === 'x'`; after `setCustomValidity('')` → `true`.
- **U3** (row 5) — `<button type=submit>.validity.valid` → `true`, `.customError` → `false` by default.
- **U4** (row 6, **the new abort path**) — placed beside E-3 in `tests_form_submit.rs`: a form + submit button with `setCustomValidity('x')`; `f.requestSubmit(b)` fires **no** `submit` event — **and assert the button DOES receive `invalid`** (the abort is observable, not silent).
- **U4b** (row 6 escapes) — the same fixture with form `novalidate`, and again with submitter `formnovalidate`, **does** fire `submit` (`html_form_proto.rs:548-549`).
- **U5** (row 7) — form `checkValidity()` stays `true` with a valid submit button; `false` once the button carries a custom error, and the button receives an `invalid` event.
- **U6** (row 8) — `<button type=reset\|button>` / `<input type=reset\|button>` → `willValidate === false`. **Fails if part 1 lands without part 3.**
- **U7** (row 9) — an `<input required>` with an empty value inside `<datalist>` → `willValidate === false`, `checkValidity() === true`. Plus a **namespace guard**: an SVG-namespaced `<datalist>` ancestor does **not** bar (mirrors `input_tests.rs:375-386`).
- **S3** (row 11) — a named `<input>` inside `<datalist>` contributes **no** entry to `collect_form_data`; the same input outside it does.
- **U8** (row 12, part 4) — a form with two failing controls: an `invalid` listener on the first mutates the second (e.g. clears its value so it would newly pass); assert the second **still** receives `invalid`, proving validation completed before any event fired.
- **U9** (row 13, part 4) — an `invalid` listener that calls `preventDefault()` marks its control *handled*; assert the threaded "unhandled invalid controls" result differs from the un-prevented case.
- **F5** (part 4) — `elidex_form::statically_validate_constraints` unit tests in `submit_tests.rs`: empty form → empty vec; all-valid → empty; mixed → the invalid ones **in tree order**; non-candidates skipped. Pure, no VM needed — which is the point of the hoist.

**Gate**: `cargo test -p elidex-form-core -p elidex-form -p elidex-js -p elidex-shell --all-features`, then the full **`/pre-push` composite** (`cargo fmt` → `mise run ci` → `/simplify` → `/code-review` → `/review` → `/elidex-review`), then **`/external-converge` from round 1**. Per §2 this work *does* meet the edge-dense trigger, and per the proactive branch of [[feedback_gate-miss-on-edge-dense-escalate-to-converge]] that means converge, not a single-pass `/external-review`. (An earlier draft claimed the opposite and contradicted its own §2.)

---

## §7 Layering check — with the one honest exception

- **Algorithm stays engine-independent** ✓ — parts 1, 3, `is_button`, and `has_datalist_ancestor` are in `elidex-form-core` (leaf); part 2 is in `elidex-form` (the submission system).
- **No crate-graph change** ✓ — no Cargo edit.
- **No B1-core (`elidex-ecs`) touch** ✓.
- **ECS-native** ✓ — no component added, removed, or re-homed; the predicates stay pure functions over `&FormControlState` (+ `Entity`/`&EcsDom` for the two ancestry walks). No side-store, no registry, no accessor wall.
- **The Layering drift is FIXED here, not deferred** (part 4). `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) was a form-level validation *algorithm* living in `vm/host/`, and `elidex-form` exported no form-level entry point (verified 2026-07-26: `grep -n "check_validity\|validate_form\|form_validity" crates/dom/elidex-form/src/lib.rs` → no match). The mandate's instruction in exactly that case is **無ければそちらを拡張** — create the home. Part 4 does: §4.10.21.2 steps 1-4 move to `elidex-form`, `vm/host/` keeps only steps 5-7 (event dispatch), which is precisely the marshalling-grade responsibility the mandate allows it. `vm/host/` **shrinks**. ✓
- **What remains deferred is a defect, not a refactor.** The shell click/Enter submit path (`crates/shell/elidex-shell/src/content/event_handlers.rs:155`/`:580` → `handle_form_submit` → `crates/shell/elidex-shell/src/content/form_input.rs:126-162`) omits §4.10.22.3 step 5.4 **entirely** — a form with an empty `<input required>` navigates on click — and carries an **independent second copy** of the submission event sequence (dispatches `submit`, never `formdata`). Both facets, one L4-shell slot: **`#11-shell-submit-interactive-validation`** (§10). Cross-lane, so it cannot ride this PR; naming the *defect* rather than the refactor is what makes it actionable.

---

## §8 Line-count budget

Verified 2026-07-26 (`wc -l`):

| File | Now | After (est.) | Verdict |
|---|---|---|---|
| `crates/dom/elidex-form-core/src/lib.rs` | 960 | ~975 | +3 variants in `is_submittable`, +`is_button` (~10), +doc. **Interaction with Slice 1**, which adds `required_applies` (~18): 960 → ~975 (PR-A) → ~993 (Slice 1). Under 1000 but with ~7 lines of headroom — if Slice 1's real delta crosses it, carve `FormControlKind` + its inherent impl (`lib.rs:89-326`) into `kind.rs` as a standalone prereq commit (the seam 0b §9 named). Flagged in Slice 1 §9 too. |
| `crates/dom/elidex-form-core/src/validation/mod.rs` | 430 | ~465 | +the two-variant bar, +`has_datalist_ancestor`, +sweep prose |
| `crates/dom/elidex-form/src/submit.rs` | 546 | ~558 | +button exclusion, +sweep, +the image-button docstring fix |
| `crates/dom/elidex-form-core/src/lib_tests.rs` | 885 | ~892 | F1 |
| `crates/dom/elidex-form-core/src/validation/tests.rs` | 745 | ~800 | F2 + F3 + F4 |
| `crates/dom/elidex-form/src/submit_tests.rs` | 765 | ~800 | S1 |
| `crates/script/elidex-js/src/vm/tests/tests_validity_state.rs` | 411 | ~470 | U1-U3, U5-U7 |
| `crates/script/elidex-js/src/vm/tests/tests_form_submit.rs` | 213 | ~260 | U4, U4b |
| `crates/script/elidex-js/src/vm/host/validity_state.rs` | 518 | 518 | sweep (doc-only) |
| `crates/script/elidex-js/src/vm/host/html_form_proto.rs` | 680 | 680 | sweep (doc-only) |

No file crosses 1000. `validation/tests.rs` (745 → ~800) is the one to watch across later slices.

---

## §9 Open questions for `/elidex-plan-review`

- **Q1 — is the entry-list preservation claim (§4.3) airtight?** It rests on: `collect_control_entry` having no `submitter` parameter; the old `is_submittable` excluding exactly the three button kinds; and the consumer chain being exhaustive. §6-S1/S2 are the empirical checks.
- **Q2 — is the `requestSubmit` abort (§5.1 row 6) acceptable as a silent behavior change?** **Recommendation: yes** — spec-correct interactive validation, with escape hatches. But it must be called out in the PR description, not just here, and §6-U4/U4b must land with it.
- **Q3 — should the ordering divergence (§4.3) be fixed rather than slotted?** **Recommendation: slot it.** It needs `submitter` threaded into `collect_form_data`/`collect_control_entry` with in-pass emission — a submission-pipeline refactor that entangles with `#11-input-image-state` (same pass).
- **Q4 — is the two-variant collapse sound?** It depends on the §4.4 modelling premise (`command`/`commandfor` = 0 hits; `<button>`-in-`<select>` unmodelled). If either is ever modelled, the bar must grow a conjunct. Review should decide whether that deserves a comment-level tripwire in `is_constraint_validation_candidate`.
- **Q5 — FACE** is in §4.10.2 but has no `FormControlKind` variant. Recorded as a known incompleteness with **no slot** (a whole surface, not a gap in this predicate) per `feedback_defer-slot-eligibility-audit-at-create`. Review may overrule.
- **Q6 — should `is_submit_button` (`crates/dom/elidex-form/src/submit.rs:283`) move to form-core beside `is_button`?** They answer adjacent questions (§4.10.2 submit-button concept vs step-5.1 button-ness). **Recommendation: leave it** — it takes `(dom, entity)` and serves `requestSubmit`'s type check, and moving it is a third re-layer with no consumer asking. Flagged so the two-predicate state reads as deliberate.
- **Q7 — the citation sweep is now PR-A0** (`docs/plans/2026-07-constraint-validation-citation-sweep.md`), a doc-only prereq. PR-A keeps only the prose that describes **its own** new behavior: `validation/mod.rs:12`'s bar list must gain the non-submit-button and datalist bars. Confirm the split leaves no cite half-repaired.
- **Q8 — export `no_validate_state` or leave it inline?** (§4.5). Judgment at impl: export if it is the two-attribute read it appears to be (form `novalidate` OR submitter `formnovalidate`); leave inline if resolving the submitter drags more of the submission algorithm across the boundary than it removes. Precedent for exporting: `build_form_submission` already resolves `formmethod`/`formaction` engine-independently.
- **Q9 — one merged ancestry walk vs two** (§4.4). The private single-pass walk duplicates `is_fieldset_disabled`'s logic inside `is_constraint_validation_candidate` to avoid two `MAX_ANCESTOR_DEPTH` passes on a hot path; `is_fieldset_disabled` cannot absorb the datalist test because `init.rs:89` calls it. Confirm the trade, or accept two passes and measure.
- **Q10 — is part 4's bug-fix scope right?** It fixes the interleave and the discarded cancelable because both are *inherent to splitting steps 1-4 from 5-7* — you cannot hoist the pure half without deciding what the impure half returns. Review should check that framing: if either fix is separable, it should be its own PR.

---

## §10 Defer slots + per-PR ≤3 audit

| Slot | Status | 4-question audit |
|---|---|---|
| **`#11-submitter-entry-tree-order`** | **NEW** (surfaced by part 2; **widened** per §4.3) | Scope = **submitter representation in the entry list**, both facets: (a) *placement* — §4.10.22.4 step 5.1 keeps the submitter's entry in tree order, elidex appends it (`crates/dom/elidex-form/src/submit.rs:158-180`); (b) *membership* — the `FormDataEvent.formData` payload (`form_data.rs:281` → `html_form_proto.rs:570`) omits the submitter entirely, though §4.10.22.3 constructs that entry list *with* it. (1) Real gap? yes, both observable. (2) Blocked by structure? yes — needs `submitter` threaded through with in-pass emission. (3) Non-regressing to defer? yes — pre-existing; §4.3 preserves today's list. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: a form-submission WPT asserting entry order, or the `#11-input-image-state` work. **Date**: 2026-11. |
| **`#11-shell-submit-interactive-validation`** | **NEW** (surfaced by §7; replaces the earlier refactor-named slot) | (1) Real gap? yes, and it is a **defect**, not a refactor: the shell click/Enter path (`crates/shell/elidex-shell/src/content/event_handlers.rs:155`/`:580` → `crates/shell/elidex-shell/src/content/form_input.rs:126-162`) omits §4.10.22.3 step 5.4 entirely, so a form with an empty `<input required>` navigates on click; and it carries an independent second copy of the submission event sequence (dispatches `submit`, never `formdata`). Both facets, one slot. (2) Blocked by structure? **cross-lane** — L4 owns shell; no active L4 worktree touches these. (3) Non-regressing to defer? yes — pre-existing; part 4 makes the fix *available* by exporting the algorithm. (4) Durable home? `project_open-defer-slots.md`, **L4 cluster**. **Trigger**: an L4 shell pass, or a click-submit validation report. **Date**: 2026-11. |
| `#11-input-image-state` | pre-existing | covers the image-button submitter's `name.x`/`name.y` entries **and** `from_type_str`'s missing `"image"` arm — which §3's Image Button row shows also makes `<input type=image>` a candidate running *text* constraints. Referenced, not created. **The false `submit.rs:276-279` docstring is repaired by PR-A0** (its §4.4 item 1), not here — the claim was double-held and is now assigned once. |

**Per-PR budget**: **2 new slots** (≤3 ✓, one slot of headroom).

**Slot co-location**: `#11-input-image-state` currently sits in the **Paint-pipeline (M4-13)** cluster of `project_open-defer-slots.md:190` — the wrong home for a form-submission gap, since it and the new `#11-submitter-entry-tree-order` need the *same* `collect_control_entry` + append pass rewritten. At ship, register the new slot in the **L3 form cluster** and **move `#11-input-image-state` there too**, with a one-line note that the two are a pair. Registration per [[reference_spawn-task-chips-not-durable]].

---

## §11 Coordination + carried edits

Re-derived 2026-07-26 from `git worktree list`, `git rev-list --count origin/main..<branch>`, and `gh pr list --state open` — **not** from MEMORY (whose L3 row is itself stale, below).

| Lane | Actual state | Overlap with PR-A | Ordering rule |
|---|---|---|---|
| **PR-A0** (`docs/plans/2026-07-constraint-validation-citation-sweep.md`, same worktree/branch) | plan-memo, doc-only | renumbers comments in every file PR-A then edits | **PR-A0 lands first — hard prereq** (§0). PR-A's parts 3/4 write prose onto the corrected numbers. |
| **Slice 1** (`elidex-wt-slice1`, `domform-slice1`, 0 commits — memo only) | plan-memo, `/elidex-plan-review` pending | `form-core/lib.rs` (PR-A: `is_submittable` + `is_button`; Slice 1: `required_applies`), `validation/mod.rs` (PR-A: basis + bars + datalist; Slice 1: disabledness collapse at `:29-31`/`:43`). Plus the §4.5 sweep boundary (Q7). | **PR-A lands first — hard prereq.** Slice 1 rebases; expect ~10-line offsets in `validation/mod.rs` citations. Cross-ref §5.3's bounded JS/CSS divergence. |
| **VM lane** (`elidex-wt-vmp4`, `vm-p4-plan`, **2 commits, unpushed**) | VM P4 umbrella plan authored, plan-review **not converged** | **Real**: PR-A edits `crates/script/elidex-js/src/vm/host/validity_state.rs` + `html_form_proto.rs` (sweep) and adds ~100 lines of VM tests. VM P4's declared root cause is the **compiler emit layer** + `vm/dispatch.rs`, not `vm/host/`. | No deliverable collision expected; PR-A lands first (VM P4 is pre-converge). Re-check if VM P4's `dispatch.rs` prereq split lands first. |
| **Layout lane** (`elidex-wt-c3a-impl`, `terminal-z-c3a-impl`, **16 commits, pushed — PR #488 open, mid-converge**) | `elidex-ecs` `dom/geometry.rs` + layout readers | none — PR-A touches no layout or `elidex-ecs` file | none |
| **History lane** (`elidex-wt-histq-b`, `session-history-slice-b`, **21 commits — PR #487 open**) | shell / session-history | none — PR-A touches no shell file | none |
| `elidex-wt-c3-plan` (`terminal-z-c3-plan`, 8 commits) | layout plan lane | none | none |
| `vm-input-value-as-date` | plan-only: one commit `75a0cebc`, one `.md`, **zero `.rs`** | none today; its future impl touches `form-core/input.rs` + `datetime.rs`, neither of which PR-A touches | none |

**Carried edits (ship with PR-A's PR):**
1. **Umbrella** `docs/plans/2026-07-form-derived-state-reconciliation.md` §6 — add a prereq row for PR-A above Slice 1 (same shape as 0a/0b).
2. **Umbrella §10:252** — "Program total = 2 new slots" is **stale**: the ledger already carries more from 0a/0b, and PR-A adds 2. Restate as a pointer to `project_open-defer-slots.md` rather than a hard-coded count that ages out.
3. **MEMORY.md L3 Active-state row** (line 62) — currently "0a #472 + 0b #479 LANDED; **Slice 1 (keystone) plan-memo DRAFTED**…" with **no mention of PR-A**. Add PR-A as the new hard prereq + its worktree/branch, and correct "▶ NEXT" accordingly.
4. **`project_open-defer-slots.md`** — register the 2 new slots and re-home `#11-input-image-state` into the L3 form cluster (§10).
5. *(The `project_html-section-renumber-sweep.md` update is **PR-A0's** carried edit, not PR-A's — see that memo's §10.)*

---

## §12 Where the brief or the earlier draft was wrong

**Round 3 — the two overturned scope decisions and what verification added.**

0a. **The layering carve was wrong to defer, and the slot was named after the wrong thing.** The mandate's instruction when no engine-independent home exists is 無ければそちらを拡張 (create it); and the umbrella already ratified the export-now/defer-the-cross-lane-adoption shape for `#11-shell-effective-disabled-predicate-adoption`. Part 4 hoists (§4.5); the slot is renamed to name the **defect** (`#11-shell-submit-interactive-validation`) rather than the refactor. Net new slots stays 2.
0b. **The sweep exclusion was directory-shaped and therefore unfalsifiable.** Replaced by fate-per-site (PR-A0 §4.6): `matching.rs:137`/`:284` are deleted by Slice 1 (OUT); `tests/matching.rs:715`/`:727` outlive it (IN) — and Slice-1 §6-C has **no row** for either, so directory exclusion would have orphaned them.
0c. **The entry-list byte-identity claim was false** — §4.3 quotes step 5.1 in full, whose *first* bullet is the datalist-ancestor bar, which part 2 had not implemented. Now implemented, with the honest delta (§5.1 row 11) replacing the identity claim.
0d. **`has_datalist_ancestor` would have re-derived a wrong identity test.** form-core already answers "is this an HTML `<datalist>`" with a namespace guard (`input.rs:781-798`) and has a regression test against exactly the false positive the naive shape would reintroduce (`input_tests.rs:375-386`). Now one shared `is_html_datalist`.
0e. **"Silently aborts" was wrong** — `run_form_check_validity:641-650` dispatches `invalid` on every failing control before returning false, so the abort is observable (§5.1 row 6, §5.2, §6-U4).
0f. **`form_data.rs:255` has two callers and one is wrong** — `build_form_data_from_form` (`:281`) backs the `FormDataEvent.formData` payload (`html_form_proto.rs:570`), where §4.10.22.3 constructs the entry list *with* the submitter. Decided explicitly: **widen** `#11-submitter-entry-tree-order` to cover membership as well as ordering (§10), rather than fix it here — it is the same append-pass rewrite.
0g. **Verified, and it changes the shape of part 4**: §4.10.21.2's steps split **1-4 pure / 5-7 event-firing** (`body html statically-validate-the-constraints`), step 6.1 fires `invalid` **cancelable** and 6.2 keeps only non-canceled controls. Today's loop interleaves them and discards `notCanceled` — two live bugs, now deltas 12-13 with tests U8/U9.
0h. **`CollectionFilter::FormControls` is the load-bearing dependency for rows 6-7** — it resolves via `matches_tag_ascii_ci_listed` (`live_collection/mod.rs:512`, doc `:549` = the HTML §4.10.2 *listed* elements), which includes `"button"`. Named in §3 so the dependency is visible.

**Round 2 — corrections to the previous brief.**

1. **The sweep is 52 sites, not "~20"** (verified 2026-07-26). Per-pattern counts now in **PR-A0 §4.1**. The brief's site list omitted `globals.rs:287`, `well_known.rs:1145`, `form-core/lib.rs:424`, 3 of the 8 `validity_state.rs` hits, 2 of the 7 `html_form_proto.rs` hits, and all 9 `tests_validity_state.rs` / `tests_html_form_proto.rs` test-comment hits.
2. **A whole additional drift cluster was unnamed: `§4.10.21.4` (10 sites, verified 2026-07-26).** It resolves to **"Security"**; `requestSubmit()` is **§4.10.3 The form element** `#dom-form-requestsubmit`. Its sites include `submit.rs:276`/`:289` — the very docstring the brief asked me to correct for the image-button lie.
3. **"Submit button" is defined in §4.10.2, not §4.10.6.** `dfn html 'submit button'` → `#concept-submit-button` in §4.10.2 Categories; §4.10.6 defines the button `type` *state* and states the bar.
4. **§4.10.6's antecedent is a disjunction, not "three conjuncts".** Two bullets: (i) Auto state ∧ no `command`/`commandfor` ∧ parent not `<select>`; **or** (ii) Submit Button state. `<button type=submit>` matches (ii) unconditionally; only the Auto state goes through (i)'s three conjuncts. §4.4 quotes it in full.
5. **The `§4.10.15.2` pattern is not one concept** (verified 2026-07-26). 4 of its 5 sites are `:required`/`:optional` *matching* cites in `elidex-css/selector/` (Slice-1-owned); only `validation/mod.rs:227` is the `minlength` cite (→ §4.10.5.3.1). Hence the §4.5 boundary rule and Q7.
6. **The renumber-sweep memo queues a third site, not two.** Besides `validation/mod.rs:210` and `validation/tests.rs:478`, it queues `tests_html_input_proto.rs:176` (§4.10.5.1.16 → §4.10.5 `#dom-input-indeterminate`) — a different concept in a file PR-A does not touch, so it stays out under the boundary rule.
7. **`native_form_request_submit` starts at `:499`, not `:537`** — `:537-556` is its validation block, which is what the brief meant and what §5.1 row 6 cites.
8. **The shell is an indirect consumer.** `content/form_input.rs:151` calls `build_form_submission`, not `collect_form_data`. The two *direct* `collect_form_data` consumers are `build_form_submission` and `form_data.rs:255`; the shell inherits through the former. §4.3 states the chain rather than a flat list of three peers.
9. **Confirmed as briefed** (recorded so the verification is auditable): `submit_form` does not exist (`build_form_submission` at `:136`, append at `:158-180`); `validity_state.rs`'s § number is at `:152` with the bar-list prose at `:154`; `submit.rs:322` is a form-owner-association claim → §4.10.18.3; the `submit.rs:276-279` image-button docstring is false; `commandfor` = 0 hits; layout lane = 16 commits with PR #488 open; `submit_tests.rs:307`/`:504` would have been duplicated by the old S1; `run_form_check_validity` at `:620-654`; `elidex-form` exports no form-level validation entry point; §4.10.8's datalist bar prose; MEMORY.md line 62 and umbrella §10:252 both stale.
