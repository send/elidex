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

## §1 Ideal anchor — a citation is a claim, and every claim in the cluster gets checked by the same rule

`§X.Y = <title>` in a comment is a checkable assertion. The repo has ~70 such assertions across the form constraint-validation / submission cluster, and the R1 probe checked only one failure mode.

**The method's defect, stated so it cannot recur.** "Does `heading --exact html <n>` fail?" detects a **non-resolving** number. It cannot detect a **valid-but-misused** one: `§4.10.21.3` resolves cleanly to "The constraint validation API", so the probe passed it — while 18 of its 19 sites use it to mean the *form submission algorithm* (§4.10.22.3); `grep -rn '§4\.10\.21\.3' crates/ --include='*.rs' | wc -l` → 19, per-site table in §4.2, verified 2026-07-26. A number that resolves to the wrong concept is exactly as wrong as one that resolves to nothing, and strictly harder to see.

**The corrected method (§3.1)**: for every `§4.10.x` cite in the cluster, resolve the number to its **title**, then check that title against **the concept the surrounding comment is actually about**. Non-resolution becomes one outcome of the check, not the whole check.

---

## §2 Coupled invariants

**Edge-density judgment: this is NOT edge-dense**, and the §2 is short for that reason rather than padded. It is doc-only, has zero behavior surface, and has one canonical algorithm (resolve → compare → replace). It does carry two coupling facts worth naming, so they are named rather than dressed up as a matrix.

| # | Invariant | What it forces |
|---|---|---|
| **I1** | **Concept-scoped completeness** — a partial repair across semantic siblings is the failure #483 was forced to correct by its external reviewer three commits back ([[feedback_semantic-sibling-selfseed-and-regate-breadth]]). | The unit is the *concept cluster*, not the file or the pattern. §4 publishes a full IN/OUT accounting so no site in the cluster is unnamed. |
| **I2** | **Successor-PR fate** — some cites annotate code that PR-A / Slice 1 delete or rewrite. | The exclusion criterion must be falsifiable per site (§4.6), not directory-shaped. A cite that **outlives** its code change is IN even when it sits in a successor PR's file. |

**I1 × I2**: completeness and hand-off pull in opposite directions — completeness says "take everything", fate says "leave what dies". The resolution is that fate is evaluated **per site against the successor's actual diff**, so the two never disagree about the same line; §4.6 shows the four contested sites resolving 2 IN / 2 OUT.

---

## §3. Spec coverage map

Doc-only: no branch of any algorithm is implemented or altered. Each row is a **citation target** the sweep introduces or corrects, with the concept it must annotate.

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21.1 Definitions | candidate / barred / satisfies constraints | candidacy vocabulary | ~20 comment sites migrating off `§4.10.20.3` (verified 2026-07-26, §4.1) | ✓ | no (doc-only) |
| WHATWG HTML §4.10.21.2 Constraint validation | statically validate, steps 1-7 | pure walk (1-4) vs `invalid`-firing (5-7) | `html_form_proto.rs:538`/`:613`, `validity_state.rs:475`, `well_known.rs:1145`, `globals.rs:287` | ✓ | no |
| WHATWG HTML §4.10.21.2 Constraint validation | interactively validate | the `requestSubmit` validation arm | `html_form_proto.rs:537` (currently mis-cited `§4.10.21.3`) | ✓ | no |
| WHATWG HTML §4.10.21.3 The constraint validation API | control-level CVA members | `willValidate` / `checkValidity` / `validationMessage` / `setCustomValidity` / `ValidityState` | ~15 sites migrating off `§4.10.20.2`/`.3`/`.4` (verified 2026-07-26, §4.1); `validation/mod.rs:188` **already correct — stays** | ✓ | no |
| WHATWG HTML §4.10.3 The form element | form-level members | `form.checkValidity()` / `reportValidity()` / `requestSubmit()` / `submit()` / `reset()` | `html_form_proto.rs:458`/`:478`/`:576`, `submit.rs:276`/`:289`, + the `§4.10.21.4` family | ✓ | no |
| WHATWG HTML §4.10.3 The form element | `requestSubmit()` **step numbers** | steps 1 / 1.1 / 1.2 — the in-tree "step 2 / 2.1 / 2.2" are stale | `submit.rs:276`/`:289`, `submit_tests.rs:663`, `tests_form_submit.rs:14`/`:26`/`:131`/`:200` | ✓ (7 sites, verified 2026-07-26 — enumerated in §4.3) | no |
| WHATWG HTML §4.10.22.3 Form submission algorithm | steps 5 / 5.4 / 5.6 / 7 | the submission algorithm proper — the concept 18 of 19 `§4.10.21.3` sites actually mean; the in-tree "step 8.4 / 8.5" are stale *steps* | `well_known.rs:1136`/`:1140`, `html_form_proto.rs:28`/`:458`/`:479`, `form_data.rs:33`/`:183`/`:233`/`:269`, `event_target_dispatch.rs:744`/`:829`, `tests_form_submit.rs:3`/`:8`/`:27`/`:214`/`:234` | ✓ (18 sites, verified 2026-07-26 — enumerated in §4.2) | no |
| WHATWG HTML §4.10.22.4 Constructing the entry list | entry-list construction + the `formdata` event | `submit.rs:306`; **`html_form_proto.rs:33`** (formdata event — *not* §4.10.3) | ✓ | no |
| WHATWG HTML §4.10.22.10 The SubmitEvent interface | interface | `vm/mod.rs:1496` | ✓ | no |
| WHATWG HTML §4.10.22.11 The FormDataEvent interface | interface | `vm/mod.rs:1500` | ✓ | no |
| WHATWG HTML §4.10.23 Resetting a form | reset algorithm | `submit.rs:457` (`reset_form`, currently `§4.10.21.5`), `html_form_proto.rs:576` | ✓ | no |
| WHATWG HTML §4.10.18.3 Association of controls and forms | `form=` IDREF association | `submit.rs:322` (currently `§4.10.15.3` — an entry-list number on an association claim) | ✓ | no |
| WHATWG HTML §4.10.19.4 Setting minimum input length requirements: the minlength attribute | the **shared** `for=input,textarea` dfn | `validation/mod.rs:227` — serves TextArea too, so **not** §4.10.5.3.1 (input-only) | ✓ | no |
| WHATWG HTML §4.10.11 The textarea element | textarea does not support `pattern` | `validation/mod.rs:210`, `validation/tests.rs:478` — the two sites the renumber-sweep memo queued for "the next PR that touches those files" | ✓ | no |
| WHATWG HTML §4.10.8 The datalist element | datalist-ancestor bar | new cite consumed by PR-A part 3; introduced here so PR-A's code lands on a correct number | ✓ | no |
| WHATWG HTML §4.10.5.1.1 Hidden state (type=hidden) | `<input type=hidden>` bar | `validation/mod.rs:32` — currently uncited | ✓ | no |

**Breadth**: K=1 spec (WHATWG HTML), M=16 rows → **single-PR scope**. Doc-only, one concept cluster, zero behavior surface; the row count enumerates citation *targets*, not implemented branches.

### §3.1 User-input touch audit

**None.** No opcode, native, dispatch path, or production statement is added or altered — the diff is comments and doc-comments only. Exposure delta: zero. The audit method itself is the substance of this PR, so it is documented as a procedure rather than a code path:

For each candidate site: (1) extract the cited `§number`; (2) `webref heading --exact html <number>` → title, or *no match*; (3) read the surrounding comment and name the **concept** it describes; (4) `webref dfn html '<concept>'` → the section that actually defines it; (5) the site is **correct** iff (2) and (4) agree; (6) fix number **and** prose, and re-check any **step** number against `body html <anchor>` — steps drift independently of sections (§4.5 found two whole families of stale steps).

---

## §4 Site accounting (full IN/OUT — no site in the cluster unnamed)

All counts verified 2026-07-26 by the stated grep.

### §4.1 Non-resolving numbers (the R1 probe's only detector)

`grep -rn '§4\.10\.20\.\|§4\.10\.14\.2\|§4\.10\.15\.2\|§4\.10\.15\.3\|§4\.10\.7\.3\.1\|§4\.10\.21\.4' crates/ --include='*.rs' | wc -l` → **51**, plus the bare `§4.10.15` module doc at `validation/mod.rs:1` = **52**.

| Pattern | Hits | Resolves to | Correct target |
|---|---|---|---|
| `§4.10.20.` | 31 | *no match* | control CVA → §4.10.21.3; candidacy → §4.10.21.1; form-level → §4.10.3; validate-the-constraints → §4.10.21.2 |
| `§4.10.21.4` | 10 | **"Security"** | `requestSubmit()` → §4.10.3; **`html_form_proto.rs:33` is the formdata event → §4.10.22.4** (the blanket map mis-routes it) |
| `§4.10.15.2` | 5 | *no match* | 4 are literally **`§4.10.15.2.4`** (`:required`/`:optional`, `matching.rs:137`/`:284`, `tests/matching.rs:715`/`:727`) → §4.16.3; `validation/mod.rs:227` (`minlength`) → **§4.10.19.4** |
| `§4.10.15.3` | 2 | *no match* | `submit.rs:306` → §4.10.22.4; `submit.rs:322` (**association**) → §4.10.18.3 |
| `§4.10.7.3.1` | 2 | *no match* | `validation/mod.rs:210`, `validation/tests.rs:478` → §4.10.11 |
| `§4.10.14.2` | 1 | *no match* | `validation/mod.rs:57` (`ValidityState`) → §4.10.21.3 |
| bare `§4.10.15` | 1 | **"The fieldset element"** | `validation/mod.rs:1` module doc → §4.10.21.1 |

### §4.2 Valid-but-misused numbers (the cluster the probe could not see)

`grep -rn '§4\.10\.21\.3' crates/ --include='*.rs' | wc -l` → **19**. `heading --exact html 4.10.21.3` **succeeds** ("The constraint validation API") — yet **18 of 19** annotate the *form submission algorithm* (per-site table below, verified 2026-07-26):

| Site | Comment says | Real target |
|---|---|---|
| `well_known.rs:1134` | `reset` event name | §4.10.23 / §4.10.3 `#dom-form-reset` |
| `well_known.rs:1136` | `submit` event name, "step 8.4" | §4.10.22.3 (step number also stale) |
| `well_known.rs:1140` | `formdata` event name, "step 8.5" | §4.10.22.3 / §4.10.22.4 (ditto) |
| `html_form_proto.rs:28` | `submit()` "step 5" | §4.10.22.3 |
| `html_form_proto.rs:458` | `form.submit()` "step 5" | §4.10.22.3 |
| `html_form_proto.rs:479` | "step 8.4-8.5" | §4.10.22.3 — **half-swept on the same line as a `§4.10.21.4` cite at `:478`**, so a pattern-scoped sweep would fix one and leave the other |
| `html_form_proto.rs:537` | "interactively validate the constraints" | **§4.10.21.2** |
| `form_data.rs:33`/`:183`/`:233`/`:269` | "step 7 construct the entry list" | §4.10.22.3 step 7 / §4.10.22.4 |
| `event_target_dispatch.rs:744`/`:829` | "step 8.4" / "step 8.5" | §4.10.22.3 |
| `tests_form_submit.rs:3`/`:8`/`:27`/`:214`/`:234` | submission arms | §4.10.22.3 — **`:3` is likewise half-swept** (`§4.10.21.3 + §4.10.21.4` on one line) |
| **`validation/mod.rs:188`** | custom validity, the constraint validation API | **§4.10.21.3 — already correct, stays** |

### §4.3 Stale *step* numbers (drift independent of section numbers)

- **`requestSubmit()` steps are 1 / 1.1 / 1.2**; 7 sites carry "step 2 / 2.1 / 2.2" (verified 2026-07-26): `submit.rs:276`, `:289`, `submit_tests.rs:663`, `tests_form_submit.rs:14`, `:26`, `:131`, `:200`. *(PR-A's §4.2 reproduced "step-2.1" from memory — the one pair in the previous delta that was not looked up. Corrected there too.)*
- **Form-submission steps are 5 / 5.4 / 5.6 / 7**; the in-tree "step 8.4 / 8.5" (5 sites, verified 2026-07-26 — see §4.2) are stale.

### §4.4 Further non-resolving `§4.10.x` cites in cluster files, previously unnamed

`html_form_proto.rs:263`, `:300`, `:338`, `:443`; `validation/mod.rs:368`; `validity_state.rs:299`; `tests_html_form_proto.rs:288`; `vm/mod.rs:1094`, `:1105`. Each gets the §3.1 procedure applied individually — they are listed so the accounting is exhaustive, not because one blanket map covers them.

Also folded in: `submit.rs:457` (`reset_form`, cites `§4.10.21.5`) → §4.10.23; `html_form_proto.rs:576` (`form.reset()`) → §4.10.3 `#dom-form-reset`; `vm/mod.rs:1496`/`:1500` (SubmitEvent / FormDataEvent) → §4.10.22.10 / §4.10.22.11.

### §4.5 Two prose repairs that are not renumbers

1. **`submit.rs:276-279` states a falsehood**: `<input type=image>` is *not* "classified as `FormControlKind::SubmitButton` by `from_element`" — `from_type_str` (`crates/dom/elidex-form-core/src/lib.rs:253-282`) has no `"image"` arm, so it becomes `TextInput`. Correct the sentence and point it at `#11-input-image-state`. Deleting a false claim, not renumbering one.
2. **`validity_state.rs:152`/`:154`**: the § number is at `:152`, the bar-list prose at `:154`. This PR fixes the **number**; PR-A adds the new bars to the **prose** (its own behavior). Split so neither PR half-states the other's change.

### §4.6 Exclusion criterion — by fate, per site, falsifiable

> **A cite is excludable only when the code it annotates is deleted by the successor PR. A cite that outlives the code change is IN**, even in a successor's file.

Applied to the four contested `elidex-css/selector/` sites (verified against the Slice-1 memo's §5 deletion set):

| Site | Annotates | Slice-1 fate | Verdict |
|---|---|---|---|
| `matching.rs:137` | `is_requirable_element`'s doc | **deleted** (Slice-1 §5-R4) | **OUT** — Slice 1 |
| `matching.rs:284` | the `:required` arm of `match_form_pseudo_class` | **rewritten** (§5-R1/R4) | **OUT** — Slice 1 |
| `tests/matching.rs:715` | `required_does_not_match_fieldset`'s assertion — still true after Slice 1 | test body re-seeded; **the assertion and its comment survive** | **IN** |
| `tests/matching.rs:727` | `optional_does_not_match_fieldset` — inserts `ElementState::default()`, **survives byte-identical** | untouched | **IN** |

**Slice-1's §6-C has no row for either test site**, so directory-shaped exclusion would have orphaned them. **Reciprocal edit**: add rows to Slice-1 §6-C recording that `matching.rs:137`/`:284` are Slice-1-owned and the two test sites are PR-A0-owned, so both memos agree per site.

**Also OUT**: `tests_html_input_proto.rs:176` (`§4.10.5.1.16` → §4.10.5 `#dom-input-indeterminate`) — a different concept (input type-state index shift), in a file no PR in this chain touches. Stays queued in `memory/project_html-section-renumber-sweep.md`.

---

## §5 Behavior deltas

**None.** Comments and doc-comments only; no production statement, signature, or test assertion changes. Success = `mise run ci` green with a diff containing no non-comment line.

---

## §6 Test plan

There is nothing to assert — the deliverable is prose. Verification is procedural:

1. **`mise run ci` green**, including the `-D warnings` `doc` job (catches a broken intra-doc link introduced while editing doc-comments).
2. **Diff discipline**: `git diff -U0 | grep '^[+-]' | grep -v '^[+-][+-]' | grep -vE '^[+-]\s*(//|///|//!|\*)'` must be **empty** — the mechanical proof that the PR is doc-only.
3. **Citation re-verification**: every `§`-number introduced by the diff re-checked with `webref heading --exact`, and every **step** number re-checked with `body html <anchor>`.
4. **Residual scan**: after the sweep, `grep -rn '§4\.10\.20\.\|§4\.10\.21\.4\|§4\.10\.14\.2\|§4\.10\.15\.3\|§4\.10\.7\.3\.1' crates/ --include='*.rs'` returns only the sites §4.6 marks OUT, and `§4.10.21.3` survives only where §4.2 says it should.

**Gate**: `mise run ci`, then `/pre-push`, then `/external-review` (single-pass). Doc-only with no behavior surface does not meet the edge-dense converge trigger (§2) — this is the one PR in the chain where a single pass is the right call, and §2 states why.

---

## §7 Layering check

- **Doc-only** — no code moves, no crate edge, no `vm/host/` algorithm, no `elidex-ecs` touch. ✓
- The sweep **reinforces** the Layering mandate indirectly: `validation/mod.rs:1`'s module doc stops claiming the fieldset section for a validation module, and `validity_state.rs`'s bar list stops describing a predicate it no longer matches. ✓
- **ECS-native**: N/A — no component read or written. ✓

---

## §8 Line-count budget

Doc-only; net line count is approximately flat (a few multi-line comments reflow). Largest touched files: `html_form_proto.rs` (680), `validity_state.rs` (518), `tests_form_submit.rs` (213), `form_data.rs`, `submit.rs` (546), `validation/mod.rs` (430). **No file approaches 1000**; no split obligation.

---

## §9 Open questions for `/elidex-plan-review`

- **Q1 — is the §3.1 method sufficient?** It catches non-resolution *and* misuse. It does **not** catch a cite that is right about the section but wrong about the anchor within it. Should anchor-level checking be in scope, or is that a further sweep?
- **Q2 — the `§4.10.22.3` vs `§4.10.22.4` boundary** for the entry-list sites: `form_data.rs`'s "step 7 construct the entry list" is a step *of* §4.10.22.3 that *invokes* §4.10.22.4. Recommendation: cite the invoking step (§4.10.22.3 step 7) where the code implements the submission step, and §4.10.22.4 where it implements entry construction itself. Confirm the split.
- **Q3 — §4.6's fate criterion**: applied per site above. Confirm the four verdicts, and confirm the reciprocal Slice-1 §6-C rows are the right coordination shape.
- **Q4 — scope floor**: §4.4 lists 9 further cites in cluster files that no blanket map covers. Confirm they are in scope (recommendation: yes — leaving known-wrong cites in a file this PR is already editing is exactly the partial repair I1 forbids).

---

## §10 Defer slots

**Zero new slots.** PR-A0 creates no deferred work: every site it identifies is either fixed here or assigned to a named owner (Slice 1 via §4.6, or the pre-existing `project_html-section-renumber-sweep.md` queue).

**Carried memory edit**: `memory/project_html-section-renumber-sweep.md` — mark the two queued textarea/`pattern` sites (`validation/mod.rs:210`, `validation/tests.rs:478`) discharged; record that the constraint-validation + form-submission cluster is swept, with the §4.6 OUT list (the 2 `matching.rs` sites → Slice 1; `tests_html_input_proto.rs:176` → still queued); and record the **method** (§3.1), since the resolution-only probe is the reusable mistake.

---

## §11 Coordination

| Lane | State (verified 2026-07-26 via `git worktree list` / `git rev-list --count` / `gh pr list`) | Overlap | Ordering |
|---|---|---|---|
| **PR-A** (same worktree/branch) | plan-memo | PR-A0 renumbers comments in files PR-A then edits (`validation/mod.rs`, `submit.rs`, `validity_state.rs`, `html_form_proto.rs`) | **PR-A0 first.** PR-A rebases; its remaining prose repairs are only those describing **its own** new behavior (the `validation/mod.rs:12` bar list). |
| **Slice 1** (`elidex-wt-slice1`, `domform-slice1`, 0 commits) | plan-memo | the 4 `elidex-css/selector/` sites, split 2 IN / 2 OUT by §4.6 | PR-A0 takes the 2 IN; Slice 1 keeps the 2 OUT + gains reciprocal §6-C rows |
| **VM lane** (`elidex-wt-vmp4`, `vm-p4-plan`, 2 commits unpushed) | plan not converged | PR-A0 edits comments in `vm/mod.rs`, `well_known.rs`, `globals.rs`, `object_kind.rs`, `vm/host/*` | doc-only → conflict risk is textual, not semantic; PR-A0 lands first |
| **Layout** (`terminal-z-c3a-impl`, 16 commits, **PR #488 open**) | mid-converge | none | none |
| **History** (`session-history-slice-b`, 21 commits, **PR #487 open**) | — | none | none |
