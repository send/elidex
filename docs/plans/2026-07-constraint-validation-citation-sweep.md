# Plan — constraint-validation + form-submission citation sweep (doc-only, PR-A0)

## §0 Status

**Lane**: DOM/form (L3). **Branch**: `domform-submittable-category`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-submittable`.
**Base**: `f7d9b5ce` (`origin/main`). **Lands FIRST**, before PR-A.
**Blocks**: `docs/plans/2026-07-form-submittable-category-repair.md` (PR-A) → `docs/plans/2026-07-form-elementstate-cache-deletion-slice-1.md` (Slice 1).
**Status**: plan-memo draft — `/elidex-plan-review` before implementation.
**Nature**: **doc-only**. Zero behavior change, zero production logic touched. Precedent = #483 (`9fe42b48`), a doc-only CORS-preflight citation sweep landed as its own PR.

**Why it is its own PR rather than a rider on PR-A.** It was a rider until the Step-4.5 re-check proved the sweep needs its own *method*: the original probe asked "does `heading --exact` fail?", which is structurally blind to a **valid-but-misused** section number. That blindness hid a 19-site cluster (§4.5). A sweep whose detector is wrong is not a rider — it is a design problem, and it gets a PR.

---

## §0.5 Spec citation table

All pairs looked up with `.claude/tools/webref` on **2026-07-26**. These are the *targets*; §4 is the full site accounting.

| Concept | § | Exact title | Anchor | webref command |
|---|---|---|---|---|
| candidate for constraint validation / barred / satisfies its constraints | HTML §4.10.21.1 | Definitions | `#candidate-for-constraint-validation` / `#concept-fv-valid` | `dfn html 'candidate for constraint validation'` |
| **statically / interactively validate the constraints** | HTML §4.10.21.2 | Constraint validation | `#statically-validate-the-constraints` / `#interactively-validate-the-constraints` | `heading --exact html 4.10.21.2`; `body html statically-validate-the-constraints` |
| control-level CVA (`willValidate` / `checkValidity()` / `reportValidity()` / `validationMessage` / `setCustomValidity()` / `ValidityState`) | HTML §4.10.21.3 | The constraint validation API | `#dom-cva-willvalidate` etc. | `dfn html 'willValidate'` / `'ValidityState'` |
| form-level `checkValidity()` / `reportValidity()` / `requestSubmit()` / `submit()` / `reset()` | HTML §4.10.3 | The form element | `#dom-form-requestsubmit` / `#dom-form-reset` | `dfn html 'requestSubmit'` / `'checkValidity'` |
| **form submission algorithm** (steps 5 / 5.4 / 5.6 / 7) | HTML §4.10.22.3 | Form submission algorithm | `#form-submission-algorithm` | `heading --exact html 4.10.22.3` |
| constructing the entry list / the `formdata` event | HTML §4.10.22.4 | Constructing the entry list | `#constructing-form-data-set` | `heading --exact html 4.10.22.4` |
| SubmitEvent | HTML §4.10.22.10 | The SubmitEvent interface | `#the-submitevent-interface` | `heading --exact html 4.10.22.10` |
| FormDataEvent | HTML §4.10.22.11 | The FormDataEvent interface | `#the-formdataevent-interface` | `heading --exact html 4.10.22.11` |
| resetting a form | HTML §4.10.23 | Resetting a form | `#resetting-a-form` | `heading --exact html 4.10.23` |
| association of controls and forms | HTML §4.10.18.3 | Association of controls and forms | `#association-of-controls-and-forms` | `heading --exact html 4.10.18.3` |
| `minlength` — the **shared** input+textarea dfn | HTML §4.10.19.4 | Setting minimum input length requirements: the minlength attribute | `#attr-fe-minlength` | `heading --exact html 4.10.19.4` |
| `<datalist>` (incl. its constraint-validation bar) | HTML §4.10.8 | The datalist element | `#the-datalist-element` | `heading --exact html 4.10.8` |
| textarea (no `pattern` support) | HTML §4.10.11 | The textarea element | `#the-textarea-element` | `heading --exact html 4.10.11` |
| `<input type=hidden>` bar | HTML §4.10.5.1.1 | Hidden state (type=hidden) | `#hidden-state-(type=hidden)` | `heading --exact html 4.10.5.1.1` |

**Verbatim prose fixing the two step-number families** (`body html statically-validate-the-constraints`):

> …return either a positive result … or a negative result … along with a (possibly empty) list of elements that are invalid and for which no script has claimed responsibility:
> 1. Let *controls* be a list of all the submittable elements whose form owner is *form*, in tree order.
> 2. Let *invalid controls* be an initially empty list of elements.
> 3. For each element *field* in *controls*, in tree order: 1. If *field* is not a candidate for constraint validation, then move on… 2. Otherwise, if *field* satisfies its constraints, then move on… 3. Otherwise, add *field* to *invalid controls*.
> 4. If *invalid controls* is empty, then return a positive result.
> 5. Let *unhandled invalid controls* be an initially empty list of elements.
> 6. For each element *field* in *invalid controls*… 1. Let *notCanceled* be the result of firing an event named `invalid` at *field*, with the **cancelable** attribute initialized to true. 2. If *notCanceled* is true, then add *field* to *unhandled invalid controls*.
> 7. Return a negative result with the list of elements in the *unhandled invalid controls* list.

---

## §1 Ideal anchor — the sweep's substance is DISCOVERY, and discovery must be exhaustive by construction

`§X.Y = <title>` in a comment is a checkable assertion. Repairing one is mechanical. **Finding all of them is not**, and that is what this PR is actually for.

**Two generations of the same defect.** The R1 probe asked *"does `heading --exact` fail?"* — a detector blind to a **valid-but-misused** number. The first draft of this memo diagnosed that correctly and then reproduced the identical shape one level up: its §3.1 procedure began *after* a candidate set existed, and supplied that set as a **hand-authored 7-pattern alternation**. Enumeration-only-by-known-pattern is the same failure as resolution-only-by-known-outcome: both can only find what the author already suspected.

Measured, the gap is not marginal. **81 distinct `§4.10.x` numbers across 632 cites in `crates/`** (§4.0); the hand-authored union covered 52 cites. Whole crates were invisible — `crates/shell/` was matched by not a single pattern.

**The corrected shape**: discovery is step **(0)**, it is a total enumeration of the space, and every number it returns is dispositioned. §4 is re-derived from its output rather than from a pattern list.

---

## §2 Coupled invariants

**Edge-density judgment, restated honestly.** The first draft called this "not edge-dense" because "there is one canonical algorithm (resolve → compare → replace)". That is true of **repair** and false of **discovery** — and discovery is this PR's substance (§1). There is no canonical algorithm for "find every mis-cited section in a concept cluster"; the last two attempts each shipped a detector that could not see its own blind spot. So: **the repair half is mechanical; the discovery half is the design**, and §2 exists for the latter.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Concept-scoped completeness** — the unit is the *concept cluster*, not the file and not the pattern. Partial repair across semantic siblings is what #483 was forced to complete by its external reviewer ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]). | One inclusion rule, applied uniformly (§4.5). The first draft stated a concept rule in I1 and a **file** rule in its §4.4 — that rule-switch is the mechanism behind the misses. |
| **I2** | **Total disposition** — every number discovery returns is IN or OUT-with-a-named-owner. Silence is the failure mode. | §4.0's table publishes all 81; §4.5 dispositions all 25 non-resolving numbers and every misuse found. |
| **I3** | **Successor-PR fate** — some cites annotate code that PR-A / Slice 1 rewrite. | Falsifiable per site (§4.6), and the criterion must cover *rewritten*, not only *deleted*. |
| **I4** | **Detector completeness** — a cite can be wrong without a `§` on the line (bare "step N") or wrong while resolving cleanly. | Discovery needs three detectors, not one (§3.1). |

**Pairwise intersections (all 6):**

| Pair | Intersection |
|---|---|
| I1 × I2 | Concept-scoping is what makes total disposition *affordable* — 632 cites cannot all be swept, but all 81 numbers can be classified, so "out of cluster" becomes an owned verdict instead of an omission. |
| I1 × I3 | Both are exclusion rules and they must not overlap: cluster-membership decides *whether* a site is ours, fate decides *whether it is ours now*. §4.5 applies them in that order, once. |
| I1 × I4 | The bare-"step N" detector only makes sense within a concept scope — repo-wide it would match every numbered list. Scoping is what makes the weakest detector usable. |
| I2 × I3 | A site handed to a successor is dispositioned, not silent — §4.6's table names it and §11 writes the reciprocal row, so neither memo can drop it. |
| I2 × I4 | Total disposition is only as total as the weakest detector: a number that resolves (misuse) or a line with no number (bare step) would otherwise never enter the table to be dispositioned. |
| I3 × I4 | Fate must be evaluated against the successor's *actual* diff, including comments the successor re-authors — which is exactly the class the "deleted"-only wording missed (§4.6). |

---

## §3. Spec coverage map

Doc-only: no algorithm branch is implemented or altered. Each row is a **citation target** the sweep introduces or corrects.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21.1 Definitions | candidate / barred / satisfies constraints | candidacy vocabulary | the `§4.10.20.3` family — **per-site table in §4.2**, not blanket-mapped | ✓ (per-site) | no (doc-only) |
| WHATWG HTML §4.10.21.2 Constraint validation | statically validate | the pure walk | `html_form_proto.rs:613`, `well_known.rs:1145` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | interactively validate | the `requestSubmit` validation arm | `html_form_proto.rs:537`, `:538` — **both are *interactively* validate**, not statically (row corrected) | ✓ | no |
| WHATWG HTML §4.10.21.3 The constraint validation API | control-level CVA members | `willValidate` / `checkValidity` / `validationMessage` / `setCustomValidity` / `ValidityState` | `validity_state.rs:475` (**control-level**, row corrected), `vm/globals.rs:287` (**CVA-mixin install**, row corrected); `validation/mod.rs:188` already correct — stays | ✓ | no |
| WHATWG HTML §4.10.3 The form element | form-level members | `form.checkValidity()` / `reportValidity()` / `requestSubmit()` / `submit()` / `reset()` | the `§4.10.21.4` + `§4.10.21.5` families (§4.2) | ✓ | no |
| WHATWG HTML §4.10.3 The form element | `requestSubmit()` **step numbers** | steps 1 / 1.1 / 1.2 — in-tree "step 2 / 2.1 / 2.2" is stale, **including 3 lines with no `§` at all** (10 sites, verified 2026-07-27) | `submit.rs:276`/`:289`, `submit_tests.rs:663`, `tests_form_submit.rs:14`/`:26`/`:131`/`:200`, **`html_form_proto.rs:484`/`:486`/`:508`** | ✓ (10 sites) | no |
| WHATWG HTML §4.10.22.3 Form submission algorithm | steps 5 / 5.4 / 5.6 / 7 | the submission algorithm — what 16 of the 19 `§4.10.21.3` sites mean; in-tree "step 8.4 / 8.5" is stale | per-site table, §4.2 | ✓ (16 sites, verified 2026-07-27) | no |
| WHATWG HTML §4.10.22.4 Constructing the entry list | entry-list construction + the `formdata` event | `submit.rs:306`; `html_form_proto.rs:33` (formdata event — **not** §4.10.3) | ✓ | no |
| WHATWG HTML §4.10.22.10 The SubmitEvent interface | interface | `vm/mod.rs:1496`, **`vm/host/events_misc.rs:306`** (byte-twin, previously unnamed) | ✓ | no |
| WHATWG HTML §4.10.22.11 The FormDataEvent interface | interface | `vm/mod.rs:1500`, **`vm/host/events_misc.rs:343`** (byte-twin) | ✓ | no |
| WHATWG HTML §4.10.23 Resetting a form | reset algorithm | `submit.rs:457`, `html_form_proto.rs:576`, **`crates/shell/elidex-shell/src/content/form_input.rs:289`** (`§4.10.15.4`, previously invisible — no pattern reached `crates/shell/`) | ✓ | no |
| WHATWG HTML §4.10.12 The output element | `<output>` reset + `<output>` is a listed form-control | **`submit.rs:459`/`:486`** (cite §4.10.13 = *progress*), **`vm/globals.rs:380`** (cites §4.10.13), `validity_state.rs:299` (cites §4.10.10.1) — **one claim, two different wrong numbers** | ✓ | no |
| WHATWG HTML §4.10.18.3 Association of controls and forms | `form=` IDREF association | `submit.rs:322` (currently `§4.10.15.3`) | ✓ | no |
| WHATWG HTML §4.10.19.4 Setting minimum input length requirements: the minlength attribute | the shared `for=input,textarea` dfn | `validation/mod.rs:227` — serves TextArea too, so **not** §4.10.5.3.1 | ✓ | no |
| WHATWG HTML §4.10.11 The textarea element | textarea does not support `pattern` | `validation/mod.rs:210`, `validation/tests.rs:478` | ✓ | no |
| WHATWG HTML §4.10.8 The datalist element | datalist-ancestor bar | new cite consumed by PR-A part 3 | ✓ | no |
| WHATWG HTML §4.10.5.1.1 Hidden state (type=hidden) | `<input type=hidden>` bar | `validation/mod.rs:32` — currently uncited | ✓ | no |
| WHATWG HTML §4.10.5.3.4 The required attribute | select `valueMissing` | `validation/mod.rs:368` (`§4.10.5.3.11`, non-resolving) | ✓ | no |

**Breadth**: K=1 spec (WHATWG HTML), M=18 rows → **single-PR scope**. Doc-only, one concept cluster, zero behavior surface.

### §3.1 User-input touch audit + the discovery method

**Exposure: none.** Comments and doc-comments only.

The method is this PR's substance, so it is stated as a procedure with **discovery as step (0)**:

**(0) Total enumeration** — the candidate set is *derived*, never authored:
```
grep -rhoE '§4\.10\.[0-9]+(\.[0-9]+)*' crates/ --include='*.rs' | sort -u \
  | while read s; do .claude/tools/webref heading --exact html "${s#§}"; done
```
**(1) Three detectors**, because one is provably insufficient:
- **(a) non-resolution** — `heading --exact` returns no match;
- **(b) misuse** — the number resolves, but its **title** does not match the concept the surrounding comment describes (`webref dfn html '<concept>'` gives the section that actually defines it);
- **(c) `§`-less step drift** — a bare "step N" near a cluster concept, which detectors (a) and (b) never see because there is no number on the line.

**(2) Repair** number **and** prose, then re-check every **step** number against `body html <anchor>` — steps drift independently of sections.

**(3) Disposition every number** from (0): IN (cluster) or OUT with a named owner (§4.5). Silence is the failure mode (I2).

---

## §4 Site accounting

### §4.0 The evidence base — all 81 numbers

Run 2026-07-27 with the §3.1 step-(0) command: **81 distinct `§4.10.x` numbers**, **632 cites** in `crates/`. **25 numbers do not resolve**, carrying **96 cites** (the "96" is a *cite* count, not a number count — the two are easy to conflate and the earlier framing did).

**The 25 non-resolving numbers, with cite counts and disposition:**

| Number | Cites | Concept it annotates | Disposition |
|---|---|---|---|
| `§4.10.10.2` | 21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | **OUT** — select/option-collection surface → §10 queue |
| `§4.10.20.3` | 20 | candidate for constraint validation / barred | **IN** → §4.10.21.1 |
| `§4.10.20.4` | 9 | form-level `checkValidity` / `invalid` event | **IN** → §4.10.3 / §4.10.21.2 |
| `§4.10.18.7` | 4 | 2× `<output>`-always-valid (CSS), 2× `form.autocomplete` | **SPLIT** (verified 2026-07-27) — the 2 CSS sites → §4.6 fate table; the 2 form-attribute sites **OUT** → §10 queue |
| `§4.10.18.5` | 4 | "actually disableable" (`:disabled`/`:enabled` scope) | **IN by concept**, but all 4 → §4.6 fate table (Slice-1 code) |
| `§4.10.15.2.4` | 4 | `:required`/`:optional` matching | **IN by concept**, all 4 → §4.6 fate table |
| `§4.10.7.5` | 3 | `HTMLSelectElement.add()` | **OUT** → §10 queue |
| `§4.10.7.4` | 3 | `select.options` live collection | **OUT** → §10 queue |
| `§4.10.20.2` | 3 | `validationMessage` / custom validity | **IN** → §4.10.21.3 |
| `§4.10.18.6` | 3 | `form.method` / `form.enctype` | **OUT** — form content attributes, not validation → §10 queue |
| `§4.10.7.3.1` | 2 | textarea does not support `pattern` | **IN** → §4.10.11 |
| `§4.10.21.5.5` | 2 | SubmitEvent | **IN** → §4.10.22.10 |
| `§4.10.21.5.4` | 2 | FormDataEvent | **IN** → §4.10.22.11 |
| `§4.10.21.5` | 2 | `form.reset()` / `reset_form` | **IN** → §4.10.3 `#dom-form-reset` / §4.10.23 |
| `§4.10.15.3` | 2 | entry collection + `form=` association | **IN** → §4.10.22.4 / §4.10.18.3 |
| `§4.10.11.1` | 2 | textarea dirty-value flag | **OUT** — value model → §10 queue |
| `§4.10.10.1` | 2 | 1× `<output>` is a listed form-control, 1× option text | **SPLIT** — `validity_state.rs:299` **IN** → §4.10.12; `html_option_proto.rs:21` **OUT** |
| `§4.10.5.3.11` | 1 | select `valueMissing` | **IN** → §4.10.5.3.4 |
| `§4.10.5.2.7` | 1 | `maxlength` enforcement | **IN** → §4.10.19.3-family (verify anchor at impl) |
| `§4.10.3.1` | 1 | `form.elements` wrapper cache | **OUT** — collections → §10 queue |
| `§4.10.18.4` | 1 | `HTMLFormControlsCollection` | **OUT** — collections → §10 queue |
| `§4.10.15.4` | 1 | **shell** form reset | **IN** → §4.10.23 (previously invisible) |
| `§4.10.15.2` | 1 | `minlength` | **IN** → §4.10.19.4 |
| `§4.10.14.2` | 1 | `ValidityState` | **IN** → §4.10.21.3 |
| `§4.10.10.3` | 1 | disabled options in select | **OUT** → §10 queue |

**IN by non-resolution: 47 cites. OUT with a named owner: 42. Split across the fate table: 7.** Every one of the 25 numbers appears above; the remaining 56 numbers resolve and are checked by detector (b), which found the families in §4.2.

### §4.1 *(folded into §4.0 — the hand-authored pattern list is gone)*

The first draft's §4.1 published a 7-pattern union of 52 cites and misfiled 2 of its 7 rows: `§4.10.21.4` resolves to "Security" and bare `§4.10.15` resolves to "The fieldset element", so both are **misuse** (§4.2), not non-resolution — 16 lines in the wrong bucket. §4.0 supersedes it.

### §4.2 Misuse — numbers that resolve to the wrong concept (detector (b))

| Number | Cites | Resolves to | Means | Disposition |
|---|---|---|---|---|
| `§4.10.21.3` | 19 | The constraint validation API | **18 are mis-cited** (verified 2026-07-27); of those, **16 mean §4.10.22.3** (form submission algorithm). The other 2: `well_known.rs:1134` (`reset` event) → §4.10.23/§4.10.3; `html_form_proto.rs:537` → §4.10.21.2. `validation/mod.rs:188` is **correct and stays**. | IN |
| `§4.10.21.4` | 10 | **Security** | `requestSubmit()` → §4.10.3; **except `html_form_proto.rs:33`**, which is the **formdata event** → §4.10.22.4 | IN |
| `§4.10.13` (subset) | 3 | **The progress element** | `<output>`: `submit.rs:459`, `:486`, `vm/globals.rs:380` → **§4.10.12** | IN |
| bare `§4.10.15` | 1 | **The fieldset element** | `validation/mod.rs:1` module doc for *form validation* → §4.10.21.1 | IN |
| `§4.10.15` (meter subset) | 5 | **The fieldset element** | `<meter>` → §4.10.14: `vm/mod.rs:1089`, `html_meter_proto.rs:2`/`:5`/`:20`/`:97`. **`:20`'s "`<progress>` … §4.10.14 vs §4.10.15" contrast is inverted** (progress = §4.10.13, meter = §4.10.14) | **OUT** — meter surface → §10 queue |

**Per-site table for the `§4.10.20.*` family (31 lines / 32 cites)** — published rather than blanket-mapped, because the adjacent row proves a blanket map mis-routes. Target by concept: `validity_state.rs:2`/`:152`/`:182`/`:407`/`:441`/`:466` + `validation/mod.rs:12` + `lib.rs:176` + `validation/tests.rs:690` + the 5 `tests_validity_state.rs` sites → **§4.10.21.1** (candidacy) or **§4.10.21.3** (CVA members) per line; `validity_state.rs:475`/`:498`, `html_form_proto.rs:43`/`:608`/`:613`/`:634`, `well_known.rs:1145`, `vm/globals.rs:287`, `object_kind.rs:983`, `vm/mod.rs:1113`, `html_fieldset_proto.rs:28`, `lib.rs:424`, the 4 `tests_html_form_proto.rs` sites → **§4.10.21.2** (validate-the-constraints) or **§4.10.3** (form-level members) per line. Each line's verdict is recorded in the PR diff, not summarized here.

### §4.3 `§`-less step drift (detector (c))

`html_form_proto.rs:484`, `:486`, `:508` carry `requestSubmit`'s stale "Step 2 / 2.1 / 2.2" with **no `§` on the line**, so steps (0)-(b) never admit them. Correct numbering is **1 / 1.1 / 1.2**. Together with the 7 `§`-carrying sites (§3), the family is **10 sites** (verified 2026-07-27).

Likewise the "step 8.4 / 8.5" family (5 sites, verified 2026-07-27 — §4.2) — form-submission steps are **5 / 5.4 / 5.6 / 7**.

### §4.4 Two prose repairs that are not renumbers

1. **`submit.rs:276-279` states a falsehood**: `<input type=image>` is *not* "classified as `FormControlKind::SubmitButton` by `from_element`" — `from_type_str` (`crates/dom/elidex-form-core/src/lib.rs:253-282`) has no `"image"` arm, so it becomes `TextInput`. **Owned by PR-A0** (this claim was double-claimed with PR-A §10; PR-A's claim is removed and its §10 now points here).
2. **`validity_state.rs:152`/`:154`**: the § number is at `:152`, the bar list at `:154`. PR-A0 fixes the number **and** completes the list — it is currently missing the **`readonly`** bar that `is_constraint_validation_candidate` actually applies (`validation/mod.rs:40`), a pre-existing prose defect no other PR owns (PR-A only adds bars from its *own* new behavior). Alternative considered: collapse the duplicated list to a `[`is_constraint_validation_candidate`]` reference so it cannot drift again — §9-Q5.

### §4.5 Inclusion rule — ONE rule, stated once

> **A site is IN iff the concept its comment describes belongs to the constraint-validation + form-submission cluster.** Not its file, not its crate, not its pattern.

The first draft stated this in I1 and then used a **file**-shaped rule in its §4.4 ("cites in cluster files"). That switch is the mechanism behind the misses — it is why `crates/shell/elidex-shell/src/content/form_input.rs:289` (a form-reset cite, unambiguously in-cluster) was invisible: no cluster *file* was in `crates/shell/`.

Applying the one rule pulls in, previously unnamed: `vm/host/events_misc.rs:306`/`:343`; `crates/shell/elidex-shell/src/content/form_input.rs:289`; `submit.rs:459`/`:486`; `vm/globals.rs:380`; `validity_state.rs:299`; `validation/mod.rs:368`; `input.rs:715`; `html_form_proto.rs:484`/`:486`/`:508`. It pushes **out**, with owners: the meter family, `§4.10.10.2`, `§4.10.7.4`/`.5`/`§4.10.10.3`, `§4.10.18.6`, `§4.10.18.4`, `§4.10.3.1`, `§4.10.11.1`, `html_option_proto.rs:21` (§10).

### §4.6 Fate — for cites whose code a successor PR re-authors

> **A cite is excludable only when the successor PR deletes the code it annotates, *or* rewrites it such that the successor re-authors the comment. A cite that outlives the change is IN.**

("deleted" alone was too narrow — it returned OUT for `matching.rs:284`, which Slice 1 *rewrites*.) Verdicts against Slice-1's §5 deletion set and its §6-C citation list:

| Site | Cite | Annotates | Slice-1 fate | Verdict |
|---|---|---|---|---|
| `matching.rs:242` | §4.10.18.5 | `:disabled` arm comment | arm rewritten, comment re-authored | **OUT** — Slice 1 |
| `matching.rs:250` | §4.10.18.3 | `:indeterminate` arm | rewritten | **OUT** — Slice 1 |
| `matching.rs:256` | §4.10.18.7 | `<output>`-always-valid | **deleted** (the special case goes) | **OUT** — Slice 1 |
| `matching.rs:284` | §4.10.15.2.4 | `:required` arm | rewritten, re-authored | **OUT** — Slice 1 |
| `matching.rs:137` | §4.10.15.2.4 | `is_requirable_element` doc | **deleted** (§5-R4) | **OUT** — Slice 1 |
| `matching.rs:405`/`:419` | §4.10.18.3/.5 | `is_indeterminate_candidate` / `is_disableable_element` docs | **deleted** (§5-R4) | **OUT** — Slice 1 |
| `matching.rs:577` | §4.10.18.5 | inline test comment | test rewritten | **OUT** — Slice 1 |
| `tests/matching.rs:715` | §4.10.15.2.4 | `required_does_not_match_fieldset` — the *assertion* survives | body re-seeded; comment not re-authored | **IN** |
| `tests/matching.rs:727` | §4.10.15.2.4 | `optional_does_not_match_fieldset` | **Slice-1 §5 lists line 726 among the 16 it rewrites**, and its re-seed rule removes the `ElementState::default()` insert — so the *body* changes; the comment still outlives it | **IN** (verdict unchanged; **basis corrected** — the earlier "survives byte-identical" was wrong) |
| `tests/matching.rs:767` | §4.10.18.7 | `valid_matches_output_element` — Slice-1 §6-A row 9 **inverts** this test | comment re-authored | **OUT** — Slice 1 |
| `tests/matching.rs:788` | §4.10.18.5 | fieldset-not-disableable | Slice-1 §6-A rows 6-7 **invert** it | **OUT** — Slice 1 |
| `tests/matching.rs:839` | §4.10.18.3 | `:indeterminate` non-candidate | rewritten | **OUT** — Slice 1 |
| `validity_state.rs:152`/`:154` | §4.10.20.3 | the bar list | PR-A adds *its own* bars only | **IN** (number + the missing `readonly` bar, §4.4) |
| `validation/mod.rs:12` | §4.10.20.3 | candidacy doc | PR-A adds its own bars | **IN** for the number; PR-A owns the new-bar prose |

**⚠ Slice-1 §6-C is wrong about one site.** Its row reads `matching.rs:137` / `:284` / `:299`, but **line 299 carries no `§` at all** (`// :optional only matches input/select/textarea (same constraint as :required).`). Verified 2026-07-27. The reciprocal edit must **correct** that row, not mirror it.

**Reciprocal edits** (§11): write the per-site verdicts into **both** Slice-1 §6-C and **PR-A** §4.4, so no memo can silently drop a site.

**Also OUT with an owner**: `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — different concept, already queued in `memory/project_html-section-renumber-sweep.md`.

---

## §5 Behavior deltas

**None.** Comments and doc-comments only; no production statement, signature, or test assertion changes. Success = `mise run ci` green with a diff containing no non-comment line.

---

## §6 Test plan

There is nothing to assert — the deliverable is prose. Verification is procedural:

1. **`mise run ci` green**, including the `-D warnings` `doc` job (catches a broken intra-doc link introduced while editing doc-comments).
2. **Diff discipline**: `git diff -U0 | grep '^[+-]' | grep -v '^[+-][+-]' | grep -vE '^[+-]\s*(//|///|//!|\*)'` must be **empty** — the mechanical proof that the PR is doc-only.
3. **Citation re-verification**: every `§`-number introduced by the diff re-checked with `webref heading --exact`, and every **step** number re-checked with `body html <anchor>`.
4. **Exit criterion — negative space, not a pattern list.** Re-run §3.1 step (0) and assert that **every** distinct `§4.10.x` in `crates/` either resolves *or* appears on §4.0's named-OUT list. A pattern-shaped residual scan cannot state this: the first draft's scan was **narrower than its own audit grep** (it dropped `§4.10.15.2` and never had `§4.10.21.5` / `§4.10.18.x` / bare `§4.10.15`), so the PR could have passed its own exit criterion with the cluster half-swept. The negative-space form has no such gap by construction:
   ```
   grep -rhoE '§4\.10\.[0-9]+(\.[0-9]+)*' crates/ --include='*.rs' | sort -u \
     | while read s; do .claude/tools/webref heading --exact html "${s#§}" >/dev/null 2>&1 \
       || echo "UNRESOLVED $s"; done
   ```
   Every `UNRESOLVED` line must match §4.0's OUT list. Plus: no `§4.10.21.3` survives except `validation/mod.rs:188`, and no `§`-less "step 2/2.1/2.2" survives in the `requestSubmit` family.

**Gate**: `mise run ci`, then `/pre-push`, then **`/external-converge` from round 1**. The reactive escalation trigger ([[feedback_gate-miss-on-edge-dense-escalate-to-converge]]) has already fired twice on this memo — two gaps found pre-dispatch, then 1 CRIT / 28 IMP at review — and the #483 precedent this PR invokes did **not** converge in a single pass (it left three open carves). §2's honest re-statement (discovery is not canonical) is the same conclusion from the design side.

---

## §7 Layering check

- **Doc-only** — no code moves, no crate edge, no new `vm/host/` algorithm, no `elidex-ecs` touch. ✓
- The sweep **reinforces** the mandate indirectly: `validation/mod.rs:1`'s module doc stops claiming the fieldset section for a validation module, and `validity_state.rs:154`'s bar list stops describing a predicate it no longer matches. ✓
- **⚠ Not a clean bill, and PR-A0 is about to make it worse before PR-A makes it better.** `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) **is** a form-level validation algorithm living in `vm/host/`, and this PR is about to author a *spec-blessed* `§4.10.21.2` comment on it — i.e. to document drift as though it were design. Carried mitigations:
  - a **concept → engine-independent-API mapping** in the rewritten comments: §4.10.21.1 candidacy → form-core's `is_constraint_validation_candidate`; "satisfies its constraints" → form-core's `validate_control`; §4.10.21.2 steps 1-4 → **`elidex_form::statically_validate_constraints`** (NEW) — PR-A part 4;
  - an explicit **hand-off line** at `html_form_proto.rs:620` naming PR-A's export, so the comment reads as "steps 5-7 here, steps 1-4 land in `elidex-form` next PR" rather than as a settled home.
- **ECS-native**: N/A — no component read or written. ✓

---

## §8 Line-count budget

Verified 2026-07-27 (`wc -l`). The earlier version of this section was **false** — it omitted two named touch sites that are already >1000 and mis-stated a third.

| File | Lines | Note |
|---|---|---|
| `crates/script/elidex-js/src/vm/mod.rs` | **2782** | already >1000; touched (`:1089`, `:1094`, `:1105`, `:1113`, `:1496`, `:1500`) |
| `crates/script/elidex-js/src/vm/object_kind.rs` | **1700** | already >1000; touched (`:983`) |
| `crates/script/elidex-js/src/vm/well_known.rs` | **1588** | already >1000; touched (`:1134`, `:1136`, `:1140`, `:1145`) |
| `crates/script/elidex-js/src/vm/globals.rs` | **1533** | already >1000; touched (`:287`, `:380`) |
| `crates/script/elidex-js/src/vm/host/html_form_proto.rs` | 680 | |
| `crates/dom/elidex-form/src/submit.rs` | 546 | |
| `crates/script/elidex-js/src/vm/host/validity_state.rs` | 518 | |
| `crates/dom/elidex-form-core/src/validation/mod.rs` | 430 | |
| `crates/shell/elidex-shell/src/content/form_input.rs` | 318 | |
| `crates/script/elidex-js/src/vm/tests/tests_form_submit.rs` | **250** | (not 213) |

**Disposition — explicit, same shape as Slice-1 §9's `components.rs` (1037) row.** Four touched files are already >1000. The touch-time split discipline targets *growth into* unbounded files; PR-A0's diff is **comments only and net-flat** (a few multi-line comments reflow ±1 line), so it adds no debt and triggers no obligation. Splitting any of them would be a speculative refactor bundled into a doc-only PR, which the same discipline forbids. Recorded here so plan-review does not read the four >1000 counts as unaddressed.

---

## §9 Open questions for `/elidex-plan-review`

- **Q1 — is the §3.1 method sufficient?** Its three detectors catch non-resolution, misuse, and `§`-less step drift. It does **not** catch a cite that is right about the section but wrong about the **anchor** within it. **Recommendation: out of scope, with a slot** (§10 `#11-citation-anchor-level-verification`) — anchor checking needs a different tool path (`body html <anchor>` per site) and would roughly double the PR. Deferring it silently while Q1 stayed open was the earlier draft's inconsistency with its own "zero deferred work" claim.
- **Q2 — two adjacent boundaries, same shape.** (a) `§4.10.22.3` vs `§4.10.22.4`: `form_data.rs`'s "step 7 construct the entry list" is a step *of* §4.10.22.3 that *invokes* §4.10.22.4. (b) **`§4.10.21.2` vs `§4.10.22.3`**: the interactive-validation arm is §4.10.22.3 step 5.4, which invokes §4.10.21.2's algorithm — the same invoking-step-vs-invoked-algorithm question. Recommendation for both: cite the **invoking step** where the code implements the caller, and the **invoked algorithm** where it implements the algorithm. Confirm.
- **Q3 — §4.6's fate criterion**: applied per site above. Confirm the four verdicts, and confirm the reciprocal Slice-1 §6-C rows are the right coordination shape.
- **Q4 — scope floor**: §4.4 lists 9 further cites in cluster files that no blanket map covers. Confirm they are in scope (recommendation: yes — leaving known-wrong cites in a file this PR is already editing is exactly the partial repair I1 forbids).

---

## §10 Defer slots + the out-of-cluster inventory

| Slot | Status | Audit |
|---|---|---|
| **`#11-citation-anchor-level-verification`** | **NEW** (from §9-Q1) | (1) Real gap? yes — §3.1's detectors verify section *numbers* and concepts, not the `#anchor` within a section; a cite can name the right section and the wrong dfn. (2) Blocked by structure? no, but it needs a per-site `body html <anchor>` pass that would roughly double this PR. (3) Non-regressing to defer? yes — no cite gets *worse*. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the next citation sweep, or a review finding a wrong-anchor cite. **Date**: 2026-11. |

**Per-PR budget**: **1 new slot** (≤3 ✓). The earlier draft claimed "zero deferred work" while §9-Q1 was open — corrected.

### §10.1 Out-of-cluster inventory — a deliverable, not a silence

The §3.1 step-(0) enumeration surfaces drift the cluster rule (§4.5) pushes **out**. Sweeping it here would collide with every active lane; leaving it unrecorded would convert a measured gap into a silent one. It is therefore filed as **new queue entries in `memory/project_html-section-renumber-sweep.md`**, which is precisely the tracked program that exists to hold it:

| Family | Cites | What it is | Correct target |
|---|---|---|---|
| `§4.10.10.2` | 21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | §4.10.10 (The option element) / §4.10.7 (The select element) per site |
| `<meter>` citing `§4.10.15` | 5 (verified 2026-07-27) | `vm/mod.rs:1089`, `html_meter_proto.rs:2`/`:5`/`:20`/`:97` — §4.10.15 is "The fieldset element" | **§4.10.14 The meter element**. ⚠ `html_meter_proto.rs:20`'s "`<progress>` has one — §4.10.14 vs §4.10.15" contrast is **inverted**: progress = §4.10.13, meter = §4.10.14 |
| `§4.10.7.4` / `§4.10.7.5` / `§4.10.10.3` | 7 | select `value` setter, `select.add()`, disabled options | §4.10.7 (select) subsections |
| `§4.10.18.6` / `§4.10.18.7` (form-attribute subset) | 5 | `form.method` / `enctype` / `autocomplete` | §4.10.3 (The form element) |
| `§4.10.18.4` / `§4.10.3.1` | 2 | `HTMLFormControlsCollection`, `form.elements` wrapper cache | §4.10.3 / the collections surface |
| `§4.10.11.1` | 2 | textarea dirty-value flag | §4.10.11 (The textarea element) |
| `§4.10.10.1` (option subset) | 1 | `html_option_proto.rs:21` option text | §4.10.10 |

**Registration shape** — follow that memo's established `## ✅ DONE — #N ✅ MERGED (sha, date, comment-only)` heading for the discharged part, and add the table above as a new open queue block. **Per-bullet discharge, not a blanket claim:**
- **Fully discharged**: the two textarea/`pattern` sites (`validation/mod.rs:210`, `validation/tests.rs:478`) that the memo's 2026-07-18 Slice-0b entry queued as item (a).
- **Not discharged**: that same entry's item (b), `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — out of cluster, stays queued.
- **Partially discharged, residue restated**: the constraint-validation + form-submission cluster is swept; the residue is §4.6's OUT list (the Slice-1-owned `elidex-css/selector/` sites) plus §10.1's out-of-cluster families.

*(The earlier draft quoted that memo as saying "the next PR that touches those files" — **that phrasing does not appear in it**. Paraphrased here instead; the actual entry is dated 2026-07-18 and marks the two sites pre-existing and diff-external to #479.)*

---

## §11 Coordination

Re-derived at authoring time 2026-07-27 (`git worktree list`; `git rev-list --count origin/main..<branch>`; `gh pr list --state open`) — **not recalled**. This table was wrong in two prior rounds; the counts below are freshly measured.

| Lane | Branch | Commits ahead | PR | Overlap | Ordering |
|---|---|---|---|---|---|
| **PR-A0** (this) | `domform-submittable-category` | 1 | — | — | lands first |
| **PR-A** | `domform-submittable-category` (same branch, stacked) | 1 | — | PR-A0 renumbers comments in every file PR-A edits | **PR-A0 first**; PR-A writes prose onto corrected numbers |
| **Slice 1** | `domform-slice1` | 1 | — | the `elidex-css/selector/` sites, dispositioned per site in §4.6 | Slice 1 keeps its OUT set; §4.6 reciprocal rows correct its §6-C |
| **VM lane** | `vm-p4-plan` | **7** | — | PR-A0 edits comments in `vm/mod.rs`, `well_known.rs`, `vm/globals.rs`, `object_kind.rs`, `vm/host/*` | doc-only → textual conflict risk only; PR-A0 lands first |
| **Layout** | `terminal-z-c3a-impl` | **17** | **#488** open | none | none |
| **Layout plan** | `terminal-z-c3-plan` | **8**, pushed | — | none | none |
| **History** | `session-history-slice-b` | **22** | **#487** open | none | none |

⚠ **New this round**: PR-A0 now touches `crates/shell/elidex-shell/src/content/form_input.rs:289` (§4.5). The **History lane** (`session-history-slice-b`, 22 commits, PR #487) is the shell lane — verify at rebase that its diff does not reach `content/form_input.rs`. Doc-only either way, so the risk is a textual conflict, not a semantic one.

**Carried edits:**
1. **`memory/project_html-section-renumber-sweep.md`** — per-bullet discharge + the §10.1 out-of-cluster inventory (§10).
2. **Slice-1 §6-C** — reciprocal per-site rows from §4.6, **including the correction** that `matching.rs:299` carries no `§` (verified 2026-07-27).
3. **PR-A §4.4** — reciprocal rows for the two sites PR-A0 now owns outright (`submit.rs:276-279`, `validity_state.rs:152`/`:154`), and removal of PR-A's duplicate claim on the first.
