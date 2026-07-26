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

**Verbatim prose — the §4.10.21.2 algorithm split** (`body html statically-validate-the-constraints`). ⚠ *Provenance note*: draft 2 captioned this block as evidence for the two **step-number** families; it is not — those verify against `#dom-form-requestsubmit` and `#form-submission-algorithm` respectively (both independently confirmed 2026-07-27). This block evidences the steps-1-4 / 5-7 split that §7 and PR-A part 4 rest on:

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

**Three generations of the same defect.** Each fix moved the blind spot up one altitude rather than removing it:

| # | Detector | Blind to |
|---|---|---|
| R1 probe | resolution-only-by-known-**outcome** | a number that resolves to the *wrong concept* |
| draft 1 | enumeration-only-by-known-**pattern** | anything the hand-authored alternation did not match |
| draft 2 | triage-only-by-known-**suspicion** | misuse the author had not already guessed — detector (a) got a published total evidence base; (b) and (c) did not |

Draft 2's proof is a **systematic off-by-one cascade across four adjacent element sections** that it never named (§4.2): `<output>` cited as §4.10.13 (**9 of 9 wrong**), `<progress>` as §4.10.14 (**9 of 9**), `<meter>` as §4.10.15 (**11 of 16**), and §4.10.10 used for `<select>`/`<datalist>` (**8 wrong**). It dispositioned 8 of ~37 and wrote "(subset)" on a row where **no correct sibling exists**.

**The principle that ends the regress: triage is necessarily total; repair is scoped.** A cite's concept is knowable only by reading it, so the misuse class cannot be bounded by concept *a priori* — only the **repair** set can. §4.0 already did this for the 25 non-resolving numbers; §4.2 now does it for all 82.

**And the root fix is a tool, not a paragraph.** Prose cannot hold a detector: every draft's discovery step decayed into the author's suspicion. `.claude/tools/webref cite-audit` (NEW, §3.1) emits every distinct `§number` cited in a tree, its resolved title, and **every cite line with its text** — the complete candidate set *with the context needed to classify it*. The script owns discovery; the memo owns classification; §6's exit criterion re-runs the script.

**The corrected shape**: discovery is step **(0)**, it is a total enumeration of the space, and every number it returns is dispositioned. §4 is re-derived from its output rather than from a pattern list.

---

## §2 Coupled invariants

**Edge-density judgment, restated honestly.** The first draft called this "not edge-dense" because "there is one canonical algorithm (resolve → compare → replace)". That is true of **repair** and false of **discovery** — and discovery is this PR's substance (§1). There is no canonical algorithm for "find every mis-cited section in a concept cluster"; the last two attempts each shipped a detector that could not see its own blind spot. So: **the repair half is mechanical; the discovery half is the design**, and §2 exists for the latter.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Concept-scoped completeness** — the unit is the *concept cluster*, not the file and not the pattern. Partial repair across semantic siblings is what #483 was forced to complete by its external reviewer ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]). | One inclusion rule, applied uniformly (§4.5). The first draft stated a concept rule in I1 and a **file** rule in its §4.4 — that rule-switch is the mechanism behind the misses. |
| **I2** | **Total disposition** — every number discovery returns is IN or OUT-with-a-named-owner. Silence is the failure mode. | §4.0's table publishes all 81; §4.5 dispositions all 25 non-resolving numbers and every misuse found. |
| **I3** | **Successor-PR fate** — some cites annotate code that PR-A / Slice 1 rewrite. | Falsifiable per site (§4.6), and the criterion must cover *rewritten*, not only *deleted*. |
| **I4** | **Detector completeness** — a cite can be wrong without a `§` on the line (bare "step N"), wrong while resolving cleanly (misuse), or invisible to a hand-written regex. | Discovery needs three detectors **and a checked-in implementation** — prose decayed three times (§1). `cite-audit` (NEW) owns detector (0)+(a); (b) and (c) stay author-judgment over the script's output. |

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
| WHATWG HTML §4.10.7 The select element | select `required` / `valueMissing` (`#attr-select-required`) | `validation/mod.rs:368` (`§4.10.5.3.11`, non-resolving). **Not §4.10.5.3.4** — that is the *input* attribute | ✓ | no |

**Breadth**: K=1 spec (WHATWG HTML), M=18 rows → **single-PR scope**. Doc-only, one concept cluster, zero behavior surface.

### §3.1 User-input touch audit + the discovery method

**Exposure: none.** Comments and doc-comments only.

The method is this PR's substance, so it is stated as a procedure with **discovery as step (0)**:

**(0) Total enumeration — a checked-in detector, not a grep** (`.claude/tools/webref cite-audit` (NEW), an *adapter command* per `_webref/DESIGN.md`'s "elidex policy in adapter commands"; the generic half is `resolver.lookup_section`, untouched):
```
.claude/tools/webref cite-audit html --prefix 4.10          # full: number + title + every cite line
.claude/tools/webref cite-audit html --prefix 4.10 --summary  # section lines only
.claude/tools/webref cite-audit html --strict               # exit 1 if any cited section fails to resolve
```
It emits, per distinct number: the resolved title (or `*** UNRESOLVED ***`), the cite count, and **every `file:line` with its comment text** — which is what makes detector (b) *executable over a total set* rather than over a suspicion.

**Why the previous hand-rolled grep was insufficient is now measurable**: draft 2's regex `§4\.10\.[0-9]+(\.[0-9]+)*` required a digit after `4.10.` and therefore **could not see bare `§4.10`** — **99 cites**, the single largest family in the tree. The script found it on first run. That is the hand-authored class failing one more time, and the reason it is now a tool.

**(1) Three detectors**, because one is provably insufficient:
- **(a) non-resolution** — the script's `*** UNRESOLVED ***` rows;
- **(b) misuse** — the number resolves, but its **title** does not match the concept the cite text describes. Now applied to **all 82** numbers, not a suspected subset;
- **(c) `§`-less step drift** — a bare "step N" near a cluster concept, invisible to (a) and (b) because there is no number on the line.

**(2) Repair** number **and** prose, then re-check every **step** number against `body html <anchor>` — steps drift independently of sections.

**(3) Disposition every number** from (0): IN (cluster) or OUT with a named owner (§4.5). Silence is the failure mode (I2).

---

## §4 Site accounting

### §4.0 The evidence base — all 81 numbers

Run 2026-07-27 via `.claude/tools/webref cite-audit html --prefix 4.10`.

**Attribution matters, and the raw numbers are not the audit set.** The tool's first cut resolved *every* bare `§N.N` against HTML, which reports **correct work as drift**: `§2.2.2` (37 cites in `crates/net/elidex-net/src/preflight/`) is WHATWG **Fetch** — the citations PR #483 deliberately swept *to* their correct Fetch numbers — and `§0.5` (4 cites, e.g. `vm/dispatch_class.rs:6`) is a **plan-memo** pointer, not a spec citation at all. The tool now attributes each cite to a spec in three buckets (§3.1) before resolving. Whole-tree, the difference is an order of magnitude:

| Measure | Unattributed (first cut) | **Attributed (authoritative)** |
|---|---|---|
| `crates/` distinct sections / cites | 1257 / 11984 | **405 / 2390** (html) |
| unresolved sections / cites | 710 / 3763 | **61 / 171** (html) |
| attributed to another spec | — | 2663 cites |
| **UNATTRIBUTED** | — | **6931 cites** |

**The `§4.10` cluster, attributed to HTML**: **74 distinct sections / 497 cites**; **24 unresolved / 92 cites**.

⚠ **Three corrections these supersede.** (1) Draft 2's 81 / 632 came from a grep that required a digit after `4.10.`, so it missed **bare `§4.10` (99 cites)**. (2) Draft 3's 82 / 731 was right about the *space* but counted unattributed cites as HTML. (3) The audit set is therefore **74 / 497**, and the 25-number table below is 24 numbers under attribution — the one that drops out is cited only from comment blocks that never name a spec.

**UNATTRIBUTED is a finding, not noise.** 6931 cites tree-wide (234 within `§4.10.x`) carry a bare `§` whose spec a reader cannot determine from its comment block. That is a defect *independent of whether the number happens to resolve*, and it is the largest single citation-hygiene gap the tool exposes. **Out of scope here** (it spans every spec and every crate) → queued in §10.1 as `#11-unattributed-spec-citations`.

**The 25 non-resolving numbers, with cite counts and disposition:**

| Number | Cites | Concept it annotates | Disposition |
|---|---|---|---|
| `§4.10.10.2` | 21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | **OUT** — select/option-collection surface → §10 queue |
| `§4.10.20.3` | 20 | candidacy / barred **and** CVA members **and** ValidityState | **IN, SPLIT** — §4.10.21.1 (barred/candidacy) / §4.10.21.3 (CVA members + `ValidityState` interface) per site; the cites straddle both. Per-site verdicts in the PR diff. |
| `§4.10.20.4` | 9 | form-level `checkValidity` / `reportValidity` / `invalid` event | **IN, SPLIT** — §4.10.3 (the form-level members) / §4.10.21.2 (the validate-the-constraints algorithm they invoke) / §4.10.21.3 (control-level `checkValidity`) per site. **Also carries a third stale-step family** (§4.3). |
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
| `§4.10.5.3.11` | 1 | select `valueMissing` | **IN** → **§4.10.7 The select element** (`#attr-select-required`). *Not* §4.10.5.3.4 — that is the **input** `required` attribute; draft 2 routed it there and was wrong. |
| `§4.10.5.2.7` | 1 | `maxlength` enforcement | **IN** → **§4.10.19.3 "Limiting user input length: the maxlength attribute"** (`#attr-fe-maxlength`, the shared `for=input,textarea` dfn — sibling of `#attr-fe-minlength` = §4.10.19.4). Resolved now, not deferred. |
| `§4.10.3.1` | 1 | `form.elements` wrapper cache | **OUT** — collections → §10 queue |
| `§4.10.18.4` | 1 | `HTMLFormControlsCollection` | **OUT** — collections → §10 queue |
| `§4.10.15.4` | 1 | **shell** form reset | **IN** → §4.10.23 (previously invisible) |
| `§4.10.15.2` | 1 | `minlength` | **IN** → §4.10.19.4 |
| `§4.10.14.2` | 1 | `ValidityState` | **IN** → §4.10.21.3 |
| `§4.10.10.3` | 1 | disabled options in select | **OUT** → §10 queue |

Every one of the 25 numbers appears above. *(Draft 2 published "47 IN / 42 OUT / 7 split", which did not reconcile with its own rows under any reading; the aggregate is dropped rather than re-guessed — the per-row dispositions above are the authoritative accounting, and the 96 total is their sum.)* The remaining **57** numbers resolve and are triaged by detector (b) in §4.2 — **all of them**, per §1's principle.

### §4.1 *(folded into §4.0 — the hand-authored pattern list is gone)*

The first draft's §4.1 published a 7-pattern union of 52 cites and misfiled 2 of its 7 rows: `§4.10.21.4` resolves to "Security" and bare `§4.10.15` resolves to "The fieldset element", so both are **misuse** (§4.2), not non-resolution — 16 lines in the wrong bucket. §4.0 supersedes it.

### §4.2 Misuse — total triage of all 57 resolving numbers (detector (b))

Applied to **every** resolving number, per §1's principle. Numbers whose title matches their cite text throughout are correct and omitted; every number with ≥1 mis-cited site appears below with a per-number verdict.

**Cluster-IN misuse:**

| Number | Cites | Resolves to | Means | Verdict |
|---|---|---|---|---|
| `§4.10.21.3` | 19 | The constraint validation API | **18 mis-cited**; of those **16 mean §4.10.22.3**. The other 2: `well_known.rs:1134` (`reset` event) → §4.10.23/§4.10.3; `html_form_proto.rs:537` → §4.10.21.2. `validation/mod.rs:188` **correct, stays** | IN |
| `§4.10.21.4` | 10 | **Security** | `requestSubmit()` → §4.10.3; **except `html_form_proto.rs:33`** = the formdata event → §4.10.22.4 | IN, SPLIT |
| `§4.10.18.3` | 10 | Association of controls and forms | 3 mis-cited (`matching.rs:250`, `:405`, `tests/matching.rs:839` — `:indeterminate`, which is §4.16.3); 7 correct | **SPLIT** — the 3 → §4.6 fate table (Slice-1-owned) |
| bare `§4.10.15` | 5 of 16 | **The fieldset element** | `validation/mod.rs:1` (form validation → §4.10.21.1) + 4 genuinely-fieldset cites that are **correct** | IN (the 1) |

**The §4.10.12–15 element off-by-one cascade** — a *different* concept cluster, so per the scope decision only its **form-reset `<output>` subset** is swept here; the rest is queued (§10.1) with complete counts. Verified 2026-07-27 via `cite-audit`:

| Number | Cites | Resolves to | Actually means | Wrong | Correct target |
|---|---|---|---|---|---|
| `§4.10.13` | 9 | The progress element | **`<output>`** — all nine | **9 of 9** | **§4.10.12 The output element** |
| `§4.10.14` | 9 | The meter element | **`<progress>`** — all nine | **9 of 9** | **§4.10.13 The progress element** |
| `§4.10.15` | 16 | The fieldset element | 11 mean **`<meter>`**; 4 genuinely fieldset; 1 is `validation/mod.rs:1` | **11 of 16** | **§4.10.14 The meter element** |
| `§4.10.10` | 19 | The option element | 4 mean **`<datalist>`**, 4 mean **`<select>`**; 11 genuinely option | **8 of 19** | §4.10.8 (datalist) / §4.10.7 (select) |
| `§4.10.12` | **0** | The output element | — | — | **nobody cites the correct output section** — the cascade is total, not partial |

**Swept here** (in-cluster: the form-**reset** `<output>` surface, which §4.10.23 already brings into scope): `crates/dom/elidex-form/src/submit.rs:459`, `:486`; `crates/script/elidex-js/src/vm/globals.rs:380`; `crates/core/elidex-ecs/src/components.rs:601`, `:607`; `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs:904` — **6 sites** (verified 2026-07-27 via `cite-audit`), all → §4.10.12.

**Queued, not swept** (§10.1): the remaining 3 `§4.10.13` sites (`html_output_proto.rs:2`/`:5`, `vm/mod.rs:1075` — the `<output>` *IDL wrapper* surface, not reset), all 9 `§4.10.14`, all 11 meter-`§4.10.15`, all 8 mis-used `§4.10.10`. **31 sites** (verified 2026-07-27).

*(Draft 2 labelled one of these "(subset)" implying correct siblings existed. For `§4.10.13` and `§4.10.14` there are none — the wrong count is 9 of 9 in both.)*

**Per-site table for the `§4.10.20.*` family (32 cites)** — published rather than blanket-mapped, and **corrected**: draft 2's version mis-routed ≥7 lines and contradicted its own §3 and §4.0 on three.

| Sites | Concept | Target |
|---|---|---|
| `object_kind.rs:983`, `vm/mod.rs:1113`, `validity_state.rs:475`, `:498`, `form-core/lib.rs:424` | `ValidityState` interface / element-level CVA members | **§4.10.21.3** *(draft 2 routed these to §4.10.21.2/§4.10.3 while its own §3 and §4.0 said §4.10.21.3)* |
| `html_form_proto.rs:43`, `:634`, `html_fieldset_proto.rs:28`, `tests_html_form_proto.rs:249`, `:265` | barred / non-candidate | **§4.10.21.1** |
| `validity_state.rs:2`, `:152`, `:182`, `:407`, `:441`, `:466`, `validation/mod.rs:12`, `form-core/lib.rs:176`, `validation/tests.rs:690`, `tests_validity_state.rs` ×5 | candidacy vocabulary / CVA members | **§4.10.21.1** or **§4.10.21.3** per line |
| `html_form_proto.rs:538`, `:608`, `:613`, `well_known.rs:1145` | statically / interactively validate the constraints | **§4.10.21.2** *(`:538` was absent from draft 2's table despite it being billed per-site)* |
| `vm/globals.rs:287` | CVA-**mixin install** | **§4.10.21.3** *(§3 corrected this; draft 2's §4.2 did not)* |
| `vm/mod.rs:1113` (2nd cite on line), `tests_html_form_proto.rs:199`, `:224` | form-level `checkValidity` | **§4.10.3** + §4.10.21.2 |

### §4.3 `§`-less step drift (detector (c))

`html_form_proto.rs:484`, `:486`, `:508` carry `requestSubmit`'s stale "Step 2 / 2.1 / 2.2" with **no `§` on the line**, so steps (0)-(b) never admit them. Correct numbering is **1 / 1.1 / 1.2**. Together with the 7 `§`-carrying sites (§3), the family is **10 sites** (verified 2026-07-27).

**`html_form_proto.rs:483` is the parent list item** of the `:484`/`:486` substeps and must renumber with them — renumbering children while the parent stays "2." leaves the list incoherent. ⚠ **Open**: that block is a *local* "Algorithm: 1..5" narration whose item 1 is a brand check, not a spec step. §9-Q6 asks whether it is re-anchored to spec steps at all or left as local narration with the spec-step claims stripped.

**A third stale-step family, never enumerated** — the `§4.10.20.4` sites carry step numbers that resolve to nothing: `validity_state.rs:475` ("step 1.b"), `:498` ("step 3 fallback"), `html_form_proto.rs:613` ("step 1"), `tests_html_form_proto.rs:224` ("step 2"). Check each against `body html check-validity-steps` / `report-validity-steps` at impl. All in-cluster, all in files this PR already edits.

Likewise the "step 8.4 / 8.5" family (5 sites, verified 2026-07-27 — §4.2) — form-submission steps are **5 / 5.4 / 5.6 / 7**.

### §4.4 Two prose repairs that are not renumbers

1. **`submit.rs:276-279` states a falsehood**: `<input type=image>` is *not* "classified as `FormControlKind::SubmitButton` by `from_element`" — `from_type_str` (`crates/dom/elidex-form-core/src/lib.rs:253-282`) has no `"image"` arm, so it becomes `TextInput`. **Owned by PR-A0** (was double-claimed with PR-A §10; PR-A's claim is removed).
   ⚠ **Do not simply delete the sentence.** It is currently the tree's signal at the *consuming* site that `FormControlKind` has no image-button variant — so `form.requestSubmit(imageInput)` throws `TypeError` where HTML requires submission. The gap is named twice elsewhere (`crates/dom/elidex-form-core/src/input.rs:814-819` routes around it deliberately; `input_tests.rs:618` calls it a pre-existing `FormControlKind` coverage gap). The repaired comment must **point explicitly at `#11-input-image-state`**, or the repair removes the last in-tree warning at the place that would break.
2. **`validity_state.rs:152`/`:154`**: the § number is at `:152`, the bar list at `:154`. PR-A0 fixes the number **and** completes the list — it is currently missing the **`readonly`** bar that `is_constraint_validation_candidate` actually applies (`validation/mod.rs:40`), a pre-existing prose defect no other PR owns (PR-A only adds bars from its *own* new behavior). ⚠ The `readonly` bar is **conditional** — `state.readonly && state.kind.readonly_applies()` (`validation/mod.rs:40`) — so writing it flat would re-create the divergence this repair exists to close. **Alternative on the table (§9-Q5)**: collapse the duplicated list to an intra-doc reference to `is_constraint_validation_candidate` so it cannot drift again.

### §4.5 Inclusion rule — ONE rule, stated once

> **A site is IN iff the concept its comment describes belongs to the constraint-validation + form-submission cluster.** Not its file, not its crate, not its pattern.

The first draft stated this in I1 and then used a **file**-shaped rule in its §4.4 ("cites in cluster files"). That switch is the mechanism behind the misses — it is why `crates/shell/elidex-shell/src/content/form_input.rs:289` (a form-reset cite, unambiguously in-cluster) was invisible: no cluster *file* was in `crates/shell/`.

Applying the one rule pulls in, previously unnamed: `vm/host/events_misc.rs:306`/`:343`; `crates/shell/elidex-shell/src/content/form_input.rs:289`; `submit.rs:459`/`:486`; `vm/globals.rs:380`; `validity_state.rs:299`; `validation/mod.rs:368`; `input.rs:715`; `html_form_proto.rs:484`/`:486`/`:508`. It pushes **out**, with owners: the meter family, `§4.10.10.2`, `§4.10.7.4`/`.5`/`§4.10.10.3`, `§4.10.18.6`, `§4.10.18.4`, `§4.10.3.1`, `§4.10.11.1`, `html_option_proto.rs:21` (§10).

### §4.6 Fate — for cites whose code a successor PR re-authors

> **A cite is excludable only when the successor PR deletes the code it annotates, *or* rewrites it such that the successor must re-author the comment. A cite that outlives the change is IN.**

**The discriminator, stated** (it was applied but never written down — all five `crates/css/elidex-css/src/selector/tests/matching.rs` sites are in Slice-1's *same* rewrite set, yet split IN/OUT):

> **Re-seeding a test body does not force a comment edit; semantically inverting its assertion does.** Slice 1 re-seeds every one of those tests (`ElementState` bits → `Attributes`/FCS), which leaves each comment's *claim* intact. But for three of them Slice-1 §6-A **inverts the asserted answer** (rows 6-7 `<fieldset>` now matches `:disabled`/`:enabled`; row 9 `<output>` no longer matches `:valid`), so the comment asserting the old answer becomes false and Slice 1 **must** re-author it. For the other two the claim survives verbatim, so nothing forces Slice 1 to touch the comment — and an untouched wrong cite is PR-A0's job.

⚠ **This is a commitment Slice-1 has not made.** Its §6-C lists **no** `tests/matching.rs` site at all. The reciprocal edit (§11) must therefore *add* the three OUT sites to Slice-1 §6-C explicitly; without that row they are orphaned by both memos. If Slice 1 declines, all five flip to IN here.

Verdicts against Slice-1's §5 deletion set and its §6-A inversion rows:

| Site | Cite | Annotates | Slice-1 fate | Verdict |
|---|---|---|---|---|
| `matching.rs:242` | §4.10.18.5 | `:disabled` arm comment | arm rewritten, comment re-authored | **OUT** — Slice 1 |
| `matching.rs:250` | §4.10.18.3 | `:indeterminate` arm | rewritten | **OUT** — Slice 1 |
| `matching.rs:256` | §4.10.18.7 | `<output>`-always-valid | **deleted** (the special case goes) | **OUT** — Slice 1 |
| `matching.rs:284` | §4.10.15.2.4 | `:required` arm | rewritten, re-authored | **OUT** — Slice 1 |
| `matching.rs:137` | §4.10.15.2.4 | `is_requirable_element` doc | **deleted** (§5-R4) | **OUT** — Slice 1 |
| `matching.rs:405`/`:419` | §4.10.18.3/.5 | `is_indeterminate_candidate` / `is_disableable_element` docs | **deleted** (§5-R4) | **OUT** — Slice 1 |
| `matching.rs:577` | §4.10.18.5 | inline test comment | test rewritten | **OUT** — Slice 1 |
| `tests/matching.rs:715` | §4.10.15.2.4 | `required_does_not_match_fieldset` | re-seeded; **claim survives** (`:required` still excludes `<fieldset>`) ⇒ no forced comment edit | **IN** |
| `tests/matching.rs:727` | §4.10.15.2.4 | `optional_does_not_match_fieldset` | **Slice-1 §5 lists line 726 among the 16 it rewrites**, and its re-seed rule removes the `ElementState::default()` insert — so the *body* changes; the comment still outlives it | **IN** (verdict unchanged; **basis corrected** — the earlier "survives byte-identical" was wrong) |
| `tests/matching.rs:767` | §4.10.18.7 | `valid_matches_output_element` | **§6-A row 9 inverts the answer** ⇒ comment must be re-authored | **OUT** — Slice 1 *(needs the §6-C row)* |
| `tests/matching.rs:788` | §4.10.18.5 | fieldset-not-disableable | **§6-A rows 6-7 invert the answer** ⇒ must be re-authored | **OUT** — Slice 1 *(needs the §6-C row)* |
| `tests/matching.rs:839` | §4.10.18.3 | `:indeterminate` non-candidate | §6-A row 8 adds the `<progress>` branch ⇒ answer changes, comment re-authored | **OUT** — Slice 1 *(needs the §6-C row)* |
| `validity_state.rs:152`/`:154` | §4.10.20.3 | the bar list | PR-A adds *its own* bars only | **IN** (number + the missing `readonly` bar, §4.4) |
| `validation/mod.rs:12` | §4.10.20.3 | candidacy doc | PR-A adds its own bars | **IN** for the number; PR-A owns the new-bar prose |

**⚠ Slice-1 §6-C is wrong about one site.** Its row reads `matching.rs:137` / `:284` / `:299`, but **line 299 carries no `§` at all** (`// :optional only matches input/select/textarea (same constraint as :required).`). Verified 2026-07-27. The reciprocal edit must **correct** that row, not mirror it.

**Reciprocal edits** (§11): write the per-site verdicts into **both** Slice-1 §6-C and **PR-A** §4.4, so no memo can silently drop a site.

**Non-`elidex-css` contested sites — the same rule, stated rather than assumed.** Earlier drafts were exhaustive for `elidex-css` only; these three are contested and were silent:

| Site | PR-A0 does | Slice 1 does | Verdict |
|---|---|---|---|
| `crates/dom/elidex-form-core/src/validation/mod.rs:32` | **adds** a new §4.10.5.1.1 cite on the hidden bar | §5-M2 rewrites the *disabledness* lines (`:29-31`, `:43`) in the same fn | **IN — no conflict.** Different lines; the added cite outlives M2's edit, which does not touch the hidden bar. |
| `crates/dom/elidex-form/src/submit.rs:306`/`:322`/`:276-279`/`:457`/`:459`/`:486` | renumbers 6 comments | §5-M1 rewrites `collect_control_entry` (`:349-355`) | **IN — no conflict.** M1's rewrite is 30+ lines below the nearest PR-A0 cite; no comment PR-A0 edits is re-authored by M1. |
| `crates/shell/elidex-shell/src/content/form_input.rs:289` | renumbers the reset cite | §5-W5 deletes `:36-39` (the `ElementState` sync) | **IN — no conflict.** Different function, 250 lines apart; `:289`'s comment outlives W5 untouched. |

**Also OUT with an owner**: `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — different concept, already queued in `memory/project_html-section-renumber-sweep.md`.

---

## §5 Behavior deltas

**No `crates/**` behavior change.** Within the engine the diff is comments and doc-comments only — no production statement, signature, or test assertion changes.

**The PR is no longer doc-only, and §6/§8 say so.** It also adds **code**: `.claude/tools/_webref/commands/cite_audit.py` (the discovery detector, §3.1), its wiring in `_webref/cli.py`, and `_webref/test_cite_audit.py`. That is deliberate — the tool is what stops the hand-authored class from recurring, and bundling it here is what gets it to `/code-review` and `/elidex-review` at pre-push, which is where a tool's correctness is actually caught. It is outside the cargo workspace and has no engine consumer, so it cannot affect engine behavior; it is scoped and tested in §6.

## §6 Test plan

There is nothing to assert — the deliverable is prose. Verification is procedural:

1. **`mise run ci` green**, including the `-D warnings` `doc` job (catches a broken intra-doc link introduced while editing doc-comments).
2. **Diff discipline, scoped**: over `crates/**` only, `git diff -U0 -- crates/ | grep '^[+-]' | grep -v '^[+-][+-]' | grep -vE '^[+-]\s*(//|///|//!|\*)'` must be **empty** — the mechanical proof that no engine non-comment line changed. The `.claude/tools/**` diff is **explicitly excluded and justified** (§5): it is the new detector plus its tests, outside the cargo workspace, with no engine consumer.
3. **Citation re-verification**: every `§`-number introduced by the diff re-checked with `webref heading --exact`, and every **step** number re-checked with `body html <anchor>`.
4. **Exit criterion — negative space, not a pattern list.** Re-run §3.1 step (0) and assert that **every** distinct `§4.10.x` in `crates/` either resolves *or* appears on §4.0's named-OUT list. A pattern-shaped residual scan cannot state this: the first draft's scan was **narrower than its own audit grep** (it dropped `§4.10.15.2` and never had `§4.10.21.5` / `§4.10.18.x` / bare `§4.10.15`), so the PR could have passed its own exit criterion with the cluster half-swept. The negative-space form has no such gap by construction:
   ```
   .claude/tools/webref cite-audit html --prefix 4.10 --strict   # exit 1 on any unresolved
   .claude/tools/webref cite-audit html --prefix 4.10            # per-cite review pass
   ```
   **Scoped to attributed-HTML cites in the cluster** — the assertion cannot cover bare cites whose spec is unknown (§4.0), so unattributed cites are *reported as a count*, not asserted on. **Per-cite over the concept axis, not per-number over resolution**: (i) `--strict` exits 0, or every remaining `*** UNRESOLVED ***` number matches §4.0's OUT list exactly; (ii) each cite in the full output carries a disposition — swept, or on §4.6's fate list, or on §10.1's queue. A pattern-shaped scan cannot state (ii); draft 2's residual scan was in fact **narrower than its own audit grep** (dropping `§4.10.15.2`, never having `§4.10.21.5` / `§4.10.18.x` / bare `§4.10.15`), so the PR could have passed its own exit criterion with the cluster half-swept. Plus: no `§4.10.21.3` survives except `validation/mod.rs:188`; no `§`-less "step 2/2.1/2.2" in the `requestSubmit` family; `§4.10.12` cite count > 0.

5. **The tool is code, so it is unit-tested** — `.claude/tools/_webref/test_cite_audit.py` (NEW, 13 tests, `unittest`, following the `test_inventory_diff.py` / `test_agent_brief.py` / `test_refresh.py` shape already in `_webref/`). It is outside the cargo workspace, so `mise run ci` does not run it; the gate runs it explicitly alongside its three siblings. Coverage maps 1:1 onto the defects that made it necessary:
   - **`TestStrictExitCode`** — `--strict` exits non-zero on an unresolved cite, zero when all resolve, and zero without the flag. *(`--strict` was a no-op: `cli.main` calls `args.func(args)` and discards the return value, so the bare `return 1` was dropped.)*
   - **`TestPrefixIsDottedComponent`** — `--prefix 4.10.2` matches `4.10.2` and `4.10.2.7` but **not** `4.10.20.3`. *(It used `str.startswith`, silently sweeping the whole `4.10.2x` range.)*
   - **`TestSpecAttribution`** (7 tests) — explicit label selects that spec; longest label wins (`WHATWG HTML` is not parsed as the bare `HTML` alias); bare cites inherit within a comment block; a non-comment line ends the block; `§0.5`-style plan-memo pointers land in UNATTRIBUTED rather than being audited as HTML; `--show-unattributed` lists them.
   - **`TestResolverExactness`** — a cited `§4.10.20.3` reports UNRESOLVED rather than passing because `§4.10.2` exists. *(`lookup_section` is prefix-tolerant and has **no** `exact` parameter — the first commit called it with `exact=True`, which Pyright flagged; the shipped guard is `hit[0] == section`.)*

   Verified green 2026-07-27: `python3 .claude/tools/_webref/test_cite_audit.py` → 13 tests OK, and the three sibling test files still OK.

**Gate**: `mise run ci`, then `/pre-push`, then **`/external-converge` from round 1**. The reactive escalation trigger ([[feedback_gate-miss-on-edge-dense-escalate-to-converge]]) has already fired twice on this memo — two gaps found pre-dispatch, then 1 CRIT / 28 IMP at review — and the #483 precedent this PR invokes did **not** converge in a single pass (it left three open carves). §2's honest re-statement (discovery is not canonical) is the same conclusion from the design side.

---

## §7 Layering check

- **Doc-only** — no code moves, no crate edge, no new `vm/host/` algorithm, no `elidex-ecs` touch. ✓
- The sweep **reinforces** the mandate indirectly: `validation/mod.rs:1`'s module doc stops claiming the fieldset section for a validation module, and `validity_state.rs:154`'s bar list stops describing a predicate it no longer matches. ✓
- **⚠ Not a clean bill, and PR-A0 is about to make it worse before PR-A makes it better.** `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) **is** a form-level validation algorithm living in `vm/host/`, and this PR is about to author a *spec-blessed* `§4.10.21.2` comment on it — i.e. to document drift as though it were design. Carried mitigations:
  - a **concept → engine-independent-API mapping** in the rewritten comments: §4.10.21.1 candidacy → form-core's `is_constraint_validation_candidate`; "satisfies its constraints" → form-core's `validate_control`; §4.10.21.2 steps 1-4 → **`elidex_form::statically_validate_constraints`** (NEW) — PR-A part 4;
  - an explicit **hand-off line** at `html_form_proto.rs:620` naming PR-A's export, so the comment reads as "steps 5-7 here, steps 1-4 land in `elidex-form` next PR" rather than as a settled home.
- **⚠ The same treatment is owed to §4.10.23's twin.** "Resetting a form" spans **three** copies: `elidex_form::reset_form` (`crates/dom/elidex-form/src/submit.rs:464`, the correct engine-independent home for step 4) plus the fire-`reset`/honor-`preventDefault` half duplicated in `crates/script/elidex-js/src/vm/host/html_form_proto.rs:592-605` **and** `crates/shell/elidex-shell/src/content/form_input.rs:290-302`. PR-A0 is about to stamp §4.10.23 authority on all three. Same mitigation as §4.10.21.2: each repaired comment carries the concept → engine-indep-API mapping (§4.10.23 step 4 → `elidex_form::reset_form`) and names the duplication rather than blessing it. **The shell copy folds into PR-A's `#11-shell-submit-interactive-validation`** (its second facet is already "an independent second copy of the submission event sequence" — the reset sequence is the same defect).
- **ECS-native**: N/A — no component read or written. ✓

---

## §8 Line-count budget

Full derivation over **every** file this memo names as a touch site (verified 2026-07-27, `wc -l`), not the subset earlier drafts published. Sorted descending; **bold = already >1000**.

| File | Lines | Why touched |
|---|---|---|
| `crates/script/elidex-js/src/vm/mod.rs` | **2782** | §4.2 (`:1075`, `:1089`, `:1094`, `:1105`, `:1113`, `:1496`, `:1500`, `:803`, `:806`, `:1070`, `:1083`) |
| `crates/script/elidex-js/src/vm/host/html_select_proto.rs` | **1806** | §4.2 §4.10.10 misuse (`:516`, `:557`) |
| `crates/script/elidex-js/src/vm/object_kind.rs` | **1700** | §4.2 (`:983`) |
| `crates/script/elidex-js/src/vm/well_known.rs` | **1588** | §4.2 (`:1134`, `:1136`, `:1140`, `:1145`) |
| `crates/script/elidex-js/src/vm/globals.rs` | **1533** | §4.2 (`:287`, `:380`) |
| `crates/script/elidex-js/src/vm/tests/tests_html_t2d_protos.rs` | **1207** | **newly pulled in by the cascade sweep** (`:904`) |
| `crates/core/elidex-ecs/src/components.rs` | **1037** | **newly pulled in by the cascade sweep** (`:601`, `:607`) |
| `crates/dom/elidex-form-core/src/lib.rs` | 960 | §4.2 (`:176`, `:424`) — **nearest to the cap**; PR-A adds ~15 more (its §8) |
| `crates/script/elidex-js/src/vm/host/events_misc.rs` | 835 | §4.5 (`:306`, `:343`) — named in §3/§4.5, absent from earlier §8 |
| `crates/script/elidex-js/src/vm/host/html_form_proto.rs` | 680 | §4.2, §4.3, §7 |
| `crates/dom/elidex-form/src/submit.rs` | 546 | §4.2, §4.3, §4.4 |
| `crates/script/elidex-js/src/vm/host/validity_state.rs` | 518 | §4.2, §4.4 |
| `crates/dom/elidex-form-core/src/validation/mod.rs` | 430 | §4.0, §4.2, §4.4 |
| `crates/shell/elidex-shell/src/content/form_input.rs` | 318 | §4.5 (`:289`), §7 |
| `crates/script/elidex-js/src/vm/tests/tests_form_submit.rs` | 250 | §4.2, §4.3 |
| *(plus the remaining test/proto files named in §4.2, all <1000)* | | |

**Disposition — explicit, same shape as Slice-1 §9's `components.rs` row.** **Seven** touched files are already >1000, two of them (`tests_html_t2d_protos.rs`, `components.rs`) pulled in *by this PR's cascade sweep*. The touch-time split discipline targets *growth into* unbounded files: PR-A0's `crates/**` diff is **comments only and net-flat** (a few multi-line comments reflow ±1 line), so it adds no debt to any of them and triggers no split obligation. Splitting any would be a speculative refactor bundled into a citation sweep, which the same discipline forbids. Recorded so plan-review reads the seven counts as dispositioned, not overlooked.

**Tool files added** (outside `crates/`, outside the cargo workspace): `_webref/commands/cite_audit.py` (~230), `_webref/test_cite_audit.py` (~155), + ~3 lines wiring in `_webref/cli.py`. None near any cap.

## §9 Open questions for `/elidex-plan-review`

- **Q1 — is the §3.1 method sufficient?** Its three detectors catch non-resolution, misuse, and `§`-less step drift. It does **not** catch a cite that is right about the section but wrong about the **anchor** within it. **Recommendation: out of scope, with a slot** (§10 `#11-citation-anchor-level-verification`) — anchor checking needs a different tool path (`body html <anchor>` per site) and would roughly double the PR. Deferring it silently while Q1 stayed open was the earlier draft's inconsistency with its own "zero deferred work" claim.
- **Q2 — two adjacent boundaries, same shape.** (a) `§4.10.22.3` vs `§4.10.22.4`: `form_data.rs`'s "step 7 construct the entry list" is a step *of* §4.10.22.3 that *invokes* §4.10.22.4. (b) **`§4.10.21.2` vs `§4.10.22.3`**: the interactive-validation arm is §4.10.22.3 step 5.4, which invokes §4.10.21.2's algorithm — the same invoking-step-vs-invoked-algorithm question. Recommendation for both: cite the **invoking step** where the code implements the caller, and the **invoked algorithm** where it implements the algorithm. Confirm.
- **Q3 — §4.6's fate criterion**: applied per site above. Confirm the four verdicts, and confirm the reciprocal Slice-1 §6-C rows are the right coordination shape.
- **Q4 — the repair scope floor, re-derived from §4.5's concept rule.** (Draft 2's Q4 argued from "cites in cluster **files**" — the very file rule §4.5 identifies as the defect, and empirically the rule that drew the §4.10.13 line wrongly.) Restated: repair = every cite whose *concept* is constraint validation or form submission, wherever it lives, **plus** the 6 form-reset `<output>` sites §4.2 pulls in via §4.10.23. Confirm the `<output>` boundary — reset is in-cluster, the `<output>` IDL-wrapper surface is not.
- **Q6 — `html_form_proto.rs:483-508`'s local "Algorithm: 1..5" narration.** Its item 1 is a brand check, not a spec step, so renumbering 2/2.1/2.2 → 1/1.1/1.2 would make a *local* list claim spec-step identity it does not have. Options: (a) re-anchor the whole list to §4.10.3's `requestSubmit` steps, (b) keep it local and strip the spec-step claims from the substeps. **Recommendation: (b)** — smaller, and it stops the list from drifting with the spec. Confirm.

---

## §10 Defer slots + the out-of-cluster inventory

| Slot | Status | Audit |
|---|---|---|
| **`#11-citation-anchor-level-verification`** | **NEW** (from §9-Q1) | (1) Real gap? yes — §3.1's detectors verify section *numbers* and concepts, not the `#anchor` within a section; a cite can name the right section and the wrong dfn. (2) Blocked by structure? no, but it needs a per-site `body html <anchor>` pass that would roughly double this PR. (3) Non-regressing to defer? yes — no cite gets *worse*. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the next citation sweep, or a review finding a wrong-anchor cite. **Date**: 2026-11. |

| **`#11-unattributed-spec-citations`** | **NEW** (from §4.0) | (1) Real gap? yes — **6931 cites** tree-wide carry a bare `§` whose spec a reader cannot determine from its comment block (234 inside `§4.10.x` alone). A citation a reader cannot attribute is a defect independent of resolution. (2) Blocked by structure? no, but it spans every spec and every crate — sweeping it here would collide with every active lane, and it is a different concept axis from this cluster. (3) Non-regressing to defer? yes — pre-existing, and `cite-audit` now *reports* the count so it cannot go quiet again. (4) Durable home? `project_open-defer-slots.md`. **Trigger**: the next cross-spec citation pass, or a review mis-attributing a bare cite. **Date**: 2026-12. |

**Per-PR budget**: **2 new slots** (≤3 ✓, one slot of headroom). Draft 2 claimed "zero deferred work" while §9-Q1 was open — corrected.

### §10.1 Out-of-cluster inventory — a deliverable, not a silence

The §3.1 step-(0) enumeration surfaces drift the cluster rule (§4.5) pushes **out**. Sweeping it here would collide with every active lane; leaving it unrecorded would convert a measured gap into a silent one. It is therefore filed as **new queue entries in `memory/project_html-section-renumber-sweep.md`**, which is precisely the tracked program that exists to hold it:

| Family | Cites | What it is | Correct target |
|---|---|---|---|
| `§4.10.10.2` | 21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | **§2.6.4.3 The HTMLOptionsCollection interface** (interface sites) / §4.10.7 (select) per site — *not* §4.10.10, which draft 2 gave; webref-verified 2026-07-27 |
| **the §4.10.12-15 element cascade** (out-of-cluster remainder) | **31** (verified 2026-07-27, §4.2) | `<output>` IDL wrappers citing §4.10.13 (3); `<progress>` citing §4.10.14 (9); `<meter>` citing §4.10.15 (11); `<select>`/`<datalist>` citing §4.10.10 (8) | §4.10.12 / §4.10.13 / §4.10.14 / §4.10.7 / §4.10.8 respectively. ⚠ `html_meter_proto.rs:20`'s "`<progress>` … §4.10.14 vs §4.10.15" contrast is **inverted**: progress = §4.10.13, meter = §4.10.14 |
| `§4.10.7.4` / `§4.10.7.5` / `§4.10.10.3` | 7 | select `value` setter, `select.add()`, disabled options | §4.10.7 (select) subsections |
| `§4.10.18.6` / `§4.10.18.7` (form-attribute subset) | 5 | `form.method` / `enctype` / `autocomplete` | **§4.10.19.6 Form submission attributes** — webref-verified 2026-07-27 |
| `§4.10.18.4` / `§4.10.3.1` | 2 | `HTMLFormControlsCollection`, `form.elements` wrapper cache | **§2.6.4.2 The HTMLFormControlsCollection interface** / §4.10.3 — webref-verified 2026-07-27 |
| `§4.10.11.1` | 2 | textarea dirty-value flag | §4.10.11 (The textarea element) |
| `§4.10.10.1` (option subset) | 1 | `html_option_proto.rs:21` option text | §4.10.10 |

**Registration shape** — follow that memo's established `## ✅ DONE — #N ✅ MERGED (sha, date, comment-only)` heading for the discharged part, and add the table above as a new open queue block. **Per-bullet discharge, not a blanket claim:**
- **Fully discharged — `project_html-section-renumber-sweep.md:34`** (the constraint-validation family, ~26 sites as listed there; verified 2026-07-27 against `cite-audit`): every site it lists is swept here. Mark the bullet DONE rather than leaving it advertising sites that no longer exist.
- **Fully discharged — line 36's SubmitEvent / FormDataEvent halves**: `vm/mod.rs:1496`/`:1500` **and** their previously-unnamed twins `vm/host/events_misc.rs:306`/`:343` (§4.5). ⚠ §10.1 must **not** re-list these — draft 2's inventory duplicated 5 of its 7 rows against line 36.
- **Fully discharged**: the two textarea/`pattern` sites (`validation/mod.rs:210`, `validation/tests.rs:478`) that the memo's 2026-07-18 Slice-0b entry queued as item (a).
- **Not discharged**: that same entry's item (b), `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — out of cluster, stays queued.
- **Partially discharged, residue restated**: the constraint-validation + form-submission cluster is swept; the residue is §4.6's OUT list (the Slice-1-owned `elidex-css/selector/` sites) plus §10.1's out-of-cluster families.

*(The earlier draft quoted that memo as saying "the next PR that touches those files" — **that phrasing does not appear in it**. Paraphrased here instead; the actual entry is dated 2026-07-18 and marks the two sites pre-existing and diff-external to #479.)*

---

## §11 Coordination

Re-derived at authoring time 2026-07-27 (`git worktree list`; `git rev-list --count origin/main..<branch>`; `gh pr list --state open`) — **not recalled**. This table was wrong in two prior rounds; the counts below are freshly measured.

| Lane | Branch | Commits ahead | PR | Overlap | Ordering |
|---|---|---|---|---|---|
| **PR-A0** (this) | `domform-submittable-category` | **2** (both PR-A0-side) | — | — | lands first |
| **PR-A** | same branch | **0 — planned, not yet committed** | — | PR-A0 renumbers comments in every file PR-A edits | **PR-A0 first**; PR-A writes prose onto corrected numbers |
| **Slice 1** | `domform-slice1` | 1 | — | the `elidex-css/selector/` sites, dispositioned per site in §4.6 | Slice 1 keeps its OUT set; §4.6 reciprocal rows correct its §6-C |
| **VM lane** | `vm-p4-plan` | **7** | — | **none** — `git diff --name-only origin/main..vm-p4-plan` = 1 file, **zero `.rs`** (verified 2026-07-27). Draft 2's overlap cell was wrong. | none |
| **Layout** | `terminal-z-c3a-impl` | **17** | **#488** open | none | none |
| **Layout plan** | `terminal-z-c3-plan` | **8**, pushed | — | none | none |
| **History** | `session-history-slice-b` | **22** | **#487** open | none | none |

**Shell-lane check — answered, not deferred.** PR-A0 touches `crates/shell/elidex-shell/src/content/form_input.rs:289` (§4.5). `git diff --name-only origin/main..session-history-slice-b` = **16 files, none of them `content/form_input.rs`** (verified 2026-07-27). **No overlap.**

**Disambiguation**: every `tests/matching.rs` in this memo means `crates/css/elidex-css/src/selector/tests/matching.rs`.

**Carried edits:**
1. **`docs/plans/2026-07-form-derived-state-reconciliation.md` §6** — **add PR-A0 and PR-A to the umbrella's slicing table** (neither appears in it today) and add an umbrella link to this memo's §0, as Slice-1's memo has. This is what makes CLAUDE.md's edge-dense **base case** apply: a terminal single PR must be *an approved umbrella's* narrowly-scoped slice, and an unlisted PR cannot claim that. §2 concedes the discovery half is edge-dense, so the base case — not §3's breadth metric — is what carries the single-PR shape. (Draft 2 answered the split obligation with breadth; those are different tests.)
2. **`memory/project_html-section-renumber-sweep.md`** — per-bullet discharge + the §10.1 out-of-cluster inventory (§10).
3. **Slice-1 §6-C** — reciprocal per-site rows from §4.6. Three things, not one: (a) **add** rows for `src/selector/tests/matching.rs:767`/`:788`/`:839`, which §4.6 assigns to Slice 1 on the semantic-inversion rule and which §6-C lists **nowhere** today — without them the sites are orphaned by both memos; (b) **correct** the existing row that names `matching.rs:299`, which carries **no `§` at all** (verified 2026-07-27); (c) record that `:715`/`:727` are PR-A0-owned. If Slice 1 declines (a), all five flip to IN here and PR-A0 sweeps them.
4. **PR-A §4.4** — reciprocal rows for the two sites PR-A0 now owns outright (`submit.rs:276-279`, `validity_state.rs:152`/`:154`), and removal of PR-A's duplicate claim on the first.
