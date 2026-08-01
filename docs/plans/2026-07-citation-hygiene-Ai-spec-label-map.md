# Plan — Slice A-i: one spec-label map, pinned, with its prose moved by role

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i** (the 2026-08-01 re-slice of
Slice A). Under that umbrella's approval boundary this is a **terminal unit** (§9).
**Branch**: `webref-cite-audit-tool`. **Nature**: pure refactor of developer tooling. **Zero `crates/**`
diff, zero gate-semantics change, zero CI topology.**
**Status**: plan-memo, **draft 1**. `/elidex-plan-review` **required before implementation**.

**This memo carries no measured digits of its own.** Every quantity is printed by a function in
`docs/plans/2026-07-citation-hygiene-A-rederive.sh`, which ships on this branch; the memo cites the function
name (`→ rederive keysets`) and a reviewer runs it. `lanes` and `staleclaims` are author-local and excluded
from `all`.

### §0.1 What A-i is, and what it deliberately is not

`origin/main` carries the same enumeration three times — `preflight.SPEC_LABEL_REVERSE`,
`coverage_map._SPEC_LABEL_MAP` and `cli.COMMON_SHORTNAMES`. Adding a spec to one did not reach the others,
so the mapping drifted by construction — the same partial hand-maintained enumeration whose failure mode
this program exists to detect. A-i collapses them onto one `.claude/tools/_webref/spec_labels.py`, **pinned
map only**.

Three boundaries, each of which was a slice-boundary error in an earlier draft of the merged memo:

1. **A-i ships no catalog fall-through.** `shortname_for` and `label_for` consult `SPECS` and stop. The
   948-entry catalog, `_catalog()`, the reverse index and the round-trip rules are **Slice B's** — B owns
   the lookup semantics that make the widening correct, and B owns its offline contract.
2. **A-i changes no failure semantics.** Landing a shared import is what first makes label resolution
   *failable*; hardening that failure is **Slice A-ii**, stacked directly on this. A-i leaves
   `preflight.py`'s `except Exception: _shortname_for = None` guard exactly as the merged memo's carve left
   it — a documented soft-warn — and A-ii replaces it. The two are one commit apart on purpose.
3. **A-i schedules nothing.** No `mise` task, no CI job. That is **A-iii**.

**A-i's spec *set* is unchanged (the same 12 pinned specs), but 9 additional *spellings* resolve** — the
shortnames themselves (`fetch`, `xhr`, `webidl`, `streams`, `webcrypto`, `ecma262`, `ecma402`,
`selectors-4`, `geometry-1`). That comes from the **shortname-as-own-parse-key** rule, not from a widened
alias list. → `rederive keysets`

---

## §0.5 Spec citation table

A-i implements no spec logic. The two citations below are the ones its own §3 carries, and both are looked
up with `.claude/tools/webref` — nothing from memory. → `rederive citations`

| Cite | § | Exact title | Anchor | Why this one |
|---|---|---|---|---|
| `WHATWG HTML §4.10.21` | HTML §4.10.21 | Constraints | `#constraints` | `html` is pinned by `SPECS`; the canonical-label spelling |
| `WHATWG Fetch §2.2.5` | Fetch §2.2.5 | Requests | `#requests` | `fetch` is pinned by `SPECS`; exercises the shortname-as-parse-key rule §0.1 names |

⚠ **Both labels are ones A-i's own pinned map resolves, and that is a deliberate constraint on this
table.** The merged memo's §3 carried a `CSSOM View §4.2` row, which resolves **only via the branch's
catalog** — the machinery this program routes to B. So that memo's own coverage map was certified by
machinery its own slice removes, and would have soft-warned against itself after landing. Round 9 found it.
A-i's rule: **a slice's §3 may only cite labels that slice's own resolver maps.**

---

## §1 Ideal anchor — one enumeration, one home, and the prose goes with it

`DESIGN.md`'s closing rule for the `_webref` core is: *"keep new generic behavior free of elidex-specific
file paths and put elidex policy in adapter commands or documentation."* A dedup that moves the **table**
and leaves the **prose** describing it scattered has not collapsed the decision surface — it has moved it.

That is not hypothetical. Measured at the merged memo's head, A's half of the tree carried **one elidex file
path the merged memo's own exit criterion forbade** (`spec_labels.py:7`), because the edit set assigned the
docstring to A and never instructed the rewrite. And it carried the string `cite-audit` at multiple sites in
A's half — including inside `cli.py`'s `--help` epilog — while `origin/main` carries **zero** and the
detector is Slice B's. → `rederive couplings`

**The corollary that drives §4**: the unit of this edit is not the file and not the code branch. It is the
**named artifact**. Every occurrence of a B-owned name, and every occurrence of an elidex path, is either
rewritten or explicitly assigned.

---

## §2 Coupled invariants

- **K1 — one enumeration.** After A-i exactly one site enumerates spec label ↔ shortname. Checkable by
  construction: the other two import it.
- **K2 — the generic core names no elidex file path.** `DESIGN.md`'s rule. ⚠ `origin/main` **already
  carries one** (`cli.py`'s `.claude/skills/elidex-review/axes.md.`), so K2 is a **delta**: A-i adds none.
  Discharging the pre-existing one is not A-i's scope.
- **K3 — the generic core names no Slice-B artifact.** ⚠ Three tokens, three scopes — see §12(2), which is
  where draft 1 of this memo wrote them as one pattern and thereby claimed a violation `origin/main` already
  has. `cite-audit` and `_catalog` appear **nowhere** in the tooling tree (matching `origin/main`);
  `webref_data` appears nowhere **in `spec_labels.py`** (it legitimately backs eight other command modules).
- **K4 — no behaviour change.** Every label `origin/main` resolved, A-i resolves identically; the added
  spellings are a strict superset over the same 12 specs. `SPEC_LABEL_REVERSE`'s 15 pairs are vendored as a
  literal and frozen.

K1 and K4 are independent of K2/K3 — one is about the table, the other about the text around it. They are
listed together because a single edit set must satisfy all four, and the merged memo's edit set satisfied
K1 and K4 while silently failing K2 and K3.

---

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG HTML §4.10.21 Constraints | n/a — no spec logic | the canonical-label spelling of a pinned spec | §4.3 — `test_spec_labels.py` | ✓ — the table is authored, not discovered | no |
| WHATWG Fetch §2.2.5 Requests | n/a — no spec logic | the shortname-as-parse-key spelling of a pinned spec | §4.3 — same suite | ✓ | no |

**Breadth**: measured by the gate on this memo. Rows here are assertions in a moved unit test; a table
larger than the property under test is padding.

### §3.1 User-input touch audit + discovery method

**No web-content input flow.** Nothing here is reachable from page content, script, or a network peer. The
inputs are a plan-memo's `§3` cell text and a comment line — both developer-authored.

**Discovery method.** Every number comes from the harness, measured against `origin/main` and never the
branch; branch/lane facts use three-dot ranges; a proposed patch is *measured*, not read; and **a claim
about *where* something occurs is grepped by concept, not by string**. A-i adds one rule the merged memo
lacked: **a claim about prose is checked by a grep over prose occurrences**, not over file assignments —
which is what §12's checks now do.

---

## §4 The edit set

### §4.1 Slice routing

**The criterion, stated once**: A-i owns the *table and the text that describes the table*. Anything that
changes what a lookup **returns** is B's; anything that changes what the gate **does with** a return value
is A-ii's; anything that decides **when** something runs is A-iii's.

| Concern | Slice | Why |
|---|---|---|
| `spec_labels.py`'s `SPECS`, the three derived dicts, the pinned lookup bodies | **A-i** | the table |
| the module docstring, the derived-dict comments, both function docstrings | **split** — see §4.2 | the text that describes the table |
| `coverage_map`'s delegation to `label_for`; `cli.py`'s blurb derivation | **A-i** | consumers of the table |
| `test_spec_labels.py` (the 8 moved tests, minus the `preflight` assertion) | **A-i** | §4.3 |
| the catalog fall-through, `_catalog()`, the reverse index, round-trip rules, `coverage_map`'s changed last-resort | **B** | lookup semantics |
| `cite_audit.py`, `test_cite_audit.py`, the `cite-audit` subparser and its import, `webref_data.py` | **B** | the detector |
| `preflight.py`'s failure semantics, the remedy strings, the no-spec-surface declaration, `SKILL.md` | **A-ii** | the gate's contract |
| `python-suites.sh`, `[tasks.tools-test]`, the `tools` job, the interpreter floor | **A-iii** | scheduling |
| `axes.md`, `CLAUDE.md` § "Spec citation", `DESIGN.md`'s reported-class contract | **C** | review policy |

⚠ **A-i still edits `preflight.py`** — one import, replacing the module-local dict. That is unavoidable:
the dedup is not done until the third copy is gone. What A-i does **not** touch there is any branch, any
diagnostic string, or `SECTION_REF_RE`.

### §4.2 `spec_labels.py`, by named artifact

Named by content, not line number. → `rederive regions`

| Region | Slice | A-i's instruction |
|---|---|---|
| module docstring — the drift rationale | **A-i** | "Four sites" → **three**; delete the `cite_audit.py` bullet; **rewrite the consumer list by role** — `.claude/skills/elidex-plan-review/preflight.py` becomes *"the plan-review gate"* (K2) |
| module docstring — the sentence naming `cite-audit` as the failure-mode detector | **A-i** | rewrite without the artifact name (K3): *"…the same partial hand-maintained enumeration this program exists to detect"* |
| module docstring — the *"`SPECS` is a fallback, not the source"* paragraph | **B** | delete in A-i; B reinstates it with the fall-through |
| `SPECS` + the three derived dicts | **A-i** | keep; **delete the 8 parse aliases** (below) |
| the `LABEL_TO_SHORTNAME` comment's load-time consumer list (a **second** list, naming `cite-audit`) | **A-i** | same two rewrites as the docstring's — K2 and K3 |
| the alias rationale comment | **A-i** | **delete** — it describes machinery A-i removes |
| `_catalog()` and its `sources.webref_data` import | **B** | absent from A-i |
| `label_for` / `shortname_for` — pinned lookup bodies | **A-i** | keep |
| `label_for` / `shortname_for` — docstrings' catalog paragraphs, the `CSS Text 3` example, the "failed **open**" sentence | **B** | delete in A-i; they describe B's fall-through |

**A-i deletes the 8 parse aliases.** Measured: removing every one leaves `LABEL_TO_SHORTNAME`
**byte-identical**, because each alias (`HTML`, `DOM`, `URL`, `Fetch`, `Streams`, `WebCrypto`, `XHR`,
`WebIDL`) lowercases to its own shortname, which the comprehension already supplies via `entry[0]`. They are
dead weight in a file A-i creates. → `rederive keysets`

### §4.2.1 `cli.py` and `coverage_map.py`, by named artifact

| Site | Slice | A-i's instruction |
|---|---|---|
| `cli.py`'s blurb-derivation comment, which names `cite-audit` | **A-i** | rewrite without the artifact name (K3) |
| `cli.py`'s `COMMON_SHORTNAMES` **`--help` epilog**, whose Examples block advertises `webref cite-audit …` | **A-i** | **delete that Example line.** ⚠ This is not a comment: it is shipped user-facing help for a subcommand that does not exist at A-i's head. Round 9 found it |
| `cli.py`'s `.claude/skills/elidex-review/axes.md.` path | **pre-existing** | untouched; K2 is a delta |
| `coverage_map._spec_label`'s last-resort `shortname.upper().replace("-", " ")` | **A-i** | keep `origin/main`'s **verbatim**; the branch's `or shortname` is correct only together with B's catalog |
| `DESIGN.md`'s `spec_labels.py` bullet | **A-i** | stated verbatim below |
| `DESIGN.md`'s `cite_audit.py` adapter bullet, the three `cite-audit` usage lines, the buckets paragraph | **B** | all branch-new; absent from A-i |

⚠ **`DESIGN.md` "minus its catalog sentence" is not a separable edit**, which is why A-i states its text
rather than describing it. The merged branch's bullet joins two clauses with a semicolon, and the *first* —
"`SPECS` pins only what upstream cannot supply" — is **false** under a pinned-only map, where `SPECS` pins
all twelve. A-i's verbatim bullet:

> `spec_labels.py` is the single source for spec shortname ↔ display label. It replaced three
> hand-maintained copies (`commands/coverage_map.py`, `cli.py`'s help blurb, and the plan-review gate's
> preflight) that had drifted apart.

B adds the fall-through sentence and the fourth copy when it lands the catalog.

### §4.3 Site the label-map tests where they belong

The merged branch's `TestSharedSpecLabelMap` carries 10 tests. **8 are A-i's** and move to
`.claude/tools/_webref/test_spec_labels.py`. `test_coverage_map_fallback_round_trips` and the
`coverage_map_label` helper are **B's** — they assert the catalog round-trip.

One assertion inside `test_all_three_consumers_derive_from_specs` does not belong in the generic tree: it
inserts the *elidex skill's* directory onto `sys.path` and imports `preflight` — the one **import-time
executable** edge blocking `DESIGN.md`'s goal of keeping the core movable to a standalone repository.

- The `coverage_map` half stays in `test_spec_labels.py`, at module-level import.
- The `preflight` half moves to **A-ii**'s `test_preflight.py`. A-i does not create that file.
- **No `sys.path` mutation survives inside any test method.**
- ⚠ The test's **name and docstring** move with the body and must be rewritten: it will assert one
  consumer, not three, and its prose names `cite_audit` twice (K3).

### §4.4 The copy-count claim, at every site that makes it

⚠ The "four hand-maintained copies" figure is asserted at **four** sites, not one, and A-i's dedup makes the
old figure wrong at all of them. Enumerated rather than described: `spec_labels.py`'s docstring, the
`SHORTNAME_TO_BLURB` comment, `cli.py`'s blurb comment, and the moved test's docstring. Each becomes
**three**, and none may name `cite_audit.py` as one of them (K3) — the detector is B's, so at A-i's head it
is not a copy that exists. → `rederive couplings`

---

## §5 Behaviour deltas

**Baseline is `origin/main`.** A-i is a refactor: the only observable change is which *spellings* resolve.

| # | Input | `origin/main` | After A-i |
|---|---|---|---|
| 1 | a canonical label (`WHATWG HTML`) | resolves | **unchanged** |
| 2 | a lower/mixed-case canonical label | resolves | **unchanged** |
| 3 | an alias spelling (`HTML`, `Fetch`, …) | resolves | **unchanged** — the alias key and the shortname key are the same string lower-cased |
| 4 | a **shortname** spelling (`fetch`, `selectors-4`, …) | **does not resolve** — soft-warn, verify skipped | **resolves** |
| 5 | a label of a spec outside the pinned 12 | does not resolve | **unchanged** — no catalog in A-i |
| 6 | `coverage_map._spec_label` on a non-pinned shortname | `.upper().replace("-", " ")` | **unchanged** — verbatim |

**The only newly-resolving class is row 4**, and it is 9 spellings over the same 12 specs — 0 changed, 0
lost, no new spec. → `rederive keysets`

---

## §6 Pins

| Pin | What it executes | §5 rows | Fails at `origin/main`? |
|---|---|---|---|
| **S1** | `shortname_for(label) == short` over `SPECS`, for canonical labels and shortnames | 1, 2, 4 | **yes** (row 4) |
| **S2** | `label_for(shortname) == label` over `SPECS` | — | no |
| **S3** | all three consumers derive from `SPECS` — identity assertion, `coverage_map` half only, **no `sys.path` mutation in the body** | — | **yes** |
| **S4** | `LABEL_TO_SHORTNAME` is byte-identical with the 8 parse aliases deleted | — | **yes** |
| **S5** | `shortname_for` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, **vendored as a literal** — correct here precisely because the point is to freeze the *old* table (K4) | 1–3 | no |
| **S6** | `coverage_map._spec_label` over the 12 pinned shortnames **and** a non-pinned sample exercising the `.upper().replace("-", " ")` last-resort | 6 | no |
| **S7** | **K3 by grep**: `cite.?audit` / `webref_data` / `_catalog` appear **nowhere** in A-i's half | — | no — `origin/main` already satisfies it, which is the point |
| **S8** | **K2 by grep, as a delta**: the set of elidex file paths in A-i's half equals the `origin/main` set | — | no — same reason |
| **T-net** | across A-i's whole suite, `subprocess.run` is never called with the resolved `WEBREF` path, and `urlopen` is never called | — | no |

S7 and S8 are the two pins the merged memo lacked, and they are the ones that would have caught round 9's
Axis 1 findings at authoring time. Both are cheap greps over a fixed file set — see §12.

⚠ **UNCHECKED, marked not omitted**: that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`, the other does not — unreachable through the gate
because `parse_spec_cell` already strips, so the *gate* claim S5 makes holds and the *function* claim is not
made).

---

## §7 Layering check

**CLAUDE.md "VM host/ is engine-bound only"** and **ECS-native** — not applicable; no `crates/**` diff.

**`DESIGN.md` generic-core / elidex-adapter split.** The honest picture, measured rather than asserted:
`origin/main`'s generic tree **already carries** plan-memo / plan-review references across two files, and
**one actual elidex file path** (`cli.py`). So A-i is promising not to *add* to a non-empty class, not
claiming the class is empty — which is why K2 is a delta and S8 checks it as one. → `rederive couplings`

| Edit | Layer |
|---|---|
| `SPECS` (labels + shortnames + help blurbs; aliases deleted) | **generic mechanism + pre-existing elidex display convention** |
| `coverage_map.py`, `cli.py` blurb derivation, `DESIGN.md` bullet, `test_spec_labels.py` | **generic** |
| `preflight.py`'s one import | **elidex skill** |

**The display labels** (`"WHATWG HTML"` etc.) are elidex convention and are **already** in the generic core
on `origin/main`, in `coverage_map._SPEC_LABEL_MAP`. A-i moves the *reverse direction* of the same table,
not new policy. **The shortname-as-own-parse-key rule** is a property of the catalog namespace rather than
elidex policy — a spec's shortname is a valid way to name it in any repository — so it is generic and stays.

**One-issue-one-way**: the label enumeration three sites → one. That is the whole of A-i.

---

## §8 Line-count budget

→ `rederive budget`. `spec_labels.py` is a new file well under any threshold; the three consumers each lose
lines. Nothing in the touch set is near 1000.

---

## §9 Edge-dense assessment

CLAUDE.md's **base case** has two conjuncts.

**(i) An approved umbrella's per-PR slice, explicitly.** The umbrella names A-i, states its scope, and was
amended to do so **before** this memo's plan-review — not by this slice's own commit. That sequencing is the
correction round 9 forced.

**(ii) Scope narrowed to a single invariant-axis intersection.** K1 and K4 are one axis (the table); K2 and
K3 are one axis (the prose around it); they intersect at exactly one point — the module docstring, which
both the dedup and the by-role rule edit. No control flow changes, no exit code changes, no configuration
changes. **This is the conjunct the merged Slice A could not satisfy**, and it is why A-i exists separately:
nine review rounds established that the table-and-prose work and the gate-semantics work do not share a
review surface.

`git diff --stat -- crates/` is empty and stays empty.

---

## §10 Open questions

Decided rather than listed ([[feedback_no-low-value-choices]]): the aliases are **deleted**; `coverage_map`'s
last-resort stays **verbatim**; the `DESIGN.md` bullet's A-i text is **stated verbatim** rather than
described; and the pre-existing `cli.py` elidex path is **left alone** (K2 is a delta).

- **Q1 — should A-i also discharge `cli.py`'s pre-existing elidex path?** It is one line, and A-i edits that
  file anyway. **Recommendation: no.** It is `axes.md`'s path, i.e. review policy, which the umbrella routes
  to **C**; discharging it here would decide C's boundary by side effect. Registered as a note for C, not a
  slot.

---

## §11 Defer slots + per-PR ≤3 audit

**Zero own deferrals.** A-i creates no failable capability (that is A-ii), no network dependency (that is
B), and no scheduling gap (that is A-iii).

**Explicitly NOT deferred**: the alias deletion, all four copy-count sites, both `cli.py` prose sites
including the `--help` epilog, both function docstrings, the second consumer list, the moved test's name and
docstring, and `DESIGN.md`'s verbatim text.

---

## §12 Exit criterion

**(1) Green:** the moved suite passes, and `git diff -- crates/` is empty.

**(2) K3 — no Slice-B artifact in A-i's half.** ⚠ **Three tokens, three different scopes, and draft 1 of
this memo got it wrong by writing them as one pattern.** Measured on `origin/main`: `cite.?audit` **0**,
`_catalog` **0**, but `webref_data` **8** — in eight unrelated command modules (`css.py`, `dfn.py`,
`element.py`, `heading.py`, `idl.py`, `specs.py`, `inventory.py`, `resolver.py`), where it is a legitimate
shared data source with nothing to do with the label surface. A single widened pattern therefore reports a
violation `origin/main` already has, which is the *same* error class §12's own ⚠ is about:

```sh
git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/          # → empty (origin/main: 0)
git grep -n '_catalog'     -- .claude/tools/_webref/ .claude/skills/          # → empty (origin/main: 0)
git grep -n 'webref_data'  -- .claude/tools/_webref/spec_labels.py            # → empty (A-i's file only)
```

The first two are restorations of an `origin/main` property; the third is the one genuinely new invariant —
A-i's file must not reach the catalog source.

**(3) K2 — no *added* elidex file path:**

```sh
bash docs/plans/2026-07-citation-hygiene-A-rederive.sh couplings   # → "ADDED BY A" empty
```

**(4) K1/K4 — one enumeration, same answers:** S3 and S5 green.

⚠ Checks (2) and (3) are **greps over prose occurrences**, not over file assignments. That is the whole
lesson of round 9's Axis 1 findings: an edit set that assigns files cannot be verified by a check that reads
file assignments.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | total by construction — branches from A-i's landed head and replaces the import guard | **A-i first** |
| **A-iii** | none — A-i ships no scheduling | after A-ii |
| **Slice B** | takes every row marked **B** in §4.2 / §4.2.1, plus the fall-through | after A-ii |
| **`elidex-wt-submittable` (PR-A0)** | touches the same `_webref` files | after A/B/C; it rebases |
| **PR #496 / PR #497 (Layout lane)** | none for A-i — no `ci.yml`, no `mise.toml` | none |

**Owed to B's memo**, stated as classes to grep, not a list read off → `rederive bmemo`. **Still
OWED-not-applied.**

**Landing checklist**

1. Update `project_citation-hygiene-program.md` (the program's cross-session SoT) with the A-i/A-ii/A-iii
   split and this slice's outcome.
2. Register nothing — A-i has no slots.

---

## §14 Provenance

A-i is carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (drafts 1–9, nine
`/elidex-plan-review` rounds). The carve is the umbrella's 2026-08-01 re-slice, forced by round 9 returning
**worse** than round 8 (3 CRIT / 33 IMP vs 0 / 30). Corrections that originated in those rounds and are
carried here rather than restated there: the alias-inertness measurement (R6), the region table's need for
prose rows and the `DESIGN.md` non-separability (R9 Axis 1), the `--help` epilog site (R9 Axis 1), and the
§3-may-only-cite-what-it-maps rule (R9 Axis 4).

The merged memo's remaining content — the capability verdict, both act-sites, the four remedies, the
no-spec-surface declaration, §5's capability matrix and §6's pin set — moves to **A-ii**, with round 9's two
CRITs unresolved and carried as its opening defects.

---

## §15 Re-derivation

`docs/plans/2026-07-citation-hygiene-A-rederive.sh`. Blocks A-i cites: `citations keysets regions couplings
budget bmemo`. `lanes` and `staleclaims` are author-local and excluded from `all`.
