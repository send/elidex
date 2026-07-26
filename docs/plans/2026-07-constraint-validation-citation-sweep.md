# Plan — a citation detector, and the sweep it can verify (PR-A0)

## §0 Status

**Lane**: DOM/form (L3). **Branch**: `domform-submittable-category`. **Worktree**: `/Users/kazuaki/repos/send.sh/elidex-wt-submittable`.
**Base**: `f7d9b5ce` (`origin/main`). **Lands FIRST**, before PR-A.
**Blocks**: `docs/plans/2026-07-form-submittable-category-repair.md` (PR-A) → `docs/plans/2026-07-form-elementstate-cache-deletion-slice-1.md` (Slice 1).
**Status**: plan-memo draft — `/elidex-plan-review` before implementation.
**Nature**: **authors a detector and sweeps what the detector can verify.** Two parts, and the framing matters:
- **`.claude/tools/webref cite-audit` (NEW) is the headline deliverable** — a tested tool that measures citation drift across the whole tree. It is durable and reusable; the sweep is downstream of it.
- **The sweep repairs what the tool attributes to HTML in the `§4.10` set**, and **measures** — rather than claims — the classes the tool cannot see (§4.8).

**This PR makes no completeness claim about "the cluster".** Four successive drafts asserted one and were wrong four times (§1); the assertion has been replaced by a bounded, checkable gate (§6) plus named, measured slots (§10). **Not doc-only**: the `crates/**` diff is comment-only, but the PR also adds tool code + tests (§5).

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

**The blind spot recedes; that is the finding.** Every round the detector improved and the blind spot moved to whatever the *new* detector could not see:

| Class | How it hides | Status |
|---|---|---|
| (a) non-resolving numbers | — | **tool** ✓ |
| (b) resolves, wrong concept | title matches the number, not the comment | judgment over tool output ✓ |
| (c) `§`-less step numbers | `_CITE_RE` requires a `§`; **structurally invisible** to the tool | **measured slot** (§4.8) |
| (d) right section, right title, **wrong step** | invisible to (b) — the title matches; invisible to (c) — a `§` is present | **measured slot** (§4.8) |
| (e) org-only labels (`WHATWG §4.10.7.3.1`) | names an org, not a spec; fits no bucket cleanly | **measured slot** (§4.8) |
| (f) the "attributed to another spec" bucket | audited by nothing | **measured slot** (§4.8) |

Each class was found **by looking**, not by a detector — so an exhaustiveness banner over "the cluster" will always recede one class ahead of whatever is currently checkable.

**So the claim is re-scoped, and the work is kept.** §1's thesis is no longer "sweep the cluster completely". It is:

> **The detector is the deliverable; its boundary is stated, not hidden.** PR-A0 repairs what the tool can verify, and converts every class it cannot verify into a slot with a current count and the command to re-derive it. Measured-and-owned beats claimed-and-wrong.

That is the One-issue-one-way answer — one canonical detector, honest about where it stops — rather than a fifth completeness assertion.

**The corrected shape**: discovery is step **(0)**, it is a total enumeration of the space, and every number it returns is dispositioned. §4 is re-derived from its output rather than from a pattern list.

---

## §2 Coupled invariants

**Edge-density judgment, restated honestly.** The first draft called this "not edge-dense" because "there is one canonical algorithm (resolve → compare → replace)". That is true of **repair** and false of **discovery** — and discovery is this PR's substance (§1). There is no canonical algorithm for "find every mis-cited section in a concept cluster"; the last two attempts each shipped a detector that could not see its own blind spot. So: **the repair half is mechanical; the discovery half is the design**, and §2 exists for the latter.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Concept-scoped completeness** — the unit is the *concept cluster*, not the file and not the pattern. Partial repair across semantic siblings is what #483 was forced to complete by its external reviewer ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]). | One inclusion rule, applied uniformly (§4.5). The first draft stated a concept rule in I1 and a **file** rule in its §4.4 — that rule-switch is the mechanism behind the misses. |
| **I2** | **Total disposition *within the detector's reach*** — every number the tool returns is IN or OUT-with-a-named-owner. Classes the tool cannot see are measured (§4.8), not claimed. | §4.0's table publishes all 81; §4.5 dispositions all 25 non-resolving numbers and every misuse found. |
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

No algorithm branch is implemented or altered **in `crates/**`** — the engine diff is comments only (the PR also adds tool code, §5). Each row is a **citation target** the sweep introduces or corrects.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21.1 Definitions | candidate / barred / satisfies constraints | candidacy vocabulary | the `§4.10.20.3` family — **per-site table in §4.2**, not blanket-mapped | ✓ (per-site) | no (comment-only in `crates/**`) |
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

**Breadth**: K=1 spec (WHATWG HTML), M=18 rows → **single-PR scope** on the breadth metric. ⚠ **Breadth is not the split test that matters here.** §2 concedes the discovery half is edge-dense, so the single-PR shape rests on CLAUDE.md's **base case** — an approved umbrella's narrowly-scoped slice — which is why §11's carried edit adds PR-A0 to the umbrella §6 slicing table. Without that row the base case does not apply and breadth alone cannot carry it.

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

## §4 Site accounting — bounded by what the detector verifies

### §4.0 The evidence base — all 81 numbers

Run 2026-07-27 via `.claude/tools/webref cite-audit html --prefix 4.10`.

**Attribution matters, and the raw numbers are not the audit set.** The tool's first cut resolved *every* bare `§N.N` against HTML, which reports **correct work as drift**: `§2.2.2` (37 cites in `crates/net/elidex-net/src/preflight/`) is WHATWG **Fetch** — the citations PR #483 deliberately swept *to* their correct Fetch numbers — and `§0.5` (4 cites, e.g. `vm/dispatch_class.rs:6`) is a **plan-memo** pointer, not a spec citation at all. The tool now attributes each cite to a spec in three buckets (§3.1) before resolving. Whole-tree, the difference is an order of magnitude:

| Measure | Unattributed (first cut) | **Attributed (authoritative)** |
|---|---|---|
| `crates/` distinct sections / cites | 1257 / 11984 | **405 / 2390** (html) |
| unresolved sections / cites | 710 / 3763 | **61 / 171** (html) |
| attributed to another spec | — | 2663 cites |
| **UNATTRIBUTED** | — | **6931 cites** |

**The `§4.10` cluster, attributed to HTML**: **75 distinct sections / 518 cites**; **24 unresolved / 94 cites**; 47 cites attributed elsewhere; **166 unattributed**.

⚠ **A fourth measurement, and why**: 74/497 became 75/518 when the attributor learned to carry a spec label across a **comment-line wrap** (§6 item 5). `form_data.rs:181-182` reads "… WHATWG XHR §4.3 constructor + WHATWG HTML / §4.10.21.3 step 7", so the wrapped cite was bucketed to `xhr` — a false negative *inside the audited set*, which is worse than an unattributed cite because nothing reports it. 21 cites recovered.

**Which measure each table uses.** §4.0's 25-number table is **raw-grep** (spec-agnostic, so it includes cites later attributed elsewhere); everything downstream is **attributed**. The 96-vs-94 delta is exactly that: `§4.10.10.2` 21 raw / 20 attributed, `§4.10.15.3` 2 / 1, `§4.10.7.3.1` 2 / 0. Stated so the two are not read as a contradiction.

### The measurement has been wrong in three successive directions — and only the third is measured

This provenance is the memo's most transferable content, because it is the **third instance of the one lesson the tool now exists to prevent**: a hand-authored enumeration reports what its author already suspected.

| # | Figure for the `§4.10` set | Method | Why it was wrong | Direction of error |
|---|---|---|---|---|
| 1 | **81 / 632** | hand-written grep `§4\.10\.[0-9]+(\.[0-9]+)*` | required a digit after `4.10.`, so it **could not match bare `§4.10`** — 99 cites, the single largest family in the tree | **under**-counted the space |
| 2 | **82 / 731** | `cite-audit`, pre-attribution | correct about the *space*, but resolved **every** bare `§N.N` against HTML, so cites belonging to other specs (and non-spec pointers) were counted as HTML | **over**-counted the audit set |
| 3 | **74 / 497** | `cite-audit` with spec attribution (§3.1) | — | **measured** |

Both earlier figures are **superseded**. Each was produced by a method that could not see its own blind spot, and each blind spot was invisible from inside the method that had it — which is precisely why discovery is now a tested tool (§6 item 5) rather than a paragraph.

Consequently the 25-number table below is **24** numbers under attribution (`§4.10.7.3.1` drops out — both its cites are unattributed). **Under attribution the resolving count is 75 − 24 = 51**, the denominator §4.2 uses; earlier drafts' 81 / 82 / "all 57 resolving" are superseded.

**UNATTRIBUTED is a finding, not noise.** 6931 cites tree-wide (234 within `§4.10.x`) carry a bare `§` whose spec a reader cannot determine from its comment block. That is a defect *independent of whether the number happens to resolve*, and it is the largest single citation-hygiene gap the tool exposes. **Out of scope here** (it spans every spec and every crate) → queued in §10.1 as `#11-unattributed-spec-citations`.

**The 25 non-resolving numbers, with cite counts and disposition:**

| Number | Cites | Concept it annotates | Disposition |
|---|---|---|---|
| `§4.10.10.2` | ~21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | **OUT** — select/option-collection surface → §10 queue |
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
| `§4.10.15.3` | 2 | entry collection + `form=` association | **IN, SPLIT** (verified 2026-07-27) — one number, **two** concepts, per site: `submit.rs:306` (`collect_form_data` doc) → **§4.10.22.4** "Constructing the entry list"; `submit.rs:322` (the `form=` IDREF arm) → **§4.10.18.3** "Association of controls and forms". The row read as a single target through four drafts; both sites are repaired in batch 1 (`19f23d57`). |
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

### §4.2 Misuse — triage of all 51 attributed-and-resolving numbers (detector (b))

Applied to all **51** numbers the tool resolves under attribution — the bound §1 states, not a claim about the cluster. Numbers whose title matches their cite text throughout are correct and omitted; every number with ≥1 mis-cited site appears below with a per-number verdict.

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

### §4.7 Collapsing the spec-label map — the memo's own thesis, applied to itself

`cite-audit`'s attribution needs a label → shortname map. It was authored as a **hand-maintained copy** carrying a "Mirrors `coverage_map.py::_SPEC_LABEL_MAP`" comment — i.e. a partial, hand-maintained enumeration with a sync note instead of a mechanism, inside the PR whose thesis is that exactly that shape self-seeds. Shipping it would have been self-refuting.

**A third copy existed and was not in the brief**: `.claude/skills/elidex-plan-review/preflight.py:51` `SPEC_LABEL_REVERSE` — the plan-review preflight's own label parser, with its own hand-written case-insensitive fallback loop. Three copies, three lifecycles, no mechanism keeping them equal; adding a spec to one reached neither other.

**Collapsed to one source**: `.claude/tools/_webref/spec_labels.py` (NEW). A single `SPECS` tuple of `(shortname, canonical label, *aliases)`; **both directions and every alias are derived from it**, so it is direction-agnostic and a new spec reaches all three consumers by construction. All three now import it; the sync comment is deleted because it became untrue, and `preflight.py`'s two-pass case-insensitive lookup collapses to one lower-cased-key `get`.

Guarded by `TestSharedSpecLabelMap` (§6 item 5), including an **identity assertion** that `cite_audit`'s map *is* the shared object — so re-inlining a copy fails the suite rather than drifting quietly.

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

### §4.8 What the detector cannot see — re-derivable, not asserted

**Counts here are commands, not facts.** Every one of these numbers moves whenever *any* comment in the tree moves — this PR's own batches shifted them four times, and a Q6 rewrite that touched one docstring moved two of them again. Carrying them as memo facts would recreate exactly the hand-maintained-list failure the detector exists to kill, so each row ships **the command that derives it**; the figures below are a **dated illustrative snapshot (2026-07-27), not normative**. This is the same reason §6's exit criterion runs the tool instead of checking a list.

| Class | Re-derive (authoritative) | Snapshot 2026-07-27 (illustrative) | Why the tool cannot see it |
|---|---|---|---|
| (c) `§`-less step numbers | `grep -rnE '[Ss]tep [0-9]' crates/ --include='*.rs'`, then read for a nearby spec concept | ≥3 known, now swept (`html_form_proto.rs` Q6 block) | `_CITE_RE` requires a `§`; a bare "Step 2.1" has none |
| (d) right section, wrong **step** | per-anchor `.claude/tools/webref body html <anchor>` against each cited step | ≥21 known | the number resolves *and* its title matches the concept; only the step is stale |
| (e) org-only labels (`WHATWG §…`) | `grep -rn 'WHATWG §' crates/ --include='*.rs' \| wc -l` | ~333 cites | "WHATWG" names an org, not a spec — no shortname to resolve against |
| (f) attributed to another spec | `.claude/tools/webref cite-audit <shortname>` per spec | ~47 in the `§4.10` set, ~2750 tree-wide | audited against *their* spec by nothing here |
| unattributed bare `§` | `.claude/tools/webref cite-audit html --prefix 4.10 --show-unattributed` (or without `--prefix` for the tree) | ~169 in-set, ~6732 tree-wide | no spec nameable from the comment block |

**Post-sweep end state.** Re-derive with `.claude/tools/webref cite-audit html --prefix 4.10 --summary`. Snapshot 2026-07-27: 69 sections / 538 cites attributed to HTML, **12 unresolved / 46 cites** — and those 12 are *exactly* the named-OUT list (§4.5, §4.6, §10.1): `§4.10.3.1`, `§4.10.7.4`, `§4.10.7.5`, `§4.10.10.1` (the OUT half of its SPLIT), `§4.10.10.2`, `§4.10.10.3`, `§4.10.11.1`, `§4.10.15.2.4` (the 2 Slice-1-owned), `§4.10.18.4`, `§4.10.18.5`, `§4.10.18.6`, `§4.10.18.7`. **§6's exit criterion is met** — and it is met by *running the tool*, which is why the criterion survives the next comment edit and these figures do not.

### §4.9 Repairs found by looking — real, and not provably exhaustive

Each is verified; none is claimed to complete a class.

1. **`§4.10.23` has only TWO steps** (`body html resetting-a-form`): step 1 fires `reset`, step 2 invokes each resettable element's reset algorithm. **§7's earlier "step 4 → `elidex_form::reset_form`" would have authored a *new wrong citation* in a citation-repair PR** — the sharpest finding of the pass. `crates/dom/elidex-form/src/submit.rs:464` `reset_form` implements **step 2**.
2. **The `§4.10.22.4` entry-list step family (14 sites, verified 2026-07-27)**: the tree says "step 7 / 8 / 8.1 / 8.2 / 10". Verified against `body html constructing-form-data-set`: top-level steps are **1..9** with the per-control arms as **substeps of step 5** (5.6 select, 5.7 checkbox/radio, 5.8 file, …), and **top-level step 10 does not exist**. Sites: `submit.rs:359`/`:363`/`:393`/`:395`/`:402`/`:405`/`:413`/`:426`; `submit_tests.rs:80`/`:84`/`:166`/`:167`/`:198`/`:225`. Both files already in the touch set; re-verify each arm against the prose before writing.
3. **Email cited as Password**: `validation/mod.rs:46` and `validation/tests.rs:358` cite **§4.10.5.1.6 = "Password state"**; email is **§4.10.5.1.5 "Email state (type=email)"**. In-cluster, in touched files.
4. **`tests_html_form_proto.rs:144`** cites **§4.10.21 = "Constraints"** for form submission → §4.10.22.3 / §4.10.3.
5. **The local "Algorithm" block (`html_form_proto.rs:483`/`:484`/`:486`/`:508`/`:537`/`:558`/`:568`) — RESOLVED, and it produced the sweep's sharpest finding.**

   **What was actually wrong.** The doc list was not *misnumbered*; it was **missing a step**. It ran brand-check → submitter → submit-event → FormData → formdata (**five** items) while the code runs brand-check → submitter → **constraint-validation** → submit-event → FormData → formdata (**six**). The list predates the validation step and was never updated when it landed. That single omission *is* the off-by-one that showed up as "doc `3`" meaning the submit event while "code `Step 3`" meant validation.

   **Why stripping the spec-step claims beat re-anchoring them — the transferable part.** Re-anchoring would have renumbered the code comments to match the doc list. But the doc list was missing a step, so aligning onto it would have **silently deleted constraint validation from the narration** — turning a visible inconsistency into an invisible omission, and leaving a *verified* spec-step number sitting on top of a list that no longer described the function. Re-anchoring is only safe when the local list is already a faithful transcription; here it demonstrably was not, and there was no way to know that *without* first asking whether each item corresponds to a spec step at all. **That question is the fix**: item 1 is a brand check with no spec counterpart, which is what proves the block is engine narration rather than a transcription, which is what makes "renumber to match the spec" false by construction.

   Generalised: **when a local list and a spec disagree, do not assume the list is a mis-transcription of the spec — first establish whether it is a transcription at all.** A list that was never spec-shaped cannot be repaired by renumbering; it can only be repaired by saying what it is.

   **Applied**: every bare `Step N` in the block is now `Algorithm step N (engine-local)`; the list is six items and matches the code; and each item that *does* implement a spec step cites it, verified against `body html form-submission-algorithm` / `dom-form-requestsubmit` — step 2 → §4.10.3 `#dom-form-requestsubmit` step 1 (1.1/1.2); step 3 → §4.10.22.3 step 5.4 (→ §4.10.21.2, no-validate state §4.10.19.6); step 4 → §4.10.22.3 step 5.6; steps 5+6 → §4.10.22.3 step 7 → §4.10.22.4 step 7. (`form.reset()`'s own fold list at `:594+` is **not** in this class — its docstring header already carries §4.10.3 → §4.10.23 with steps 1 and 2 spelled out, and "Group β F-7 + F-8 fold" names the list as internal; nothing there reads as a spec step.)
6. **`<output>` IN/OUT re-verdicted under one rule.** §4.2 swept `components.rs:601`/`:607` but queued `vm/mod.rs:1075`, though all three describe the same `OutputDefaultValue` state machine — the line was drawn by comment *wording*, not by §4.5's concept rule. Under that rule all three are the same concept (`<output>` default-value state, reached from the reset algorithm): **all three are IN**, all → §4.10.12.


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

5. **The tool is code, so it is unit-tested** — `.claude/tools/_webref/test_cite_audit.py` (NEW, 13 tests, `unittest`, following the `test_inventory_diff.py` / `test_agent_brief.py` / `test_refresh.py` shape already in `_webref/`). It is outside the cargo workspace, so `mise run ci` does not run it; the gate runs it explicitly alongside its three siblings. ⚠ Same caveat for the type check: **Pyright is the only checker reaching `.claude/tools/**`**, it is on PATH locally (`/opt/homebrew/bin/pyright`) but `npx --no-install pyright` **fails** (package not cached) — so any hook or CI invoking it via `npx` will silently not run. Coverage maps 1:1 onto the defects that made it necessary:
   - **`TestStrictExitCode`** — `--strict` exits non-zero on an unresolved cite, zero when all resolve, and zero without the flag. *(`--strict` was a no-op: `cli.main` calls `args.func(args)` and discards the return value, so the bare `return 1` was dropped.)*
   - **`TestPrefixIsDottedComponent`** — `--prefix 4.10.2` matches `4.10.2` and `4.10.2.7` but **not** `4.10.20.3`. *(It used `str.startswith`, silently sweeping the whole `4.10.2x` range.)*
   - **`TestSpecAttribution`** (7 tests) — explicit label selects that spec; longest label wins (`WHATWG HTML` is not parsed as the bare `HTML` alias); bare cites inherit within a comment block; a non-comment line ends the block; `§0.5`-style plan-memo pointers land in UNATTRIBUTED rather than being audited as HTML; `--show-unattributed` lists them.
   - **`TestResolverExactness`** — a cited `§4.10.20.3` reports UNRESOLVED rather than passing because `§4.10.2` exists. *(`lookup_section` is prefix-tolerant and has **no** `exact` parameter — the first commit called it with `exact=True`, which Pyright flagged; the shipped guard is `hit[0] == section`.)*

   - **`TestSharedSpecLabelMap`** (6 tests) — both directions round-trip; every shortname is its own alias; lookup is case/space-insensitive; an unknown label returns `None` rather than a guess; **no alias is claimed by two specs**; and `cite_audit.LABEL_TO_SHORTNAME` **is** `spec_labels.LABEL_TO_SHORTNAME` (an identity assertion, so re-inlining the map fails the suite).

   - **`TestLabelWrapCarry`** (3) + **`TestPrefixScopesEveryReportedClass`** (2) — the label-wrap false negative (§4.0) and `--prefix` scoping the unattributed/other-spec counts, not just `by_section`.

   Verified green 2026-07-27: `python3 .claude/tools/_webref/test_cite_audit.py` → **26 tests OK**; full `_webref` suite **38** (26 + 6 + 5 + 1). *(Counts are re-measured each round, not recalled — an item whose job is proving the tool is checked must not itself be checked-by-recall; two earlier drafts of this line were stale.)*

   *(Note for future readers: the `sys.path.insert` at the top of the test file is the same shim `test_inventory_diff.py` / `test_agent_brief.py` / `test_refresh.py` already use to import `_webref` as a package from a script. Pyright flags it; it is the established precedent in this directory, **not** a defect to "fix".)*

**Gate**: `mise run ci`, then `/pre-push`, then **`/external-converge` from round 1**. The reactive escalation trigger ([[feedback_gate-miss-on-edge-dense-escalate-to-converge]]) has already fired twice on this memo — two gaps found pre-dispatch, then 1 CRIT / 28 IMP at review — and the #483 precedent this PR invokes did **not** converge in a single pass (it left three open carves). §2's honest re-statement (discovery is not canonical) is the same conclusion from the design side.

---

## §7 Layering check

- **No engine code moves** — no crate edge, no new `vm/host/` algorithm, no `elidex-ecs` touch; the `crates/**` diff is comments only. The PR *does* add tool code under `.claude/tools/`, outside the cargo workspace with no engine consumer (§5). ✓
- The sweep **reinforces** the mandate indirectly: `validation/mod.rs:1`'s module doc stops claiming the fieldset section for a validation module, and `validity_state.rs:154`'s bar list stops describing a predicate it no longer matches. ✓
- **⚠ Not a clean bill, and PR-A0 is about to make it worse before PR-A makes it better.** `run_form_check_validity` (`crates/script/elidex-js/src/vm/host/html_form_proto.rs:620-654`) **is** a form-level validation algorithm living in `vm/host/`, and this PR is about to author a *spec-blessed* `§4.10.21.2` comment on it — i.e. to document drift as though it were design. Carried mitigations:
  - a **concept → engine-independent-API mapping** in the rewritten comments: §4.10.21.1 candidacy → form-core's `is_constraint_validation_candidate`; "satisfies its constraints" → form-core's `validate_control`; §4.10.21.2 steps 1-4 → **`elidex_form::statically_validate_constraints`** (NEW) — PR-A part 4;
  - an explicit **hand-off line** at `html_form_proto.rs:620` naming PR-A's export, so the comment reads as "steps 5-7 here, steps 1-4 land in `elidex-form` next PR" rather than as a settled home.
- **⚠ The same treatment is owed to §4.10.23's twin.** "Resetting a form" spans **three** copies: `elidex_form::reset_form` (`crates/dom/elidex-form/src/submit.rs:464`, the correct engine-independent home for step 4) plus the fire-`reset`/honor-`preventDefault` half duplicated in `crates/script/elidex-js/src/vm/host/html_form_proto.rs:592-605` **and** `crates/shell/elidex-shell/src/content/form_input.rs:290-302`. PR-A0 is about to stamp §4.10.23 authority on all three. Same mitigation as §4.10.21.2: each repaired comment carries the concept → engine-indep-API mapping (**§4.10.23 step 2** → `elidex_form::reset_form` — §4.10.23 has only two steps; an earlier draft of this line said "step 4", which would have authored a *new wrong citation* inside a citation-repair PR, §4.9 item 1) and names the duplication rather than blessing it. **The shell copy folds into PR-A's `#11-shell-submit-interactive-validation`** (its second facet is already "an independent second copy of the submission event sequence" — the reset sequence is the same defect).
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

| **`#11-unattributed-spec-citations`** | **NEW** (from §4.0) | (1) Real gap? yes — thousands of cites tree-wide carry a bare `§` whose spec a reader cannot determine from its comment block (snapshot 2026-07-27: ~6732 tree-wide, ~169 in the `§4.10` set; earlier drafts said 234, which wrongly folded in the other-spec cites). **The figure is illustrative — re-derive, do not cite this line.** A citation a reader cannot attribute is a defect independent of resolution. (2) Blocked by structure? no, but it spans every spec and every crate — sweeping it here would collide with every active lane, and it is a different concept axis from this cluster. (3) Non-regressing to defer? yes — pre-existing, and `cite-audit` now *reports* the count so it cannot go quiet again. (4) Durable home? `project_open-defer-slots.md`. **Re-derive, don't re-discover** — the measurement is one command: `.claude/tools/webref cite-audit html --show-unattributed` (whole tree) or `.claude/tools/webref cite-audit html --prefix 4.10 --show-unattributed` (this set — `--prefix` now scopes the unattributed list too; it previously filtered only `by_section`, so a scoped run still dumped the whole tree). The count is printed in every run's header, so the next toucher gets the current number rather than this memo's snapshot. **Trigger**: the next cross-spec citation pass, or a review mis-attributing a bare cite. **Date**: 2026-12. |

**Per-PR budget — re-audited 2026-07-27**: **2 new slots**, `#11-citation-anchor-level-verification` (§9-Q1) and `#11-unattributed-spec-citations` (§4.0), against the ≤3 cap — **one slot of headroom**. Nothing else in this round created deferred work: the shared-spec-label collapse (§4.7) is *done here*, not slotted, and the §4.10.12-15 cascade remainder goes to an existing tracked program (§10.1) rather than a new slot. Draft 2 claimed "zero deferred work" while §9-Q1 was open — corrected.

### §10.1 Out-of-cluster inventory — a deliverable, not a silence

The §3.1 step-(0) enumeration surfaces drift the cluster rule (§4.5) pushes **out**. Sweeping it here would collide with every active lane; leaving it unrecorded would convert a measured gap into a silent one. It is therefore filed as **new queue entries in `memory/project_html-section-renumber-sweep.md`**, which is precisely the tracked program that exists to hold it:

| Family | Cites | What it is | Correct target |
|---|---|---|---|
| `§4.10.10.2` | ~21 | `HTMLOptionsCollection` / option selectedness / "ask for a reset" | **§2.6.4.3 The HTMLOptionsCollection interface** (interface sites) / §4.10.7 (select) per site — *not* §4.10.10, which draft 2 gave; webref-verified 2026-07-27 |
| **the §4.10.12-15 element cascade** (out-of-cluster remainder) | ~31 at 2026-07-27 — re-derive per §4.8 | `<output>` IDL wrappers citing §4.10.13 (3); `<progress>` citing §4.10.14 (9); `<meter>` citing §4.10.15 (11); `<select>`/`<datalist>` citing §4.10.10 (8) | §4.10.12 / §4.10.13 / §4.10.14 / §4.10.7 / §4.10.8 respectively. ⚠ `html_meter_proto.rs:20`'s "`<progress>` … §4.10.14 vs §4.10.15" contrast is **inverted**: progress = §4.10.13, meter = §4.10.14 |
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

**Carried edits — split by which branch owns the file.**

**In this branch (done):**
1. **`docs/plans/2026-07-form-derived-state-reconciliation.md` §6** — slicing-table rows for **A0** and **A**, plus the prereq-chain sentence. Load-bearing, not bookkeeping: it is what makes CLAUDE.md's edge-dense **base case** apply to both PRs, which is what §2/§3's split argument rests on. The umbrella is committed on `main`, so this branch can edit it.
2. **PR-A §4.4** — the reciprocal ownership rows, including that the `§4.10.20.x` / `§4.10.21.3` / `§4.10.21.4` / `§4.10.21.5` families are **already swept by A0** on this branch, so PR-A rebases onto the corrected numbers rather than re-citing the old ones.

**NOT in this branch — owned by `domform-slice1`:**
3. **Slice-1 §6-C's reciprocal rows.** That memo lives in a different worktree; reaching across branches is the shared-tree hazard the worktree discipline exists to prevent, so the hand-off is **owned by the Slice-1 branch**. It must (a) **add** rows for `crates/css/elidex-css/src/selector/tests/matching.rs:767`/`:788`/`:839` — §4.6 assigns them to Slice 1 on the semantic-inversion rule and §6-C lists none of them today; (b) **correct** the row naming `matching.rs:299`, which carries **no `§` at all** (verified 2026-07-27); (c) record that `:715`/`:727` were PR-A0-owned and **are now swept** (→ §4.16.3 `#selector-required` / `#selector-optional`).
   **Fallback, stated so it is decided rather than discovered**: if Slice 1 declines (a), all five `tests/matching.rs` sites flip to **IN** and a follow-on PR-A0 sweeps them.

**NOT in this branch — landing admin, applied at ship (post-merge), content ready to paste:**
4. **`memory/project_html-section-renumber-sweep.md`** — per the landing discipline, memory files are updated at ship, not in the branch. Ready content:
   - **Fully discharged**: line 34's constraint-validation family; line 36's SubmitEvent/FormDataEvent halves (incl. the `events_misc.rs` twins); the 2026-07-18 Slice-0b entry's item (a) (the textarea/`pattern` pair `validation/mod.rs:210` + `validation/tests.rs:478`).
   - **Not discharged**: that entry's item (b), `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — out of cluster, stays queued.
   - **New queue block**: §10.1's out-of-cluster inventory. **Re-run `.claude/tools/webref cite-audit html --prefix 4.10` at ship and paste the current numbers** — do not copy this memo's snapshot; every comment edit between now and merge moves them (§4.8).
   - Heading shape: `## ✅ DONE — #N ✅ MERGED (sha, date, comment-only)`.
5. **`memory/project_open-defer-slots.md`** — register the two new slots (`#11-citation-anchor-level-verification`, `#11-unattributed-spec-citations`) in the L3 cluster, each with **its re-derivation command rather than a count** — same reason as above.

Items 4-5 are listed explicitly here **so they cannot evaporate between merge and follow-up** ([[reference_spawn-task-chips-not-durable]]).
