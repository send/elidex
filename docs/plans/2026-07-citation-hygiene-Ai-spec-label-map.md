# Plan — Slice A-i: one spec-label map in the generic tree

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i**. **Branch**:
`webref-cite-audit-tool`. **Status**: plan-memo, **draft 5**. `/elidex-plan-review` before implementation.

⚠ **The memo is a record, not a specification.** Per the umbrella's *review cost tracks blast radius*, **the
canonical statement of what the code does is the diff and the tests**; quantities come from
`docs/plans/2026-07-citation-hygiene-A-rederive.sh`, cited by block name.

### §0.1 What A-i is

`origin/main` carries one spec-label enumeration **three** times — `coverage_map._SPEC_LABEL_MAP`,
`cli.COMMON_SHORTNAMES`, `preflight.SPEC_LABEL_REVERSE`. A-i creates `.claude/tools/_webref/spec_labels.py`,
**pinned map only**, and collapses **the two in the generic tree** onto it.

⚠ **`preflight.py`'s map is not touched** — it migrates in **A-ii**. The four-cell measurement and its
conclusion (**the gate's copy is not separable from the gate's failure semantics**) are stated once, in the
umbrella's A-i row, and not restated here. So K1 completes across A-i + A-ii. A-i is inside the generic tree
with **one stated exception** — the `preflight.py` comment naming a symbol A-i deletes (§4.1), the *comment*
being the unit — and touches no gate semantics, no CI topology. The resolution delta this produces is stated
once, in §5.

---

## §0.5 Spec citation table

A-i implements no spec logic. Both labels are pinned by `SPECS`, per the umbrella's *a slice may only cite
labels its own resolver maps*. Looked up with `.claude/tools/webref`, nothing from memory.
→ `rederive citations`

| Cite | § | Exact title | Anchor |
|---|---|---|---|
| `WHATWG HTML §4.10.21` | HTML §4.10.21 | Constraints | `#constraints` |
| `WHATWG Fetch §2.2.5` | Fetch §2.2.5 | Requests | `#requests` |

Two rows, two **distinct pinned specs**, so K=2 and the table is not one spec twice. Measured, `WHATWG Fetch`
is `entry[1]`, the canonical label, resolving identically at baseline — the spellings that exercise the
shortname-as-parse-key rule are `Fetch` / `fetch`.

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
  **two further segments** and *generic core* is **`.claude/tools/`**, not `.claude/tools/_webref/`. The
  tool's own invocation path `.claude/tools/webref` is one segment and occurs **22** times in `origin/main`'s
  `cli.py`; excluding it is intended — an install path is not a path into elidex's tree — and
  `rederive couplings` carries the predicate in the block rather than leaving it implicit in a regex. An
  **absolute**, not a delta, and it has **two** pre-existing instances, not one:
  `git grep -noE '\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' origin/main -- .claude/tools/` →
  `_webref/cli.py:78` **and** `.claude/tools/webref:5`, carrying the byte-identical string
  `.claude/skills/elidex-review/axes.md.`. A-i discharges **both** by the same by-role rewrite. The entry
  point is generic core by any reading — 16 lines, a docstring plus `from _webref.cli import main`, the
  docstring being the site — and unlike `cli.py` it has no other routing at all.
  ⚠ **`rederive couplings` filters to `.claude/tools/_webref/`, now narrower than K2**, so it cannot witness
  the second site; the widening is an owed harness edit (§13), A-i not editing the harness inside its own
  review.
- **K3 — the generic core names no Slice-B artifact.** `cite-audit` and `_catalog` are absent from
  `.claude/tools/_webref/` and `.claude/skills/` (matching `origin/main`, measured 0 each); `webref_data` is
  absent from `spec_labels.py`. Measured (`git grep -lI 'webref_data' origin/main --
  .claude/tools/_webref/`), `webref_data` is **8 files, 6 of them command modules** (`css` `dfn` `element`
  `heading` `idl` `specs`) — the rest are `inventory.py` and `resolver.py`, neither a command module.
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

Measured, `git ls-tree origin/main -- .claude/tools/_webref/spec_labels.py` prints nothing and
`git show origin/main:.claude/tools/_webref/cli.py | grep -c cite-audit` is **0**.

The consequence is a lineage decision. Measured, exactly one commit carries the whole `.claude/`
implementation — `git log --oneline origin/main..HEAD -- .claude/` → `b3a7d469 tools(webref): carve the
cite-audit detector out of the citation sweep` (`git show --stat`: eight paths, +945 / −64). **It is dropped
from A-i's lineage.**

**The content is recoverable.** The pointer is *content plus a second location*, not a bare SHA (§14's lesson
— a SHA in a rebasing branch has a half-life): branch `domform-submittable-category` (worktree
`elidex-wt-submittable`) carries a byte-identical copy, and
`git diff --stat HEAD domform-submittable-category -- .claude/tools/_webref/
.claude/skills/elidex-plan-review/preflight.py` prints nothing. ⚠ Both paths are load-bearing — seven of the
eight live under `_webref/` and `preflight.py` is the eighth — and widening to `-- .claude/` instead is *not*
the fix: measured, that form prints six unrelated files.

⚠ **The decision surfaces an already-recorded debt, and A-i states it as owed rather than done.** B's and
A-ii's memos were authored while Slice A was one merged slice; the 2026-08-01 re-slice changed B's base and
neither memo was re-derived. `memory/project_citation-hygiene-program.md` records it verbatim: *"**▶ OWED:
B's memo re-slice edits.** Delegated once and the agent **died on an account monthly spend limit**, so B's
memo is **untouched**."* Measured, B's memo is written as repairs to a tree that exists at its base, and that
tree is `b3a7d469`'s. So the re-introductions are **owed, not done** — sites in §13.1, so the next author
need not re-derive the census. A-i does **not** re-derive them here: 696 and 578 lines of another slice's
content, which is the decision-surface duplication this program exists to remove. **The gates are B's and
A-ii's own plan-reviews**, neither of which passes on a memo whose base is wrong.

⚠ The obligation is not *created* by this decision: `cite_audit.py` cannot survive at A-i's head under any
lineage route, **K3** forbidding it. Dropping `b3a7d469` changes when the debt is visible, not that it
exists.

### §4.1 The rule this slice is built on

⚠ Four rounds running, the root finding has been *a read whose write-path the draft changes, without
reconciling the other readers of that state.* It is now a command: **before writing an edit-set row for a
piece of state, run `rederive readers <symbol> [ref]` and assign every line it prints** — the census ranges
over a ref (default `origin/main`) and separates code from prose, because every failed edit set assigned code
and left prose. Census for the two symbols A-i removes, and for the API it adds:

| symbol | code | prose |
|---|---|---|
| `_SPEC_LABEL_MAP` | `coverage_map.py` :13 :30 :31 | `preflight.py:48` — the "keep in sync" comment. **A-i's**, below |
| `label_for` / `shortname_for` | **none at `origin/main`** — the module is new, so `rederive readers label_for origin/main` trips its loud-empty guard, which is the correct signal here and not a clean bill of health | none |
| `COMMON_SHORTNAMES` | `cli.py` :27 :80 | **none** — measured; the blurb lines are the literal's own body, which A-i moves into `SPECS`, not a reader of the symbol |

⚠ Measured, `grep -nE '_SPEC_LABEL_MAP|keep in sync' …-Aii-gate-failure-semantics.md` → **no hits**. The
rule, once: **behaviour** travels with the gate's failure semantics (A-ii); **prose naming a symbol this
slice deletes** travels with the deletion (A-i). This is the single exception to "A-i touches no
adapter file" (§0.1, §7, §12(1)), and **the unit is the comment, not the line**: on `origin/main` the
sentence *"Mirror of `.claude/tools/webref` `_SPEC_LABEL_MAP` but reversed; keep in sync when adding new
specs to that map"* spans **:47-49** inside the `:47-50` block, and `:48` is where the symbol is spelled. No
other `preflight.py` line moves here. `SPEC_LABEL_REVERSE`'s census stays A-ii's: its two gate-output readers
(`preflight.py` :409, :422) and its **four plan-memo *files*** (six lines, measured), one of which registers
`#11-preflight-css-module-labels` — A-ii dispositions it (§13). Its seventh prose reader is
`preflight.py:342`, a comment inside the gate's own unparseable-mode explanation; A-ii's, with the rest of
that census. → `rederive readers`

### §4.2 What changes, by named artifact

Rows are re-derived against `origin/main`, where `_spec_label` is two statements, no docstring (measured).
The A/B region boundaries the `spec_labels.py` rows rest on → `rederive regions`.

| artifact | change |
|---|---|
| `spec_labels.py` — `SPECS` + the three derived dicts | **new**; **8 parse aliases omitted** (measured inert: the map is byte-identical without them, since each alias lowercases to its own shortname) |
| `spec_labels.py` — `label_for` / `shortname_for` | **new**, the module's whole API — `label_for` is `coverage_map`'s delegate, `shortname_for` A-ii's and S1's. Both pinned by §6 (S1, S2); no reader exists on `origin/main` to reconcile |
| `spec_labels.py` — module docstring | authored to say **two in the generic tree**; names no `cite_audit.py` (K3); names `preflight.py` **by role** (K2) |
| `spec_labels.py` — the `LABEL_TO_SHORTNAME` comment's load-time consumer list | same two constraints — it is a **second** consumer list |
| `spec_labels.py` — catalog paragraph, both function docstrings' catalog clauses | absent in A-i; **B** authors them with the fall-through |
| `cli.py` — blurb derivation | import `SHORTNAME_TO_BLURB`; the derived block must reproduce `origin/main`'s literal byte-identically (S3b) |
| `cli.py` — the new derivation comment | authored without the B artifact name (K3) |
| `cli.py:78` — `.claude/skills/elidex-review/axes.md.` | **by-role rewrite** (K2, absolute) — one of the **two** pre-existing instances |
| `.claude/tools/webref:5` — the same string in the entry point's docstring | **by-role rewrite**, the second instance. Outside `_webref/` but inside `.claude/tools/`, which is what K2 now scopes over |
| `coverage_map.py` — `_spec_label` | delegate to `label_for`; keep `origin/main`'s last-resort `.upper().replace("-", " ")` **verbatim** |
| `DESIGN.md` — the `spec_labels.py` bullet | new bullet, verbatim below |
| `DESIGN.md` — the `cite_audit.py` adapter paragraph + its 3 `cite-audit` example lines + the attribution-buckets paragraph | **absent in A-i**; they describe a command A-i does not ship. **B** authors them with the detector |
| `test_spec_labels.py` | **new** — one test per §6 pin (S1, S2, S3, S3b, S4, S5, S6, S7, S8, T-net) plus the four claims A-i's own comments make: case/space tolerance, unknown → `None`, the empty-`SPECS` re-exec pinning the comprehension form, and both directions round-tripping. ⚠ **The count is derived from §6, not inherited**: drafts 1-4 said "8 tests", a residue of the dropped `TestSharedSpecLabelMap`, whose 8 A-i tests reached S1/S2/S3 only — arithmetically short of 9 pins + T-net. Under §4's lineage the suite is **authored**, so §6 governs. `test_coverage_map_fallback_round_trips` is B's; A-i does not author it. No prose in it names `cite_audit`, and no test asserts over parse aliases, since A-i ships none |

Each row is scoped to **every occurrence** in the named artifact, not to a bullet list inside it.

⚠ **The round-trip fix `b3a7d469` carried, which A-i reverts and Slice B owns.** Its third `coverage_map.py`
hunk is not a move: it replaced `_spec_label`'s last resort `shortname.upper().replace("-", " ")` with
`label_for(shortname) or shortname`, plus an 11-line docstring giving the reason — `coverage-map css-text-3`
renders `CSS TEXT 3`, which `shortname_for` cannot read back, putting *"generator and plan-review gate out of
round-trip for every spec outside the pinned set"*. A-i reverts it: **K4** requires identical resolution and
A-i is a pure refactor, whereas this is a behaviour change whose correctness runs through the catalog
fall-through that decides what a non-pinned shortname resolves to. **Owner: Slice B**, with that
fall-through. It is the same round-trip defect class this program exists to fix, so it is named rather than
left to vanish. (B `:374` already reasons *from* it and quotes a docstring absent at B's base — §13.1.)

**Copy-count statements — five, all authored by A-i.** `origin/main` carries **no** copy-count claim anywhere
under `.claude/` (measured; the near hit `webref_data.py:57` "No hand-maintained alias map" carries no count),
so each is new prose and the constraint is on **wording**: `spec_labels.py`'s module
docstring, its `SPECS` header comment, `cli.py`'s derivation comment, `DESIGN.md`'s bullet, and
`test_spec_labels.py`'s class docstring — each saying **two in the generic tree** and naming only
`coverage_map` and `cli`'s blurb.

A-i's verbatim `DESIGN.md` bullet, stated here because Slice C shares the file:

> `spec_labels.py` is the single source for spec shortname ↔ display label. It replaced the hand-maintained
> copies in `commands/coverage_map.py` and `cli.py`'s help blurb, which had drifted apart.

B adds the fall-through sentence; A-ii adds the gate's copy to the list.

---

## §5 Behaviour delta

The one change in what the map admits is which **spellings** resolve. The spec set is unchanged — the same 12
pinned specs — and **9 additional spellings resolve**, the shortnames themselves, from the
shortname-as-own-parse-key rule rather than a widened alias list: 0 changed, 0 lost. Everything else —
canonical labels, the three real aliases (`HTML`, `DOM`, `URL`), non-pinned shortnames through
`coverage_map`'s last-resort — is unchanged. ⚠ It is **not observable inside A-i's tree**: nothing A-i ships
calls `shortname_for`, whose reverse direction the widening lives in. It is pinned by **S1** and consumed by
**A-ii**, when the gate's copy migrates. → `rederive keysets`

**A-i changes no gate behaviour — by reachability, not by file membership.** Measured,
`verify_citation` (`origin/main:.claude/skills/elidex-plan-review/preflight.py:265`) subprocesses
`[sys.executable, WEBREF, "heading", "--exact", …]` for **every citation it verifies**, and
`.claude/tools/webref` is `from _webref.cli import main` — so `cli.py` runs and `commands/coverage_map.py` is
imported on every gate run. What holds: `coverage_map._spec_label` has exactly **one** caller on
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
| **S8** | K2 as an **absolute**, under §2's predicate: no `.claude/(skills\|tools)/` + two-further-segments path anywhere in **`.claude/tools/`** | **yes** — `origin/main` has **two** (`_webref/cli.py:78`, `.claude/tools/webref:5`), of which the harness sees one; §2's note |
| **T-net** | across A-i's suite, `subprocess.run` is never called with the resolved `WEBREF` path and `urlopen` is never called | no |

**UNCHECKED, marked not omitted**: that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`; unreachable through the gate, and the gate is
A-ii's).

---

## §7 Layering

**VM host / ECS-native**: not applicable, no `crates/**` diff. **Generic core vs adapter**: A-i is generic —
`spec_labels.py`, `coverage_map.py`, `cli.py`, the `.claude/tools/webref` entry point, `DESIGN.md`,
`test_spec_labels.py` — **plus §4.1's one adapter comment**, `preflight.py:47-49`, prose naming a symbol this
slice deletes, moving no behaviour.

**K2 being an absolute** makes §12(3) a plain grep — the block greps the whole generic tree at once rather
than gating on a `comm -13` delta of base against head. The exception is irrelevant either way, the block
ranging over `.claude/tools/` (today, `.claude/tools/_webref/` — §12(3)'s stated gap). The block does still compute a set
difference, but for a **reported** line (*of which in A's half*), not its verdict — deriving A's half as the
generic tree minus B's files, since an inclusion list cannot see a file the slice creates.
→ `rederive couplings`

**One-issue-one-way**: the label enumeration goes three sites → one, two of the three in this slice.

---

## §8 Line-count budget

→ `rederive budget`. `spec_labels.py` is a new file well under any threshold; the two consumers lose lines.

**The harness split is A-i's, and it is done.** Measured commit order: `261bfaa6` (840 L) → **`58338dd5`,
the A-i carve** (840) → `788825ab` (898) → `6be9c564` (901): A-i grew it 840 → 901 across **two
post-carve commits**, both serving §4.1 (`readers`) and §4.2 (`regions`), so A-i's own touch carried it past
the 700-800 authoring band and the umbrella's owed-split rule fell here. Discharged as A-i's prereq, before
implementation: `06e50b41` split it
on the slice seam, `3987bfbc` gated `couplings` on K2's absolute and wrote down its predicate, `4121b667` made
`readers`' code/prose split a real partition with a loud-empty trip-wire. Layout now (`wc -l
…-A-rederive*.sh`): dispatcher **55**, `-common` **438**, `-Ai` **156**, `-Aii` **272**, `-B` **103**, `-Aiii`
**37**; 1061 total, no part past the band. ⚠ Honestly, the band was crossed *while writing* and the split came
at **review** time — per `memory/feedback_touch-time-split-means-while-writing.md` the compliant moment was
`788825ab`.

---

## §9 Edge-dense assessment

**The trigger fires on its text, and its prescribed remedy has been applied — twice.** §2 enumerates **four**
coupled invariants and **five** pairwise intersections (measured: `grep -cE '^- \*\*K[0-9]'` → 4;
`grep -cE '^\| K[0-9] × K[0-9]'` → 5), which is what CLAUDE.md's *"≥3 intersecting invariant axes"* names. Its
text carries **no *design* qualifier**, so partitioning K1-K4 into "design" and "edit-hygiene" is not a
reading of the rule — and the partition is falsified here anyway: **K4** is pinned by five of §6's executable
pins (S2, S3, S4, S5, S6) and produces §5's measured delta, and **K2** is a layering invariant §1 quotes from
`DESIGN.md`. §2 stands as written.

What terminates the recursion is CLAUDE.md's own remedy — *umbrella plan + PR ごとの plan に分割し各 PR を
個別に full review* — applied to the 785-line memo (→ A/B/C) and then to Slice A (→ A-i/A-ii/A-iii), both
under an approved umbrella, each slice carrying its own `/elidex-plan-review`. Clause (c) is then decisive
verbatim: *承認済 umbrella 配下で plan-review を通った narrowly-scoped per-PR slice は terminal 単位*, and
touching the same subsystem is explicitly not a re-split trigger.

⚠ That licenses **not re-splitting**; it does not license inheriting the merged slice's review apparatus.
That is the umbrella's separate *review cost tracks blast radius* constraint, which turns on blast radius
(zero `crates/**`, two consumers, a dict lookup), not on the trigger — so both hold without the partition.
⚠ Two umbrella facts follow: it states that constraint by reasoning from the trigger **not** firing on A-i,
so landing owes the amendment (§13); and its A-i scope cell was last amended **during** this slice's review
(round 2's finding), on user approval and outside a slice commit — the self-ratification the re-slice avoided.

---

## §11 Defer slots

**Zero own deferrals.** A-i creates no failable capability (§5's reachability argument, not a membership
claim), no network dependency (`SPECS` is pinned; the catalog fall-through is B's), no scheduling gap. The
harness's owed split, the one trigger §8 carried, is **discharged** in the three prereq commits §8 names, so
**no defer slot** is owed at landing. Owed *actions* are a different category and are §13's — the two harness
edits, the umbrella amendments, and the re-derivations B and A-ii owe.

---

## §12 Exit criterion

Every diff check names an explicit ref.

1. **Green**: `test_spec_labels.py` passes; `git diff origin/main...HEAD -- crates/` **empty**;
   `git diff origin/main...HEAD -- .claude/skills/` is **exactly the one `preflight.py:47-49` comment** of
   §4.1 — not empty, and any other hunk under `.claude/skills/` fails this check. At this branch's HEAD that
   diff is **89** lines (one file, +20 / −30 — `b3a7d469`'s `preflight.py` change, which §4 drops), so the
   check is live and currently red; A-i is unimplemented.
2. **K3**: at A-i's head, `git grep -nE 'cite.?audit' -- .claude/tools/_webref/ .claude/skills/` → empty;
   `git grep -n '_catalog' -- …` → empty; `git grep -n 'webref_data' -- …/spec_labels.py` → empty. On
   `origin/main` these measure **0** and **0**, so under §4's base it passes by construction and its job is to
   catch a re-import from `b3a7d469`; at this branch's HEAD the same greps return **28** and **3** lines,
   which is what keeps it from being a tautology. (`webref_data` legitimately has file hits elsewhere under
   `.claude/tools/_webref/` — **8 at `origin/main`, 10 at this branch's HEAD**; K3 forbids it only in
   `spec_labels.py`.)
3. **K2**: `bash …-A-rederive.sh couplings` → `elidex file paths at HEAD (K2 / S8 — MUST BE 0)` = **0**, under
   §2's two-further-segments predicate. At this branch's HEAD it prints **RED** with two paths (`cli.py` and
   the branch's `spec_labels.py`; verified 2026-08-01) — correct and expected, A-i being unimplemented;
   `origin/main` has **1 within the block's `_webref/` filter, 2 within K2's `.claude/tools/` scope**, and
   A-i discharges both. Until §13's owed harness edit widens the filter, this check under-covers K2 by
   exactly the entry-point site and can report GREEN on a claim K2 makes false — §2's note, stated once
   there.
4. **K1/K4**: S3, S3b and S5 green.

Checks 2 and 3 are greps over prose occurrences, not over file assignments.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | takes `preflight.py`'s copy **and** its failure semantics, plus `SPEC_LABEL_REVERSE`'s full census (§4.1) and `#11-preflight-css-module-labels`. ⚠ The two hand-offs below are **owed**, not routed — neither has a receiving site in A-ii today | **A-i first** |
| **A-iii** | none | after A-ii |
| **Slice B** | takes every row marked B in §4.2, the fall-through, and the `@lru_cache` below | after A-ii |
| **Slice C** | shares `DESIGN.md`; A-i states its bullet verbatim, C owns the reported-class contract | after B |
| **PR-A0 (`elidex-wt-submittable`)** | touches the same `_webref` files — and carries the byte-identical tree §4 names as `b3a7d469`'s recovery location | after A/B/C; it rebases |
| **PR #496 / #497** | **none**, by file disjointness rather than by tree | none |

→ `rederive lanes` (ranges over the files A-i contends on, not only `docs/plans/`).

Measured, `gh pr view 496 --json files -q '.files[].path'` prints **8** paths, of which **2** fall inside the
two trees A-i lives in — `.claude/tools/layout-box-reader-trip-wire.sh` and
`docs/plans/2026-07-terminal-z-c3a-impl-plan.md`; the other six (`ci.yml`, `CLAUDE.md`, `CONTRIBUTING.md`,
`docs/audits/…`, `mise.toml`, `scripts/trip-wires.sh`) are outside both and not the filter's subject. So the
#496 / #497 verdict rests on **disjoint files**, not disjoint trees: #496 touches no `_webref` file, no
`elidex-plan-review/` file and no `…-citation-hygiene-*` memo; #497 is `crates/**` only.

### §13.1 ⚠ OWED — B's and A-ii's memos are written against the pre-re-slice base

Not an A-i defect and **not A-i's to fix** (§4). Recorded here so the next author finds the sites without
re-deriving either census. **The gates are B's and A-ii's own `/elidex-plan-review` rounds**, neither of
which can pass on a memo whose base is wrong.

| memo | sites | why it is false at the new base |
|---|---|---|
| **B** (696 L) | `:11` *"Slice A lands first and B rebases onto it"*; `:13` *"Branch: new, cut from Slice A's landed head"*; `:18-21` describing the carve as having moved `cite_audit.py`, `spec_labels.py` and the rest "onto this branch **unchanged**"; `:578` / `:580` baselining 289 and 410 lines; **17** line-anchored `<file>.py:<n>` edits — 11 into `cite_audit.py`, 6 into `spec_labels.py` (`grep -coE '(cite_audit\|spec_labels)\.py:[0-9]'`) — concentrated in its §4.1's nine under-report paths; plus `:374` (§4.1) and `:618` / `:637` (below) | measured, `git cat-file -e origin/main:.claude/tools/_webref/commands/cite_audit.py` **fails**, as does the same test for `spec_labels.py`. B does not *repair* those files at its base; it **creates** them |
| **A-ii** (578 L) | `:148`, a routing row handing A-i *"`spec_labels.py`, the three consumers, `DESIGN.md`"* marked **landed** — which double-books `preflight.py`, claimed by its own next row; `:174` and `:504-505`, both premised on *"the asymmetry / the in-process reach is **created by A-i** moving the map"* | A-i has not moved `preflight.py`'s map since draft 3, and §12(1) now forbids it. A-i's `preflight.py` touch is one comment and adds no `_webref` import, so the asymmetry — and the deferral `:504-505` classes as **own** — are created by **A-ii** |

**Frozen literals.** S5's 15 `SPEC_LABEL_REVERSE` pairs **and** S3b's vendored `COMMON_SHORTNAMES` blurb text
are both `origin/main` snapshots taken at vendoring time and refreshed never — which is what makes them pins
rather than mirrors (K4). ⚠ **A-ii must not refresh either — and this is OWED, not routed**: measured,
`grep -ciE 'frozen|refresh|S3b|15 pairs' …-Aii….md` → **0**. Nothing in A-ii receives it today.

**Known hole → A-ii, OWED.** Not an A-i defect: K4 asserts identity with `origin/main`, never completeness,
and A-i must not "fix" the map. The pinned label for `webidl` is **`Web IDL`, unprefixed**, though webref
reports `organization=WHATWG` for it and `xhr` is pinned `WHATWG XHR` — so under the frozen map a
`WHATWG`-prefixed spelling returns `None`. ⚠ **The failing spelling is `WHATWG WebIDL`, no space** — and
A-ii does not receive this hand-off today (`grep -ciE 'web ?idl' …-Aii….md` → **0**). Measured,
`git grep -clI 'WHATWG Web IDL' -- . ':!docs/plans/2026-07-citation-hygiene*'` → **0** files;
`git grep -clI 'WHATWG WebIDL' …` → **5**, all in `crates/script/elidex-js/`: `src/vm/error.rs:33`,
`src/vm/host/fetch/mod.rs:258`, `src/vm/host/request_response/mod.rs:188`,
`src/vm/tests/tests_events_misc.rs:400`, `src/vm/tests/tests_worker.rs:832`. The `Web ?IDL` regex matches
both spellings, which is how the count 5 is right while the spaced spelling it was attached to would key a
remedy closing **0** of them. Found by Axis 4.

**Owed to Slice B — B *adds* it.** `sources/webref_data.py`'s `@lru_cache(maxsize=None)` on `try_fetch_data`
(**+9 / −0**) rides on `b3a7d469` and leaves A-i's lineage with it. A real optimization by its own
docstring — *"60 lookups were 60 identical HTTP GETs at ~46 ms, 18.6s of a 47.4s run"* — sitting on
`heading`'s fetch path, so it is **routed, not dropped**: B owns the catalog fall-through and is the
many-lookups-per-spec consumer. ⚠ **B's memo does not support this routing and must not be cited as if it
did**, and the umbrella does not carry it either (`grep -c 'lru_cache' …-umbrella.md` → **0**, so the
matching row is owed). B `:618` (§10 Q3) reasons from the decorator being *already present*, which at B's new
base it is not; `:637` files the resulting docstring/`--help` disagreement as a "**pre-existing** defect not
owned by this PR", a classification that inverts once B is the commit that adds it. Both fold into §13.1's
owed re-derivation.

**Landing checklist**

1. Update `project_citation-hygiene-program.md` and `active-lane-detail.md` with A-i's outcome. ⚠ The places
   carrying this program's state disagree on the **draft number**, internal to two files:
   `project_citation-hygiene-program.md`'s frontmatter and `active-lane-detail.md:82` say *draft 1* while
   their own bodies (and `MEMORY.md`) say *draft 3*. Collapse to the program memo with pointers, frontmatter
   included.
2. **Amend the umbrella** — a checklist item, not an edit, because §9 forbids A-i amending its own approval
   boundary during its own review. Six, each verified against the live file: (a) `:112-113` still registers
   the harness split as open ("901 lines, ~29 blocks … routed to none") though `06e50b41` discharged it (§8);
   and four falsified statements in the A-i row, all turning on the `origin/main` base under which none of
   the named artifacts exists — (b) "**A-i touches no adapter file**", falsified by §4.1's one comment;
   (c) "**delete** the 8 inert parse aliases", which A-i never creates, so **omits**; (d) "**move** the 8
   label-map tests", measured absent on `origin/main`
   (`git grep -lI '_SPEC_LABEL_MAP\|COMMON_SHORTNAMES' origin/main -- '*test*'` → empty), so A-i **authors**
   them; (e) "**correct** the copy-count claim at all five sites", where `origin/main` carries no such claim
   under `.claude/` at all, so all five are **authored** (§4.2). Plus (f) the *review cost tracks blast
   radius* bullet, which reasons from the edge-dense trigger **not** firing on A-i (§9), and the `@lru_cache`
   row above.
3. **Two owed harness edits**, both stated rather than made (§9's rule, and A-i's implementation touches no
   `docs/plans/` script): widen `couplings`'s path filter from `.claude/tools/_webref/` to `.claude/tools/`
   so S8 can witness K2's second site (§2, §12(3)); and move `suites` from `-Aiii.sh` to `-common.sh` — the
   harness's own seam rule is *cited by more than one memo → `-common.sh`*, and `suites` is cited by A-iii
   **and** the umbrella, which `-Aiii.sh:4` records as a known exception rather than fixing.
4. Register nothing — A-i has no slots; `#11-preflight-css-module-labels` and
   `#11-webref-preflight-inprocess-resolution` are **A-ii's**. ⚠ Neither is in the **SoT file**
   (`project_open-defer-slots.md`, `grep -c` → 0), but
   `git grep -n 'preflight-css-module-labels' origin/main -- docs/plans/` finds it registered in two **landed**
   memos, `2026-07-terminal-z-c3a-seam-and-audit-plan.md:655` (row 8, the authoritative hand-off) and
   `2026-07-terminal-z-c3a-impl-plan.md:538`. Owner **PM**; trigger **before the next plan-memo citing a CSS
   module — C-3b at the latest**. The C-3b lane is live, so A-ii inherits that deadline, not an open-ended
   re-homing.

---

## §14 Provenance

Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (nine rounds; recoverable at
`git show 707b69cc^:docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md` — the SHA an earlier
umbrella revision named was destroyed by a rebase).

## §15 Re-derivation

Entry point unchanged: `docs/plans/2026-07-citation-hygiene-A-rederive.sh <block>` — now a **55-line
dispatcher** sourcing five parts (`-common` `-Ai` `-Aii` `-Aiii` `-B`), so every block name still resolves
through that one path whichever part defines it (§8), verified by running each block A-i cites: **`citations
keysets readers regions couplings budget`**, plus `lanes` in §13. `regions` is cited in §4.2; `lanes`
is author-local in the harness's sense (`AUTHOR_LOCAL="lanes staleclaims"`, excluded from `all` because it
reads the machine's worktree list), which does not bar a memo from citing it.

⚠ **`readers` is in neither `all` nor `AUTHOR_LOCAL`**, so a reviewer who runs `all` gets **5 of A-i's 6**
blocks and no notice of the sixth. The exclusion is correct — `readers` takes a required `<symbol>` argument
and has no meaningful zero-arg form — but it is undeclared. Run it per symbol: `readers _SPEC_LABEL_MAP`,
`readers COMMON_SHORTNAMES`, `readers SPEC_LABEL_REVERSE`, `readers label_for origin/main` (§4.1, §4.2).
