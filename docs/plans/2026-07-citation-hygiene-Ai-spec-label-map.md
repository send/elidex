# Plan — Slice A-i: one spec-label map in the generic tree

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i**. **Branch**:
`webref-cite-audit-tool`. **Status**: plan-memo, **draft 4**. `/elidex-plan-review` before implementation.

⚠ **The memo is a record, not a specification.** Drafts 1-2 were 404 / 462 lines and drew 1 CRIT / 28 IMP and
1 CRIT / 38 IMP; draft 3 was 297 and drew 3 CRIT / 8 IMP / 7 MIN. Across all three, nearly every finding was a
defect in the *description*, not the change, and round 3's three CRITs shared one root — stated with its
measurement at the head of §4, which is the base every edit-set row is now relative to. Per the umbrella's
*review cost tracks blast radius*, **the canonical statement of what the code does is the diff and the
tests**; quantities come from `docs/plans/2026-07-citation-hygiene-A-rederive.sh`, cited by block name.

### §0.1 What A-i is

`origin/main` carries one spec-label enumeration **three** times — `coverage_map._SPEC_LABEL_MAP`,
`cli.COMMON_SHORTNAMES`, `preflight.SPEC_LABEL_REVERSE`. A-i creates `.claude/tools/_webref/spec_labels.py`,
**pinned map only**, and collapses **the two in the generic tree** onto it.

⚠ **`preflight.py`'s map is not touched** — it migrates in **A-ii**, since drafts 1-2 both tried to land it
here and both regressed the gate in opposite directions. The four-cell measurement and its conclusion (**the
gate's copy is not separable from the gate's failure semantics**) are stated once, in the umbrella's A-i row,
and not restated here. So K1 completes across A-i + A-ii. A-i is inside the generic tree with **one stated
exception** — a single `preflight.py` comment naming a symbol A-i deletes (§4.1) — and touches no gate
semantics, no CI topology. The resolution delta this produces is stated once, in §5.

---

## §0.5 Spec citation table

A-i implements no spec logic. Both labels are pinned by `SPECS`, per the umbrella's *a slice may only cite
labels its own resolver maps*. Looked up with `.claude/tools/webref`, nothing from memory.
→ `rederive citations`

| Cite | § | Exact title | Anchor |
|---|---|---|---|
| `WHATWG HTML §4.10.21` | HTML §4.10.21 | Constraints | `#constraints` |
| `WHATWG Fetch §2.2.5` | Fetch §2.2.5 | Requests | `#requests` |

Two rows, two **distinct pinned specs**, so K=2 and the table is not one spec twice. ⚠ Draft 1 justified the
second as "exercises the shortname-as-parse-key rule"; measured, `WHATWG Fetch` is `entry[1]`, the canonical
label, resolving identically at baseline — the spellings that exercise the rule are `Fetch` / `fetch`.

---

## §1 Ideal anchor

A dedup that moves the **table** and leaves the **prose** describing it scattered has not collapsed the
decision surface — it has moved it. `DESIGN.md`'s closing rule for the generic core is operative: *keep new
generic behavior free of elidex-specific file paths, and put elidex policy in adapter commands or
documentation.* **The unit of this edit is the named artifact**, not the file and not the code branch: every
occurrence of a Slice-B artifact name, every elidex file path and every copy-count claim is either rewritten
or explicitly assigned, and the enumeration of those occurrences is **derived**, not authored (§4.1).

---

## §2 Coupled invariants

- **K1 — one enumeration in the generic tree.** After A-i, `coverage_map` and `cli` import rather than
  enumerate. `preflight`'s copy is A-ii's; K1 completes there.
- **K2 — the generic core names no elidex file path**, where *file path* means `.claude/(skills|tools)/` plus
  **two further segments**. ⚠ Drafts 1-3 wrote "no elidex file path **at all**", false at face value: the
  tool's own invocation path `.claude/tools/webref` is one segment and occurs **22** times in `origin/main`'s
  `cli.py`. Excluding it is intended — an install path is not a path into elidex's tree — and `rederive
  couplings` now carries the predicate in the block instead of leaving it implicit in a regex. An
  **absolute**, not a delta: A-i discharges `origin/main`'s one instance (`cli.py:78`,
  `.claude/skills/elidex-review/axes.md.`) by the same by-role rewrite it applies to its own — A-i already
  edits that file, and Slice C, the earlier routing target, has no `cli.py` mandate.
- **K3 — the generic core names no Slice-B artifact.** `cite-audit` and `_catalog` are absent from
  `.claude/tools/_webref/` and `.claude/skills/` (matching `origin/main`, measured 0 each); `webref_data` is
  absent from `spec_labels.py`. ⚠ Drafts 1-3 said it "legitimately backs eight other command modules";
  measured (`git grep -lI 'webref_data' origin/main -- .claude/tools/_webref/`) it is **8 files, 6 of them
  command modules** (`css` `dfn` `element` `heading` `idl` `specs`) — the rest are `inventory.py` and
  `resolver.py`, neither a command module.
- **K4 — labels resolve identically.** Strict superset over the same 12 specs; `origin/main`'s 15
  `SPEC_LABEL_REVERSE` pairs vendored as a literal and frozen (§13).

**Pairwise intersections** — they cannot be applied one at a time:

| pair | intersection |
|---|---|
| K1 × K3 | every statement asserting the copy **count** also names its **members**, so both invariants land in one sentence each. **Five statements, all authored by A-i** — §4.2 |
| K1 × K2 | the docstring's consumer list is both an enumeration and a place elidex paths appear; a **second** such list lives on `LABEL_TO_SHORTNAME` |
| K2 × K3 | the same prose usually carries both, so it is one rewrite — `cli.py`'s derivation comment names an elidex consumer *and* a B artifact |
| K1 × K4 | omitting the aliases must leave the map byte-identical, which is what makes it safe in a refactor slice (`rederive keysets`) |
| K3 × K4 | `coverage_map`'s last-resort is `origin/main`'s verbatim — a K4 requirement, and the reason A-i authors no catalog prose on it |

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

**A-i's implementation is authored from `origin/main`. Every row below is relative to that tree.**

⚠ Drafts 1-3 stated no base and were written against two trees at once — round 3's three CRITs. Measured,
`git ls-tree origin/main -- .claude/tools/_webref/spec_labels.py` prints nothing and
`git show origin/main:.claude/tools/_webref/cli.py | grep -c cite-audit` is **0**, so §0.1's *creates* was
`origin/main`-relative while §4.2 rows phrased as *deletions* were branch-relative with nothing to delete,
and the copy-count claim counted branch-only sites though §3.1 forbids exactly that.

The consequence is a lineage decision. Measured, exactly **one** commit carries the whole `.claude/`
implementation — `git log --oneline origin/main..HEAD -- .claude/` → `b3a7d469 tools(webref): carve the
cite-audit detector out of the citation sweep`. **It is dropped from A-i's lineage**; Slice B and A-ii each
re-introduce their own half from their own memos. Nothing is lost, and the pointer is recorded as *content
plus a second location* rather than a bare SHA (§14's lesson — a SHA in a rebasing branch has a half-life):
**branch `domform-submittable-category` (worktree `elidex-wt-submittable`) carries a byte-identical `_webref`
tree**, and `git diff --stat HEAD domform-submittable-category -- .claude/tools/_webref/` prints nothing.

### §4.1 The rule this slice is built on

⚠ Four rounds running, the root finding has been *a read whose write-path the draft changes, without
reconciling the other readers of that state.* It is now a command: **before writing an edit-set row for a
piece of state, run `rederive readers <symbol> [ref]` and assign every line it prints** — the census ranges
over a ref (default `origin/main`) and separates code from prose, because every failed edit set assigned code
and left prose. Census for the two symbols A-i removes:

| symbol | code | prose |
|---|---|---|
| `_SPEC_LABEL_MAP` | `coverage_map.py` :13 :30 :31 | `preflight.py:48` — the "keep in sync" comment. **A-i's**, below |
| `COMMON_SHORTNAMES` | `cli.py` :27 :80 | **none** — measured; the blurb lines are the literal's own body, which A-i moves into `SPECS`, not a reader of the symbol |

⚠ Drafts 1-3 routed `preflight.py:48` to A-ii "because it is in the adapter" — a **file**-based assignment,
four sections after §1 fixed the unit as the named artifact, and the move K2 refuses for `cli.py`. Measured,
`grep -nE '_SPEC_LABEL_MAP|keep in sync' …-Aii-gate-failure-semantics.md` → **no hits**: it had fallen between
the slices. The rule, once: **behaviour** travels with the gate's failure semantics (A-ii); **prose naming a
symbol this slice deletes** travels with the deletion (A-i). This is the single exception to "A-i touches no
adapter file" (§0.1, §7, §12(1)) and it is **one comment** — no other `preflight.py` line moves here.
`SPEC_LABEL_REVERSE`'s census stays A-ii's: its two gate-output readers (`preflight.py` :409, :422) and its
**four** plan-memo readers (measured; drafts said five, a branch-era figure), one of which registers
`#11-preflight-css-module-labels` — A-ii dispositions it (§13). → `rederive readers`

### §4.2 What changes, by named artifact

Rows are re-derived against `origin/main`. Rows drafts 1-3 phrased as reverts of branch content are **gone** —
a `cli.py` `--help` Example line for `cite-audit` and a `coverage_map._spec_label` docstring, neither of which
exists on `origin/main` (measured: `grep -c cite-audit` → 0; `_spec_label` there is two statements, no
docstring). The A/B region boundaries the `spec_labels.py` rows rest on → `rederive regions`.

| artifact | change |
|---|---|
| `spec_labels.py` — `SPECS` + the three derived dicts | **new**; **8 parse aliases omitted** (measured inert: the map is byte-identical without them, since each alias lowercases to its own shortname) |
| `spec_labels.py` — module docstring | authored to say **two in the generic tree**; names no `cite_audit.py` (K3); names `preflight.py` **by role** (K2) |
| `spec_labels.py` — the `LABEL_TO_SHORTNAME` comment's load-time consumer list | same two constraints — it is a **second** consumer list |
| `spec_labels.py` — catalog paragraph, both function docstrings' catalog clauses | absent in A-i; **B** authors them with the fall-through |
| `cli.py` — blurb derivation | import `SHORTNAME_TO_BLURB`; the derived block must reproduce `origin/main`'s literal byte-identically (S3b) |
| `cli.py` — the new derivation comment | authored without the B artifact name (K3) |
| `cli.py:78` — `.claude/skills/elidex-review/axes.md.` | **by-role rewrite** (K2, absolute) — the one pre-existing instance |
| `coverage_map.py` — `_spec_label` | delegate to `label_for`; keep `origin/main`'s last-resort `.upper().replace("-", " ")` **verbatim** |
| `DESIGN.md` — the `spec_labels.py` bullet | new bullet, verbatim below |
| `test_spec_labels.py` | **new** — 8 tests over the pinned map (`test_coverage_map_fallback_round_trips` is B's; A-i does not author it). No prose in it names `cite_audit`, and no test asserts over parse aliases, since A-i ships none |

Each row is scoped to **every occurrence** in the named artifact, not to a bullet list inside it.

**Copy-count statements — five, all authored by A-i.** `origin/main` carries **no** copy-count claim anywhere
under `.claude/` (measured; the near hit `webref_data.py:57` "No hand-maintained alias map" carries no count),
so each is new prose and the constraint is on **wording**, not on a correction: `spec_labels.py`'s module
docstring, its `SPECS` header comment, `cli.py`'s derivation comment, `DESIGN.md`'s bullet, and
`test_spec_labels.py`'s class docstring — each saying **two in the generic tree** and naming only
`coverage_map` and `cli`'s blurb. ⚠ Drafts 1-2 named `SHORTNAME_TO_BLURB`'s comment, which carries no count,
and omitted `DESIGN.md`; draft 3 fixed membership but called the five *corrections*, counting branch-only
sites.

A-i's verbatim `DESIGN.md` bullet, stated here because Slice C shares the file:

> `spec_labels.py` is the single source for spec shortname ↔ display label. It replaced the hand-maintained
> copies in `commands/coverage_map.py` and `cli.py`'s help blurb, which had drifted apart.

B adds the fall-through sentence; A-ii adds the gate's copy to the list.

---

## §5 Behaviour delta

The only observable change is which **spellings** resolve. The spec set is unchanged — the same 12 pinned
specs — and **9 additional spellings resolve**, the shortnames themselves, from the shortname-as-own-parse-key
rule rather than a widened alias list: 0 changed, 0 lost. Everything else — canonical labels, the three real
aliases (`HTML`, `DOM`, `URL`), non-pinned shortnames through `coverage_map`'s last-resort — is unchanged.
→ `rederive keysets`

**A-i changes no gate behaviour — by reachability, not by file membership.** ⚠ Drafts 1-3 said "because it
touches no gate file": a non-sequitur whose premise is also false as an *execution* claim. Measured,
`verify_citation` (`origin/main:.claude/skills/elidex-plan-review/preflight.py:265`) subprocesses
`[sys.executable, WEBREF, "heading", "--exact", …]` for **every citation it verifies**, and
`.claude/tools/webref` is `from _webref.cli import main` — so `cli.py` runs and `commands/coverage_map.py` is
imported on every gate run. What holds instead: `coverage_map._spec_label` has exactly **one** caller on
`origin/main` (`cmd_coverage_map:72`), which `cmd_heading` never reaches; `cli.COMMON_SHORTNAMES` is read at
one site (`epilog=`, `cli.py:80`) and *is* built on every run, which **S3b** makes harmless by pinning the
derived block byte-identical, its only rendering being `--help` / argparse error output the gate never
triggers; and `spec_labels.py` is import-inert (a tuple, three comprehensions, no I/O), the biting half of
which **T-net** pins.

---

## §6 Pins

| Pin | What it executes | Fails at `origin/main`? |
|---|---|---|
| **S1** | `shortname_for(label) == short` over `SPECS`, for canonical labels **and** shortnames | **yes** (the shortname case) |
| **S2** | `label_for(shortname) == label` over `SPECS` | no |
| **S3** | `coverage_map._spec_label` derives from `SPECS` — identity assertion, no `sys.path` mutation | **yes** |
| **S3b** | `cli.COMMON_SHORTNAMES`'s derived block reproduces `origin/main`'s literal byte-identically, **vendored as a literal** so the pin survives the slice that deletes the original | **yes** |
| **S4** | `LABEL_TO_SHORTNAME` is byte-identical with the 8 aliases omitted | **yes** |
| **S5** | `shortname_for` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, **vendored as a literal** — correct precisely because the point is to freeze the *old* table (K4) | no |
| **S6** | `_spec_label` over the 12 pinned shortnames **and** a non-pinned sample exercising the last-resort | no |
| **S7** | K3 by grep: `cite.?audit` and `_catalog` absent from `.claude/tools/_webref/` + `.claude/skills/`; `webref_data` absent from `spec_labels.py` | no — `origin/main` satisfies it, which is the point |
| **S8** | K2 as an **absolute**, under §2's predicate: no `.claude/(skills\|tools)/` + two-further-segments path anywhere in `.claude/tools/_webref/` | **yes** — `origin/main` has one |
| **T-net** | across A-i's suite, `subprocess.run` is never called with the resolved `WEBREF` path and `urlopen` is never called | no |

**UNCHECKED, marked not omitted**: that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`; unreachable through the gate, and the gate is
A-ii's).

---

## §7 Layering

**VM host / ECS-native**: not applicable, no `crates/**` diff. **Generic core vs adapter**: A-i is generic —
`spec_labels.py`, `coverage_map.py`, `cli.py`, `DESIGN.md`, `test_spec_labels.py` — **plus §4.1's one adapter
comment**, `preflight.py:48`, prose naming a symbol this slice deletes, moving no behaviour.

⚠ Drafts 1-3 derived §12(3)'s shape from that (now qualified) membership claim: "a plain grep rather than a
set difference". What is true: **K2 being an absolute** makes it a plain grep — the block used to gate on a
`comm -13` delta of base against head and now greps the whole generic tree at once. The exception is
irrelevant either way, §12(3) ranging over `.claude/tools/_webref/` only. The block does still compute a set
difference, but for a **reported** line (*of which in A's half*), not its verdict — deriving A's half as the
generic tree minus B's files, since an inclusion list cannot see a file the slice creates.
→ `rederive couplings`

**One-issue-one-way**: the label enumeration goes three sites → one, two of the three in this slice.

---

## §8 Line-count budget

→ `rederive budget`. `spec_labels.py` is a new file well under any threshold; the two consumers lose lines.

⚠ **The harness split is A-i's, and it is done.** Drafts 1-3 said A-i "cites six of its blocks and does not
touch it" — false. Measured commit order: `261bfaa6` (840 L) → **`58338dd5`, the A-i carve** (840) →
`788825ab` (898) → `6be9c564` (901): A-i grew it 840 → 901 across **two post-carve commits**, both serving
§4.1 (`readers`) and §4.2 (`regions`), so A-i's own touch carried it past the 700-800 authoring band and the
umbrella's owed-split rule fell here. Discharged as A-i's prereq, before implementation: `06e50b41` split it
on the slice seam, `3987bfbc` gated `couplings` on K2's absolute and wrote down its predicate, `4121b667` made
`readers`' code/prose split a real partition with a loud-empty trip-wire. Layout now (`wc -l
…-A-rederive*.sh`): dispatcher **55**, `-common` **438**, `-Ai` **156**, `-Aii` **272**, `-B` **103**, `-Aiii`
**37**; 1061 total, no part past the band. ⚠ Honestly, the band was crossed *while writing* and the split came
at **review** time — per `memory/feedback_touch-time-split-means-while-writing.md` the compliant moment was
`788825ab`.

---

## §9 Edge-dense assessment

**A-i does not trip CLAUDE.md's trigger.** One invariant (labels resolve identically), a canonical algorithm
(dict lookup), zero `crates/**`, no control flow.

⚠ Drafts 1-3 left that reading contradicting **§2**, which is titled *Coupled invariants*, enumerates **four**
and tabulates **five** pairwise intersections (measured: `grep -cE '^- \*\*K[0-9]'` → 4;
`grep -cE '^\| K[0-9] × K[0-9]'` → 5). Both are right, over different things: CLAUDE.md's trigger (a) ranges
over intersecting **design** invariant axes — a subsystem's behaviour — whereas K1-K4 are **edit-hygiene**
invariants over this slice's own bookkeeping, coupled to each other and to nothing A-i executes. The
reconciliation belongs here, not in a weakening of §2.

⚠ Drafts 1-2 argued the umbrella base case instead and it was falsified twice on its own conjuncts. Simpler
and true: the *merged* Slice A tripped the trigger and A-i inherited its apparatus without earning it.
Conjunct (i) does hold — but ⚠ the umbrella's scope cell was last amended **during** this slice's review
(round 2's finding), on user approval and outside a slice commit, which is the self-ratification the re-slice
was careful to avoid.

---

## §11 Defer slots

**Zero own deferrals**, and now true rather than asserted. A-i creates no failable capability (§5's
reachability argument, not a membership claim), no network dependency (`SPECS` is pinned; the catalog
fall-through is B's), no scheduling gap. ⚠ Drafts 1-2 claimed this while migrating `preflight.py`, which was
false. ⚠ Draft 3 claimed it while §8 carried a live trigger — the harness's owed split — with neither a
`Why deferred` nor a `Re-evaluation date`; a deferral by omission is still a deferral. That trigger is
**discharged** in the three prereq commits §8 names, so nothing is owed at landing.

---

## §12 Exit criterion

Every diff check names an explicit ref. ⚠ Draft 3's were `git diff -- <path>` with none, which compares the
worktree to the index and returns 0 on **any** clean tree — vacuous. Measured at the time,
`git diff -- .claude/skills/ | wc -l` → **0** while `git diff origin/main...HEAD -- .claude/skills/ | wc -l` →
**89**, a `preflight.py` migration sitting at HEAD that the check could not see.

1. **Green**: `test_spec_labels.py` passes; `git diff origin/main...HEAD -- crates/` **empty**;
   `git diff origin/main...HEAD -- .claude/skills/` is **exactly the one `preflight.py:48` comment** of §4.1 —
   not empty, and any other hunk under `.claude/skills/` fails this check.
2. **K3**: at A-i's head, `git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/` → empty;
   `git grep -n '_catalog' -- …` → empty; `git grep -n 'webref_data' -- …/spec_labels.py` → empty. On
   `origin/main` these measure **0** and **0**, so under §4's base it passes by construction and its job is to
   catch a re-import from `b3a7d469`; at this branch's HEAD the same greps return **28** and **3** lines,
   which is what keeps it from being a tautology. (`webref_data` legitimately has 8 file hits elsewhere — K3.)
3. **K2**: `bash …-A-rederive.sh couplings` → `elidex file paths at HEAD (K2 / S8 — MUST BE 0)` = **0**, under
   §2's two-further-segments predicate. At this branch's HEAD it prints **RED** with two paths (`cli.py` and
   the branch's `spec_labels.py`; verified 2026-08-01) — correct and expected, A-i being unimplemented;
   `origin/main` has **1**, the one A-i discharges.
4. **K1/K4**: S3, S3b and S5 green.

Checks 2 and 3 are greps over prose occurrences, not over file assignments.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | takes `preflight.py`'s copy **and** its failure semantics, plus `SPEC_LABEL_REVERSE`'s census — the two gate-output strings, the four plan-memo readers, `#11-preflight-css-module-labels`, and the two hand-offs below | **A-i first** |
| **A-iii** | none | after A-ii |
| **Slice B** | takes every row marked B in §4.2, the fall-through, and the `@lru_cache` below | after A-ii |
| **Slice C** | shares `DESIGN.md`; A-i states its bullet verbatim, C owns the reported-class contract | after B |
| **PR-A0 (`elidex-wt-submittable`)** | touches the same `_webref` files — and carries the byte-identical tree §4 names as `b3a7d469`'s recovery location | after A/B/C; it rebases |
| **PR #496 / #497** | **none**, by file disjointness rather than by tree | none |

→ `rederive lanes` (ranges over the files A-i contends on, not only `docs/plans/`).

⚠ The #496 / #497 verdict rested on "A-i touches no `ci.yml`, `mise.toml` or `crates/**`", which does not
establish it: measured (`gh pr view 496 --json files -q '.files[].path'`), #496 also touches
`.claude/tools/layout-box-reader-trip-wire.sh` and `docs/plans/2026-07-terminal-z-c3a-impl-plan.md` — both of
the two trees A-i lives entirely inside. The real basis is **disjoint files**: #496 touches no `_webref` file,
no `elidex-plan-review/` file and no `…-citation-hygiene-*` memo; #497 is `crates/**` only.

**Frozen literals.** S5's 15 `SPEC_LABEL_REVERSE` pairs **and** S3b's vendored `COMMON_SHORTNAMES` blurb text
are both `origin/main` snapshots taken at vendoring time and refreshed never — which is what makes them pins
rather than mirrors (K4). ⚠ Draft 3 said this of S5 only. **A-ii must not refresh either.**

**Known hole → A-ii** (not an A-i defect: K4 asserts identity with `origin/main`, never completeness, and A-i
must not "fix" the map). The pinned label for `webidl` is **`Web IDL`, unprefixed**, though webref reports
`organization=WHATWG` for it and `xhr` is pinned `WHATWG XHR` — so under the frozen map
`shortname_for("WHATWG Web IDL")` is `None`, and that spelling occurs at **5** sites under `crates/`, all in
`crates/script/elidex-js/` (`git grep -cIE 'WHATWG Web ?IDL' -- . ':!docs/plans/2026-07-citation-hygiene*'`).
Found by Axis 4; A-ii owns the disposition.

**Owed to Slice B**: `sources/webref_data.py`'s `@lru_cache(maxsize=None)` on `try_fetch_data` (**+9 / −0**)
rides on `b3a7d469` and leaves A-i's lineage with it. A real optimization by its own docstring, sitting on
`heading`'s fetch path, so it is **routed, not dropped** — B owns the catalog fall-through, is the
many-lookups-per-spec consumer, and its memo already reasons from `try_fetch_data` being `@lru_cache`d (§10
Q3). The umbrella owes a matching row.

**Landing checklist**

1. Update `project_citation-hygiene-program.md` and `active-lane-detail.md` with A-i's outcome. ⚠ Draft 3 said
   the three places carrying this program's state "disagree on round 1's MIN count"; measured, all three agree
   on **1 CRIT / 28 IMP** and **none records a MIN count at all**. The real disagreement is the **draft
   number**, internal to two files: `project_citation-hygiene-program.md`'s frontmatter and
   `active-lane-detail.md:82` say *draft 1* while their own bodies (and `MEMORY.md`) say *draft 3*. Collapse to
   the program memo with pointers, frontmatter included.
2. Register nothing — A-i has no slots; `#11-preflight-css-module-labels` and
   `#11-webref-preflight-inprocess-resolution` are **A-ii's**. ⚠ Draft 3 said "measured, neither is in any
   ledger today": true of the **SoT file** (`project_open-defer-slots.md`, `grep -c` → 0), false in general —
   `git grep -n 'preflight-css-module-labels' origin/main -- docs/plans/` finds it registered in two **landed**
   memos, `2026-07-terminal-z-c3a-seam-and-audit-plan.md:655` (row 8, the authoritative hand-off) and
   `2026-07-terminal-z-c3a-impl-plan.md:538`. Owner **PM**; trigger **before the next plan-memo citing a CSS
   module — C-3b at the latest**. The C-3b lane is live, so A-ii inherits that deadline, not an open-ended
   re-homing.

---

## §14 Provenance

Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (nine rounds; recoverable at
`git show 707b69cc^:docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md` — the SHA an earlier
umbrella revision named was destroyed by a rebase). A-i's own three rounds and their counts are in §0; draft 3
narrowed the scope and stopped restating the diff, draft 4 states the base tree §4 is relative to.

## §15 Re-derivation

Entry point unchanged: `docs/plans/2026-07-citation-hygiene-A-rederive.sh <block>` — now a **55-line
dispatcher** sourcing five parts (`-common` `-Ai` `-Aii` `-Aiii` `-B`), so every block name still resolves
through that one path whichever part defines it (§8), verified by running each block A-i cites: **`citations
keysets readers regions couplings budget`**, plus `lanes` in §13. ⚠ Draft 3 listed `regions` while the body
cited it zero times, and called `lanes` author-local while citing it. `regions` is now cited in §4.2; `lanes`
is author-local in the harness's sense (`AUTHOR_LOCAL="lanes staleclaims"`, excluded from `all` because it
reads the machine's worktree list), which does not bar a memo from citing it.
