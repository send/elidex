# Plan — Slice A-i: one spec-label map in the generic tree

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i**. **Branch**:
`webref-cite-audit-tool`. **Status**: plan-memo, **draft 3**. `/elidex-plan-review` before implementation.

⚠ **Draft 3 is a record, not a specification.** Drafts 1-2 were 404 and 462 lines and drew 1 CRIT / 28 IMP
and 1 CRIT / 38 IMP. Of round 2's 38, **one** was a defect in the change; the rest were defects in its
description — a decision stated at six sites and updated at one, an enumeration whose members were wrong, a
delta table restating what a diff shows. The umbrella's *review cost tracks blast radius* constraint is the
response: **this slice's canonical statement of what the code does is the diff and the tests.** The memo
records decisions, the measurements behind them, and what must be pinned. Where a claim would restate the
diff, it is not made.

Quantities are printed by `docs/plans/2026-07-citation-hygiene-A-rederive.sh`; the memo cites the block name.
`lanes` / `staleclaims` are author-local and excluded from `all`.

### §0.1 What A-i is

`origin/main` carries one spec-label enumeration **three** times — `coverage_map._SPEC_LABEL_MAP`,
`cli.COMMON_SHORTNAMES`, and `preflight.SPEC_LABEL_REVERSE`. A-i creates
`.claude/tools/_webref/spec_labels.py`, **pinned map only**, and collapses **the two in the generic tree**
onto it.

⚠ **`preflight.py` is not touched.** Its copy migrates in **A-ii**. Drafts 1-2 both tried to land it here and
both regressed the gate, in opposite directions — measured on `origin/main` with the tools tree absent:

| | default mode | `--no-verify --no-grep-pass` |
|---|---|---|
| `origin/main` | exit 1 (`webref tool missing`) | **exit 0**, correct summary |
| guarded import (draft 1) | **exit 0** — fail-open | exit 0 |
| hard import (draft 2) | traceback | **traceback** |

Preserving both cells needs a capability check at the verification stage, suppressed by `--no-verify` —
which is A-ii's act-site 1. **The gate's copy is not separable from the gate's failure semantics.** So K1
completes across A-i + A-ii, and A-i is **entirely inside the generic tree**: no adapter file, no gate
semantics, no CI topology.

**A-i's spec set is unchanged (the same 12 pinned specs); 9 additional *spellings* resolve** — the shortnames
themselves — from the shortname-as-own-parse-key rule, not from a widened alias list. → `rederive keysets`

---

## §0.5 Spec citation table

A-i implements no spec logic. Both labels below are pinned by `SPECS`, per the umbrella's *a slice may only
cite labels its own resolver maps*. Looked up with `.claude/tools/webref`, nothing from memory.
→ `rederive citations`

| Cite | § | Exact title | Anchor |
|---|---|---|---|
| `WHATWG HTML §4.10.21` | HTML §4.10.21 | Constraints | `#constraints` |
| `WHATWG Fetch §2.2.5` | Fetch §2.2.5 | Requests | `#requests` |

Two rows, two **distinct pinned specs**, so K=2 and the table is not one spec twice. ⚠ Draft 1 justified the
second as "exercises the shortname-as-parse-key rule" and draft 2 retracted that here while leaving it
standing in §3 — measured, `WHATWG Fetch` is `entry[1]`, the canonical label, and resolves identically at
baseline; the spellings that exercise the rule are `Fetch` / `fetch`, which `origin/main` does not resolve
(its only aliases are `HTML`, `DOM`, `URL`).

---

## §1 Ideal anchor

A dedup that moves the **table** and leaves the **prose** describing it scattered has not collapsed the
decision surface — it has moved it. `DESIGN.md`'s closing rule for the generic core is the operative one:
*keep new generic behavior free of elidex-specific file paths, and put elidex policy in adapter commands or
documentation.*

**The unit of this edit is the named artifact**, not the file and not the code branch. Every occurrence of a
Slice-B artifact name, every elidex file path, and every copy-count claim is either rewritten or explicitly
assigned — and the enumeration of those occurrences is **derived**, not authored (§4.1).

---

## §2 Coupled invariants

- **K1 — one enumeration in the generic tree.** After A-i, `coverage_map` and `cli` import rather than
  enumerate. `preflight`'s copy is A-ii's; K1 completes there.
- **K2 — the generic core names no elidex file path.** An **absolute**, not a delta: A-i discharges
  `origin/main`'s one pre-existing instance (`cli.py`'s `.claude/skills/elidex-review/axes.md.`) by the same
  by-role rewrite it applies to its own. It is in A-i's tree, A-i already edits that file, and Slice C — the
  earlier routing target — has no `cli.py` mandate.
- **K3 — the generic core names no Slice-B artifact.** `cite-audit` and `_catalog` are absent from
  `.claude/tools/_webref/` and `.claude/skills/` (matching `origin/main`, measured 0 each); `webref_data` is
  absent from `spec_labels.py` (it legitimately backs eight other command modules).
- **K4 — labels resolve identically.** Strict superset over the same 12 specs; `origin/main`'s 15
  `SPEC_LABEL_REVERSE` pairs vendored as a literal and frozen.

**Pairwise intersections** — they cannot be applied one at a time:

| pair | intersection |
|---|---|
| K1 × K3 | every site asserting the copy **count** also names its **members**; after A-i the count changes *and* `cite_audit.py` stops being a member. **Five sites** — §4.2 |
| K1 × K2 | the docstring's consumer list is both an enumeration and a place elidex paths appear; a **second** such list lives on `LABEL_TO_SHORTNAME` |
| K2 × K3 | the same prose usually carries both, so it is one rewrite — `cli.py`'s blurb comment names an elidex consumer *and* a B artifact |
| K1 × K4 | deleting the aliases must leave the map byte-identical, which is what makes it safe in a refactor slice (`rederive keysets`) |
| K3 × K4 | `coverage_map`'s last-resort reverts to `origin/main` verbatim — a K4 requirement, and the reason its branch-new docstring describing B's catalog must go |

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | n/a — no spec logic | a canonical label pinned by `SPECS` | §4.2 — `spec_labels.py`'s `SPECS` tuple | ✓ | no |
| WHATWG Fetch §2.2.5 Requests | n/a — no spec logic | a second canonical label pinned by `SPECS` | §4.2 — same tuple | ✓ | no |

**Breadth**: measured by the gate on this memo.

### §3.1 Discovery method

No web-content input flow; the inputs are developer-authored comment text. Numbers are measured against
`origin/main`, never the branch. **A claim about where something occurs is grepped by concept, not by
string** — and a claim about *prose* is checked by a grep over prose occurrences, not over file assignments.

---

## §4 The edit set

### §4.1 The rule this slice is built on

⚠ Four review rounds running, the root finding has been *a read whose write-path the draft changes, without
reconciling the other readers of that state.* It is now a command: **before writing an edit-set row for a
piece of state, run `rederive readers <symbol> [ref]` and assign every line it prints.** The census ranges
over a ref (defaulting to `origin/main`) and separates code from prose, because every failed edit set
assigned code and left prose.

Census for the two symbols A-i removes:

| symbol | code | prose |
|---|---|---|
| `_SPEC_LABEL_MAP` | `coverage_map.py` :13 :30 :31 | `preflight.py:48` — the "keep in sync" comment, **A-ii's** (it is in the adapter) |
| `COMMON_SHORTNAMES` | `cli.py` :27 :80 | the literal's own body: blurb lines and the `cite-audit` Examples line |

`SPEC_LABEL_REVERSE`'s census belongs to **A-ii** now, including its two gate-output readers
(`preflight.py` :409, :422) and its **four** plan-memo readers (measured; drafts said five, a branch-era
figure). One of those registers `#11-preflight-css-module-labels`, whose subject is that map — A-ii
dispositions it. → `rederive readers`

### §4.2 What changes, by named artifact

| artifact | change |
|---|---|
| `spec_labels.py` — `SPECS` + the three derived dicts | new file; **8 parse aliases deleted** (measured inert: the map is byte-identical without them, since each alias lowercases to its own shortname) |
| `spec_labels.py` — module docstring | "Four sites" → **two in the generic tree**; drop the `cite_audit.py` bullet (K3); rewrite the `preflight.py` bullet **by role** (K2) |
| `spec_labels.py` — the `LABEL_TO_SHORTNAME` comment's load-time consumer list | same two rewrites — it is a **second** consumer list |
| `spec_labels.py` — the alias rationale, the tuple-shape line, the in-comprehension alias comment, the variadic annotation | delete or narrow; they describe machinery A-i removes |
| `spec_labels.py` — the catalog paragraph, both function docstrings' catalog clauses | absent in A-i; **B** reinstates them with the fall-through |
| `cli.py` — blurb derivation | import `SHORTNAME_TO_BLURB`; the derived block must reproduce `origin/main`'s literal byte-identically (S3b) |
| `cli.py` — blurb comment | rewrite without the B artifact name (K3) |
| `cli.py` — the `--help` epilog's `webref cite-audit …` Example line | **delete** — shipped user-facing help for a subcommand absent at A-i's head |
| `cli.py` — `.claude/skills/elidex-review/axes.md.` | **by-role rewrite** (K2, absolute) |
| `coverage_map.py` — `_spec_label` | delegate to `label_for`; keep `origin/main`'s last-resort `.upper().replace("-", " ")` **verbatim** |
| `coverage_map.py` — `_spec_label`'s branch-new docstring | delete the catalog clauses; both are false without the fall-through |
| `DESIGN.md` — the `spec_labels.py` bullet | stated verbatim below |
| `test_cite_audit.py` → `test_spec_labels.py` | move `TestSharedSpecLabelMap`'s 8 A-i tests (the class has **9** methods; `test_coverage_map_fallback_round_trips` is B's). Rewrite the **three** prose sites naming `cite_audit` — the class docstring, `test_module_leaves_no_temporaries_to_delete`'s docstring, and the derivation test's — plus `test_aliases_do_not_collide_across_specs`'s name, docstring and worked example, which describe deleted machinery |

**Copy-count claim — five sites, measured**: `spec_labels.py:3`, `spec_labels.py:32` (the `SPECS` header
comment), `cli.py:30`, `DESIGN.md:55`, and the moved test's class docstring (already "three", but it names
`cite_audit` as a member, which K3 forbids). ⚠ Drafts 1-2 both named `SHORTNAME_TO_BLURB`'s comment, which
carries **no count**, and both omitted `DESIGN.md:55`.

⚠ **`DESIGN.md` "minus its catalog sentence" is not separable** — the clause shares a semicolon-joined
sentence with one that is false under a pinned-only map. A-i's verbatim bullet:

> `spec_labels.py` is the single source for spec shortname ↔ display label. It replaced the hand-maintained
> copies in `commands/coverage_map.py` and `cli.py`'s help blurb, which had drifted apart.

B adds the fall-through sentence; A-ii adds the gate's copy to the list.

---

## §5 Behaviour delta

The only observable change is which **spellings** resolve: 9 shortname spellings begin to resolve, over the
same 12 specs, 0 changed and 0 lost. Everything else — canonical labels, the three real aliases (`HTML`,
`DOM`, `URL`), non-pinned shortnames through `coverage_map`'s last-resort — is unchanged.
→ `rederive keysets`

A-i changes **no** gate behaviour, because it touches no gate file.

---

## §6 Pins

| Pin | What it executes | Fails at `origin/main`? |
|---|---|---|
| **S1** | `shortname_for(label) == short` over `SPECS`, for canonical labels **and** shortnames | **yes** (the shortname case) |
| **S2** | `label_for(shortname) == label` over `SPECS` | no |
| **S3** | `coverage_map._spec_label` derives from `SPECS` — identity assertion, no `sys.path` mutation | **yes** |
| **S3b** | `cli.COMMON_SHORTNAMES`'s derived block reproduces `origin/main`'s literal byte-identically, **vendored as a literal** so the pin survives the slice that deletes the original | **yes** |
| **S4** | `LABEL_TO_SHORTNAME` is byte-identical with the 8 aliases deleted | **yes** |
| **S5** | `shortname_for` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, **vendored as a literal** — correct precisely because the point is to freeze the *old* table (K4) | no |
| **S6** | `_spec_label` over the 12 pinned shortnames **and** a non-pinned sample exercising the last-resort | no |
| **S7** | K3 by grep: `cite.?audit` and `_catalog` absent from `.claude/tools/_webref/` + `.claude/skills/`; `webref_data` absent from `spec_labels.py` | no — `origin/main` satisfies it, which is the point |
| **S8** | K2 as an **absolute**: no elidex file path in `.claude/tools/_webref/` at all | **yes** — `origin/main` has one |
| **T-net** | across A-i's suite, `subprocess.run` is never called with the resolved `WEBREF` path and `urlopen` is never called | no |

**UNCHECKED, marked not omitted**: that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`; unreachable through the gate, and the gate is
A-ii's).

---

## §7 Layering

**VM host / ECS-native**: not applicable, no `crates/**` diff. **Generic core vs adapter**: A-i is
**entirely generic** — `spec_labels.py`, `coverage_map.py`, `cli.py`, `DESIGN.md`, `test_spec_labels.py`. It
touches no adapter file, which is what makes K2 statable as an absolute and §12(3) a plain grep rather than a
set difference. → `rederive couplings`

**One-issue-one-way**: the label enumeration goes three sites → one, two of the three in this slice.

---

## §8 Line-count budget

→ `rederive budget`. `spec_labels.py` is a new file well under any threshold; the two consumers lose lines.
⚠ **The 901-line `…-A-rederive.sh` is the one artifact on this branch past the 700-800 authoring band**, it
serves four slices, and the umbrella now owes its split to whichever slice next touches it. A-i cites six of
its blocks and does not touch it.

---

## §9 Edge-dense assessment

**A-i does not trip CLAUDE.md's trigger.** One invariant (labels resolve identically), a canonical algorithm
(dict lookup), zero `crates/**`, no control flow. ⚠ Drafts 1-2 argued the umbrella base case instead, and
that argument was falsified twice on its own conjuncts. The honest statement is simpler: the *merged* Slice A
tripped the trigger; A-i inherited its apparatus without earning it, and the umbrella's *review cost tracks
blast radius* constraint now says not to.

Conjunct (i) still holds — the umbrella names A-i and states its scope. ⚠ Its scope cell was last amended
**during** this slice's review (A-i round 2's finding), which is the self-ratification the re-slice was
careful to avoid; the current amendment was made on user approval, outside a slice commit.

---

## §11 Defer slots

**Zero own deferrals.** A-i creates no failable capability (it touches no gate), no network dependency, no
scheduling gap. ⚠ Drafts 1-2 claimed this while migrating `preflight.py`, which was false; it is true of the
narrowed scope by construction.

---

## §12 Exit criterion

1. **Green**: `test_spec_labels.py` passes; `git diff -- crates/` and `git diff -- .claude/skills/` both
   **empty** (A-i touches no adapter file).
2. **K3**: `git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/` → empty;
   `git grep -n '_catalog' -- .claude/tools/_webref/ .claude/skills/` → empty;
   `git grep -n 'webref_data' -- .claude/tools/_webref/spec_labels.py` → empty.
   (`origin/main` measures 0 and 0; `webref_data` legitimately has 8 hits elsewhere.)
3. **K2**: `bash …-A-rederive.sh couplings` → **no elidex file path in `.claude/tools/_webref/` at all**.
4. **K1/K4**: S3, S3b and S5 green.

Checks 2 and 3 are greps over prose occurrences, not over file assignments.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | takes `preflight.py`'s copy **and** its failure semantics, plus `SPEC_LABEL_REVERSE`'s census — the two gate-output strings, the four plan-memo readers, and `#11-preflight-css-module-labels` | **A-i first** |
| **A-iii** | none | after A-ii |
| **Slice B** | takes every row marked B in §4.2, plus the fall-through | after A-ii |
| **Slice C** | shares `DESIGN.md`; A-i states its bullet verbatim, C owns the reported-class contract | after B |
| **PR-A0 (`elidex-wt-submittable`)** | touches the same `_webref` files | after A/B/C; it rebases |
| **PR #496 / #497** | none — A-i touches no `ci.yml`, `mise.toml` or `crates/**` | none |

→ `rederive lanes` (ranges over the files A-i contends on, not only `docs/plans/`).

**Landing checklist**

1. Update `project_citation-hygiene-program.md` and `active-lane-detail.md` with A-i's outcome. ⚠ This
   program's state is currently stated in **three** places and they disagree on round 1's MIN count; collapse
   to the program memo with pointers.
2. Register nothing — A-i has no slots. `#11-preflight-css-module-labels` and
   `#11-webref-preflight-inprocess-resolution` are **A-ii's**, and measured, neither is in any ledger today.

---

## §14 Provenance

Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (nine rounds; recoverable at
`git show 707b69cc^:docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md` — the SHA an earlier
umbrella revision named was destroyed by a rebase). A-i itself: draft 1 → round 1 (1 CRIT / 28 IMP) →
draft 2 → round 2 (1 CRIT / 38 IMP) → draft 3, which narrows the scope and stops restating the diff.

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`. Blocks A-i cites: **`citations keysets readers regions
couplings budget`**. ⚠ `lanes` and `staleclaims` are author-local and excluded from `all`.
