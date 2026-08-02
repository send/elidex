# Plan — Slice A-i: one spec-label map in the generic tree

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i**. **Branch**:
`webref-cite-audit-tool`. **Status**: **draft 5, implemented.** `/elidex-plan-review` closed the gate on merit
at this draft; `/elidex-review` then returned 0 CRIT / 7 IMP / 15 MIN on the implementation plus this memo,
and those dispositions are applied in the same commit set as this revision. **The draft number is
deliberately unchanged** — the three memory files carrying this program's state all say draft 5 (checklist
item 1), and bumping it here would re-create the disagreement that item just cleared. Every quantity below is
re-measured at head.

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
  point is generic core by any reading — 16 lines at `origin/main`, a docstring plus
  `from _webref.cli import main`, the
  docstring being the site — and unlike `cli.py` it has no other routing at all.
  ✅ **`rederive couplings` now filters to `.claude/tools/`** and reports both pre-existing sites; the owed
  widening was taken in this commit set, not deferred (§13 item 3).
- **K3 — the generic core names no Slice-B artifact.** `cite-audit` and `_catalog` are absent from
  `.claude/tools/` and `.claude/skills/` (matching `origin/main`, measured 0 each at both refs); `webref_data`
  is absent from `spec_labels.py`. Measured (`git grep -lI 'webref_data' origin/main --
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
`#11-preflight-css-module-labels` — ⚠ that slot is **owed, not routed to A-ii**; see §13 checklist item 4,
which measures A-ii's single mention of it and finds it is a census row, not an obligation. Its seventh prose reader is
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
| `test_spec_labels.py` | **new** — **15 tests**, derived and re-counted, not inherited. **11 carry a §6 pin**: one each for S1, S2, S3, S3b, S4, S5, S6, S8 and T-net, and **two for S7** (the artifact-name scan and the `webref_data` clause are separate tests). **4 carry no pin**, one per claim A-i's own comments make: case/space tolerance, unknown → `None`, the empty-`SPECS` re-exec pinning the comprehension form, and **both directions composing into a round trip** — the fourth was *claimed by drafts 1-5 and absent from the suite*, so it is added here rather than dropped from the derivation (measured: it holds over all 12 rows, both ways). ⚠ Drafts 1-4 said "8 tests", a residue of the dropped `TestSharedSpecLabelMap`, whose 8 A-i tests reached S1/S2/S3 only; draft 5 said 10 pins + 4 extras = 14, which matched the file only because S7's double-count offset the missing round-trip. Under §4's lineage the suite is **authored**, so §6 governs and the arithmetic is 11 + 4 = 15. `test_coverage_map_fallback_round_trips` is B's; A-i does not author it. No prose in it names `cite_audit`, and no test asserts over parse aliases, since A-i ships none |

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

> `spec_labels.py` is the single source in the generic tree for spec shortname ↔ display label. It replaced
> the two hand-maintained copies there — `commands/coverage_map.py`'s label map and `cli.py`'s help blurb —
> which had drifted apart.

⚠ The **count and the "generic tree" qualifier are load-bearing**, and an earlier draft's bullet carried
neither: alone among the five copy-count sites it opened "single source for spec shortname ↔ display label",
which is *false* while `preflight.SPEC_LABEL_REVERSE` lives. B adds the fall-through sentence; **A-ii is what
drops the qualifier**, because A-ii is the slice after which the claim is true unqualified.

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
| **S3** | `coverage_map._spec_label` derives from `SPECS` — **perturbation**, not agreement: `SHORTNAME_TO_LABEL["html"]` is set to a sentinel under `try/finally` and the consumer must follow. ⚠ An agreement-only assertion **did not test this**: measured, replaying it against `origin/main`'s re-inlined `_spec_label` (`_SPEC_LABEL_MAP` + the same last resort) passes over all 12 rows, so K1's `coverage_map` half was unpinned in the slice whose thesis is that hand-maintained copies drift. Against that body the perturbation fails `'WHATWG HTML' != 'SPEC LABEL DERIVATION SENTINEL'` | **yes** |
| **S3b** | `cli.COMMON_SHORTNAMES`'s derived block reproduces `origin/main`'s literal byte-identically, **vendored as a literal** so the pin survives the slice that deletes the original | **yes** |
| **S4** | `LABEL_TO_SHORTNAME` is byte-identical with the 8 aliases omitted | **yes** |
| **S5** | `shortname_for` agrees with `origin/main`'s 15 `SPEC_LABEL_REVERSE` pairs, **vendored as a literal** — correct precisely because the point is to freeze the *old* table (K4) | no |
| **S6** | `_spec_label` over the 12 pinned shortnames **and** a non-pinned sample exercising the last-resort | no |
| **S7** | K3 by scan: `cite.?audit` and `_catalog` absent, and `webref_data` absent from `spec_labels.py`. ⚠ **The two ranges are NESTED, not split by tree** — an earlier draft of this row said "split", and there is no partition. Measured, walking each tree under the rule both scanners use: the unit suite ranges over `.claude/tools/_webref/`, **33** files (verified 2026-08-02); `rederive couplings` ranges over `.claude/tools/`, **39** files (verified 2026-08-02) — the same 33 **plus** 6 (`webref` and five elidex trip-wire artifacts) — and `.claude/skills/`, **10** files (verified 2026-08-02). The package is a **strict subset**, and the two scanners use the same regexes, so the suite's two tree-scanning tests have **zero discriminating power** over `couplings`: every plant the suite catches, `couplings` catches. What only `couplings` witnesses is the 6 + 10 files outside the package. The exception is S7's **third** clause — `webref_data` in `spec_labels.py` — which the suite checks and `couplings` does not | no — `origin/main` satisfies it at both ranges (measured 0), which is the point |
| **S8** | K2 as an **absolute**, under §2's predicate: no `.claude/(skills\|tools)/` + two-further-segments path anywhere in **`.claude/tools/`**. **Nested the same way**, not "the suite covers the package, `couplings` the rest": `couplings` covers the package **and** the 6 + 10 files outside it (verified 2026-08-02). ⚠ Whether that containment should be removed is a **scope** question over §2's K2/K3 definitions, routed to plan-review; this row only stops describing a partition that does not exist | **yes** — `origin/main` has **two** (`_webref/cli.py:78`, `.claude/tools/webref:5`), and after the §13-item-3 widening the harness sees **both** |
| **T-net** | **the import path** is inert: under `subprocess.run` and `urlopen` poisoned, both modules re-execute and answer. ⚠ Scoped to the import, not "across A-i's suite" — measured, it is one `patch(` block in one of 15 test methods (`grep -n 'def test_' …/test_spec_labels.py | wc -l` → 15), and that is the right scope: the module load is the thing the gate pays for on every citation, and re-executing it once under the poison is what exercises it | no |

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
than gating on a `comm -13` delta of base against head. The exception is irrelevant either way, the block now
ranging over `.claude/tools/` in fact and not only in intent. The block does still compute a set
difference, but for a **reported** line (*of which in A's half*), not its verdict — deriving A's half as the
generic tree minus B's files, since an inclusion list cannot see a file the slice creates.
→ `rederive couplings`

⚠ **The unit suite does not share that scope, deliberately.** `test_spec_labels.py` scans **`WEBREF_PKG`
only**. It is a test of the webref package, so locating a repo root in it would (a) make it the *first*
generic-tree file to do so — measured, the three pre-existing generic suites all stop at `parents[1]` and
`parents[3]` existed on `origin/main` solely in the adapter (`preflight.py:44`) — which inverts `DESIGN.md`'s
closing rule in code, in the slice whose subject is removing elidex paths from the generic core; and (b) put
five elidex trip-wire artifacts owned by other lanes inside a webref unit test's blast radius. Measured, those
five clear S8's predicate only by **path depth** (`.claude/tools/layout-box-reader-allowlist.tsv` is one
further segment, not two), so the Layout lane's next task — `#11-layoutbox-trip-wire-not-in-ci` — could turn
this package's suite red for a reason with no webref content. K2's and K3's cross-tree assertions therefore
live in `couplings`, which is where cross-tree assertions belong and where the widening was owed anyway.

⚠ **The two ranges are nested, not a partition** — an earlier draft of this section said the trees were
*split* between the instruments, and §6's S7/S8 said "split by tree" and "`rederive couplings` the rest".
Measured, walking each tree under the rule both scanners use (`__pycache__` skipped, undecodable files
skipped), and with the same two regexes on both sides: the suite ranges over `.claude/tools/_webref/` — **33**
files; `couplings` ranges over `.claude/tools/` — **39**, the same 33 plus `webref` and the five trip-wire
artifacts — plus `.claude/skills/` — **10**. The package range is a **strict subset** of the harness's.
Verified by planting a violation in each of the three trees, before and after: a package plant turns the
suite red **and** `couplings` RED, so the suite's two tree-scanning tests discriminate **nothing** that
`couplings` would miss; `.claude/tools/`-outside-the-package and `.claude/skills/` plants are caught by
`couplings` alone, and before the widening by neither. What the suite adds over `couplings` is S7's third
clause (`webref_data` in `spec_labels.py`, which `couplings` does not check) and the schedule it runs on —
not range. Whether the containment should be removed is a **scope** question over §2's K2/K3 definitions and
is routed to plan-review, not decided here.

**One-issue-one-way**: the label enumeration goes three sites → one, two of the three in this slice.

---

## §8 Line-count budget

→ `rederive budget`. `spec_labels.py` is a new file well under any threshold; the two consumers lose lines.

**The harness split is A-i's, and it is done.** ⚠ **The self-audit's dating was wrong and is corrected here.**
Measured per-commit (`git show <c>:…-A-rederive.sh | wc -l`): `b37d2ba3` **291** → `e5e73755` **634** →
`e0930ffb` **686** → `38f40eac` **799** → `261bfaa6` **840** → `58338dd5`, **the A-i carve**, **840** →
`788825ab` **898** → `6be9c564` **901**. So the file *entered* the 700-800 authoring band at `38f40eac` and
*left* it at `261bfaa6` — **both before the carve**. A-i did not carry it past the band; A-i **inherited it
already past** and added 61 lines across two post-carve commits, both serving §4.1 (`readers`) and §4.2
(`regions`). Per `memory/feedback_touch-time-split-means-while-writing.md` the compliant moment was
`38f40eac`, the commit that wrote it into the band — not `788825ab`, which an earlier draft named, and not
A-i's own touch. **The discharge is real and the disposition right; only the dating was wrong.** Discharged as
A-i's prereq, before implementation: `06e50b41` split it on the slice seam, `3987bfbc` gated `couplings` on
K2's absolute and wrote down its predicate, `4121b667` made `readers`' code/prose split a real partition with
a loud-empty trip-wire; §13 item 3's widening then landed in this commit set.

⚠ **The layout figures were stale and are re-derived here.** `37c7eb02` added `_wtscan` to `-common.sh`
*after* `89cc4051` and `4a3c4616` had recorded the layout, and the gate-status commit in this set added more.
So the three quantities this memo carried — `-common` **468**, **1091** total, **29** blocks — were falsified
by later commits *inside the commit set whose stated thesis is that every quantity was re-derived*. That is
the umbrella's own `:91` constraint (*"Counts are commands. No slice memo carries a quantity it did not
derive"*) failing on the memo that inherits it, and the failure mode is specifically **a count re-derived once
and then not re-derived after the next edit to the thing it counts**. Layout now, derived (`wc -l
…-A-rederive*.sh`; blocks by `cat …-A-rederive*.sh | grep -cE '^[A-Za-z_][A-Za-z0-9_]*\(\)'`): dispatcher
**68**, `-common` **550**, `-Ai` **156**, `-Aii` **272**, `-B` **103**, `-Aiii` **37**; **1186** total,
**30** blocks, largest part **550**, no part past the band.

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
⚠ Two umbrella facts follow. The first: it stated that constraint by reasoning from the trigger **not**
firing on A-i — ✅ **corrected in this commit set**, because that clause is a *reasoning* defect, not a scope
grant, and §9's bar is on a slice amending what it is approved to do (§13 checklist item 2). The second
stands: its A-i scope cell was last amended **during** this slice's review (round 2's finding), on user
approval and outside a slice commit — the self-ratification the re-slice avoided, and the reason clauses
(b)-(e) go to landing rather than here.

---

## §11 Defer slots

**Zero own deferrals.** A-i creates no failable capability (§5's reachability argument, not a membership
claim), no network dependency (`SPECS` is pinned; the catalog fall-through is B's), no scheduling gap. The
harness's owed split, the one trigger §8 carried, is **discharged** in the three prereq commits §8 names, so
**no defer slot** is owed at landing. Owed *actions* are a different category and are §13's — after this
commit set: **one** harness edit (`suites`' relocation; the `couplings` widening is taken), the umbrella's
four scope-grant clauses plus the `@lru_cache` row, the re-homing of `#11-preflight-css-module-labels`, and
the re-derivations B and A-ii owe.

---

## §12 Exit criterion

Every diff check names an explicit ref.

1. **Green**: `test_spec_labels.py` passes (**15 tests**, and the whole `_webref` suite is 27);
   `git diff origin/main...HEAD -- crates/` **empty** (measured **0** lines);
   `git diff origin/main...HEAD -- .claude/skills/` is **exactly the one `preflight.py:47-49` comment** of
   §4.1 — not empty, and any other hunk under `.claude/skills/` fails this check. Measured at head: **17**
   diff lines, one file, one hunk, +3 / −3. ⚠ An earlier draft recorded **89** lines here (+20 / −30) and
   called the check "currently red, A-i unimplemented" — that was `b3a7d469`'s `preflight.py` change, which
   §4 drops; both statements are now stale and the check is **green**.
2. **K3**: at A-i's head, `bash …-A-rederive.sh couplings` → `Slice-B artifact names at HEAD (K3 / S7 — MUST
   BE 0)` = **0** and **exit 0**; and `webref_data` is absent from `spec_labels.py`, which
   `test_spec_labels.py`'s `test_the_shared_map_does_not_reach_upstream` reads **off disk** (measured, `grep
   -c webref_data …/spec_labels.py` → **0**). Both measure **0** on `origin/main` too (`git grep -oE -e
   'cite.?audit' -e '_catalog' origin/main -- .claude/tools/ .claude/skills/` → **0**), so under §4's base
   this passes by construction and its job is to catch a re-import from `b3a7d469`.

   ⚠ **The head instrument is the working-tree scan, not `git grep`.** An earlier revision of this criterion
   specified `git grep -nE 'cite.?audit' -- .claude/tools/ .claude/skills/`. Measured, with `cite-audit`
   planted in an **untracked** file under `.claude/skills/`, that command exits **1** with empty output —
   which reads as a **pass** — while `couplings` reports **RED** and exits **1** on the same tree. `git grep`
   searches *tracked* files, so it cannot see a violation in a file that exists but has not been added, and
   this criterion's whole job is to catch a re-import — an act that starts as a new, unadded file. That is
   exactly the defect `37c7eb02` fixed **inside** the harness, left live one file over in the criterion the
   harness exists to serve. `git grep` is kept **only where an `origin/main` baseline is read**, above: a ref
   has no working tree to walk, and only git can read one.

   The scope stays `.claude/tools/` + `.claude/skills/`, not `.claude/tools/_webref/`; §7 records which
   instrument ranges over which tree. (`webref_data` legitimately has file hits elsewhere under
   `.claude/tools/_webref/` — measured **8 at `origin/main` and 8 at head**, an earlier draft's "10 at head"
   being `b3a7d469`'s two extra files; K3 forbids it only in `spec_labels.py`.)
3. **K2**: `bash …-A-rederive.sh couplings` → `elidex file paths at HEAD (K2 / S8 — MUST BE 0)` = **0**, under
   §2's two-further-segments predicate, and the block now **ranges over `.claude/tools/`**, so it witnesses
   both of K2's sites rather than one. Measured at head: **VERDICT GREEN**, **exit 0**, with the baseline line
   reporting `pre-existing on origin/main` = **2** (`_webref/cli.py:78` and `.claude/tools/webref:5`) — the
   count that was **1** while the filter was narrower, which is how the under-coverage was visible. The same
   block now also carries K3's cross-tree limb, so one verdict covers both invariants outside the package.

   ⚠ **The verdict is a return status, not only a printed line — three ways it used not to be.** Measured
   before this commit set, all three with a violation planted: (a) the block printed `VERDICT: RED` and
   **exited 0**, and inside `… all` (300+ lines of output) that RED line is unanchored text — `couplings` now
   `return 1`s on either RED, and `all` ends with a `FAILED BLOCKS:` roster and propagates; (b) the verdict
   was a `wc -l` over a producer whose exit status was discarded, and `wc -l` of nothing is **0**, which is
   the **pass** condition — with `python3` shadowed by `#!/bin/sh\nexit 127` the block printed `: 0` and
   `VERDICT: GREEN`, so *the scanner never ran* was indistinguishable from *no violations*; a scanner failure
   is now **RED** (`rc=127`, exit **1**), never green; (c) the scan roots were relative and resolved against
   **cwd** — run from `/` the dispatcher's `cd "$(git rev-parse --show-toplevel)"` errored, **no-opped**, and
   the block printed GREEN with the violation present, and run from a *sibling worktree* it audited that
   worktree instead of this branch's; the root now derives from `${BASH_SOURCE[0]}` and failing to resolve it
   exits **2** loudly. Re-measured after: RED/exit 1 from `/`, from a sibling worktree, and from a
   subdirectory alike.
4. **K1/K4**: S3, S3b and S5 green — and S3 is green *as a perturbation*, not as an agreement (§6), so K1's
   `coverage_map` half is now actually pinned.

Checks 2 and 3 are scans for prose occurrences, not for file assignments. **Every check named here has been
run against a deliberately planted violation** and observed red **and non-zero** — the plant matrix is
package-internal / `.claude/tools/`-outside-the-package / untracked-under-`.claude/skills/` / unplanted, plus
the shadowed-`python3` and three-cwd cases of item 3. A pin that cannot witness its own negation is not a
check; neither is one that witnesses it and then exits 0, which is what this commit set found.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | takes `preflight.py`'s copy **and** its failure semantics, plus `SPEC_LABEL_REVERSE`'s full census (§4.1). ⚠ The hand-offs below are **owed**, not routed — none has a receiving site in A-ii today, and that now includes `#11-preflight-css-module-labels` (checklist item 4) | **A-i first** |
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
| **B** (696 L) | `:11` *"Slice A lands first and B rebases onto it"*; `:13` *"Branch: new, cut from Slice A's landed head"*; `:18-21` describing the carve as having moved `cite_audit.py`, `spec_labels.py` and the rest "onto this branch **unchanged**"; `:578` / `:580` baselining 289 and 410 lines; **17** line-anchored `<file>.py:<n>` edits — 11 into `cite_audit.py`, 6 into `spec_labels.py` (`grep -coE '(cite_audit\|spec_labels)\.py:[0-9]'`) — concentrated in its §4.1's nine under-report paths; plus `:374` (§4.1) and `:618` / `:637` (below). ⚠ **And two sites where A-i's own work is what is at risk**, verified by line number: **`:581`**, the `test_spec_labels.py` row of the same size table, reading `\| — \| ~110 \| new (S1-S5) \|`; and **`:529`**, a `test_spec_labels.py` heading followed by B's **own S1–S5**, which mean different things than A-i's S1–S8 (B's S1 is a round-trip over 948 catalog entries, B `:529`'s own figure, verified 2026-08-01 against that line; A-i's S1 is `shortname_for` over `SPECS`). B `:470-471` cite the same file under the same numbering | measured, `git cat-file -e origin/main:.claude/tools/_webref/commands/cite_audit.py` **fails**, as does the same test for `spec_labels.py`. B does not *repair* those files at its base; it **creates** them. For `:581`/`:529` the consequence is sharper than staleness: an author working from B authors a fresh ~110-line file under a **colliding pin numbering** and drops A-i's S3, S3b, S4, S5, S6, S7, S8 and T-net — the only mechanical enforcement of K2 and K3 in the tree. Measured, A-i ships that file at **309 lines and 15 tests**, not `—` |
| **A-ii** (578 L) | `:148`, a routing row handing A-i *"`spec_labels.py`, the three consumers, `DESIGN.md`"* marked **landed** — which double-books `preflight.py`, claimed by its own next row; `:174` and `:504-505`, both premised on *"the asymmetry / the in-process reach is **created by A-i** moving the map"* | A-i has not moved `preflight.py`'s map since draft 3, and §12(1) now forbids it. A-i's `preflight.py` touch is one comment and adds no `_webref` import, so the asymmetry — and the deferral `:504-505` classes as **own** — are created by **A-ii** |

**Owed to Slice B — three assertions that pin the round-trip defect GREEN.** `test_spec_labels.py`'s S6 test
carries, at `:235-237` of the shipped file, `_spec_label("css-text-3") == "CSS TEXT 3"`,
`_spec_label("cssom-view-1") == "CSSOM VIEW 1"` and `shortname_for("CSS TEXT 3") is None`. They are correct
for A-i — §4.2's ⚠ explains why the last resort stays `origin/main`'s verbatim — and they are exactly what
B `:374` changes: under *"`label_for` must return a label that round-trips, or the shortname"* the first two
become **false**, so **B must delete them**, and nothing records that today. ⚠ The third is a different case
and is stated separately rather than folded in: `"CSS TEXT 3"` is neither a catalog title nor a shortname, so
under B's reverse index it plausibly still returns `None` — it does not become false, it becomes
**vestigial**, because the output it was pinning as unreadable is no longer the output. B disposes of it
either way; A-i does not assume which.

**Owed to Slice B — the `partition` block is broken by A-i's own K3, and was failing silently.** Measured,
`rederive partition` (a Slice-B block, in `…-A-rederive-B.sh`) calls `spec_labels._catalog()`, which A-i
removes from the generic tree because K3 forbids it. So the block has raised `AttributeError` since
`6be73a82`, and `all` **swallowed it** — the same discarded-exit-status bug the Step 4.5 pass found in
`couplings`, one level up. `all` now carries an anchored `FAILED BLOCKS:` roster and propagates, so
`bash …-A-rederive.sh all` exits **1** on this branch, reporting `partition(exit 1)`. That is correct
reporting, not a regression: **`all`'s exit status cannot be a green gate on this branch until B lands**, and
B is the slice that restores `_catalog`. ⚠ Do not "fix" it by reverting the roster — silence is what let it
run broken for four commits.

⚠ **ROUTED TO PLAN-REVIEW, umbrella altitude — K2/K3's scope is mis-drawn, and no check can be right until it
is redrawn.** A Trigger-B root-cause pass on the Step 4.5 fix found that §2 binds *generic core* to
`.claude/tools/`, while `DESIGN.md` defines the generic core by responsibility and lists modules all under
`_webref/`; its only occurrences of `.claude/tools/` are invocation examples. **Measured, the widening buys
zero evidence** — `git grep -oE '<PATHRE>' origin/main -- .claude/tools/` and the same restricted to
`.claude/tools/_webref/ .claude/tools/webref` both return **2**, the identical two sites — while importing
five other-lane artifacts (`layout-box-reader-allowlist.tsv`, `layout-box-reader-trip-wire.sh`, and three
`*-trip-wire.sh`) into A-i's §12(3) exit criterion, so a Layout-lane edit with no webref content can fail it.
K3 is mis-drawn a second way: its headline says *the generic core* names no Slice-B artifact, but its body
ranges over `.claude/skills/`, which by `DESIGN.md`'s own split is the **adapter**. The likely correction —
bind K2/K3 to `_webref/` plus the `.claude/tools/webref` entry script, and collapse to one enforcement point
— costs no evidence and removes all cross-lane coupling. **It is not A-i's to take**: re-stating a
plan-ratified invariant's scope routes to plan-review ([[feedback_plan-ratified-surface-is-a-design-change]]),
the canonical-site choice is shared with A-ii and A-iii (both cite `couplings`), and wiring a CI trip-wire
collides with the Layout lane's approved `#11-layoutbox-trip-wire-not-in-ci`.

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

**Second known hole → A-ii, OWED.** Same class, recorded the same way and for the same reason: pre-existing,
byte-identical to `origin/main`, so **K4 forbids A-i touching the map**. `webcrypto` is pinned as the
**series** label `Web Cryptography API`, unlevelled. Measured, `.claude/tools/webref specs Cryptography`
resolves to **`webcrypto-2  Web Cryptography API Level 2`** (the other two hits are
`webcrypto-modern-algos` and `webcrypto-secure-curves`; there is no bare `webcrypto` spec). So a memo row
written as `Web Cryptography API §N` verifies against **L2's numbering under a level-free label**, and will
silently re-target when the series advances to L3 — the same defect class as B's `CSSOM`→`cssom-1` /
`Selectors`→`selectors-4` re-pointing, arriving through the pinned map instead of the catalog. A-i ships the
map unchanged and hands the hole to A-ii's completeness pass, exactly as the `webidl` one is handed.
⚠ A-ii does not receive it today either: measured, `grep -ciE 'webcrypto' …-Aii….md` → **0**.

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

1. Update `project_citation-hygiene-program.md` and `active-lane-detail.md` with A-i's outcome, collapsing to
   the program memo with pointers, frontmatter included. ⚠ **The draft-number disagreement this item used to
   flag is now a no-op** — verified against the live files, `project_citation-hygiene-program.md` (frontmatter
   and `:49`/`:56`), `active-lane-detail.md:82` and `MEMORY.md` all say **draft 5**. The action stands; the
   warning is dropped.
2. **Amend the umbrella** — for the **four scope-grant clauses only**, because §9 forbids A-i amending *its
   own approval boundary* during its own review. All four sit in the A-i row and turn on the `origin/main`
   base under which none of the named artifacts exists: (b) "**A-i touches no adapter file**", falsified by
   §4.1's one comment; (c) "**delete** the 8 inert parse aliases", which A-i never creates, so **omits**;
   (d) "**move** the 8 label-map tests", measured absent on `origin/main`
   (`git grep -lI '_SPEC_LABEL_MAP\|COMMON_SHORTNAMES' origin/main -- '*test*'` → empty), so A-i **authors**
   them; (e) "**correct** the copy-count claim at all five sites", where `origin/main` carries no such claim
   under `.claude/` at all, so all five are **authored** (§4.2). Plus the `@lru_cache` row above.
   ✅ **(a) and (f) were separated out and fixed in A-i's own commit set**, and the separation is the point of
   §9's rule rather than an exception to it: §9 bars a slice from widening or narrowing *what it is approved
   to do*, which is what (b)-(e) state. (a) `:112-113` is a **status register** — "901 lines … Whichever slice
   next touches it splits it first" — that this PR **discharged** (`06e50b41`; measured, `wc -l
   …-A-rederive*.sh` is a 68-line dispatcher plus five parts, 1186 total, largest 550 — §8 re-derives these).
   Landing it as an open
   obligation would set A-ii's author up to redo a split already in the tree. (f) the *review cost tracks
   blast radius* bullet reasoned from "A-i has one invariant", which **the same commit-set falsifies**
   (`grep -cE '^- \*\*K[0-9]'` → 4, `grep -cE '^\| K[0-9] × K[0-9]'` → 5) and which A-i's own §9 no longer
   claims; it now reasons from blast radius, as it always should have. Neither changes A-i's scope by a line.
   Also corrected there: `umbrella:64`'s claim that `ee2d0dc0` "no longer exists (`git cat-file -e` fails)" —
   measured, `git cat-file -e` returns **0** and the blob still reads 1196 lines. The conclusion (prefer
   `<commit-that-deleted-it>^`) is sound, but on the ground of **unreachability**
   (`git branch -a --contains ee2d0dc0` → empty), not non-existence.
3. **Harness edits.** ✅ **Discharged in part**: `couplings`'s path filter is **widened** from
   `.claude/tools/_webref/` to `.claude/tools/`, so S8 witnesses K2's second site, and the block also gained
   K3's cross-tree limb (§2, §7, §12(2)/(3)). ⚠ **Still owed**: move `suites` from `-Aiii.sh` to
   `-common.sh` — the harness's own seam rule is *cited by more than one memo → `-common.sh`*, and `suites`
   is cited by A-iii **and** the umbrella, which `-Aiii.sh:4` records as a known exception rather than fixing.
4. Register nothing — A-i has no slots. `#11-webref-preflight-inprocess-resolution` is **A-ii's**, and A-ii's
   own §11 registers it. ⚠ **`#11-preflight-css-module-labels` is a different case and an earlier draft got
   it wrong**: this memo asserted it was A-ii's, but measured, A-ii's memo mentions it **once**, at `:150`,
   and that line is a *reader-census row* about `SPEC_LABEL_REVERSE`'s four plan-memo readers — not an
   obligation. A-ii's §11 lists **one** own slot and its landing checklist registers only that one. So the
   slot is **owed, not routed** — the same label §13's other two hand-offs carry, and for the same reason:
   after A-i lands it is absent from the SoT (`project_open-defer-slots.md`, `grep -c` → **0**) and owned by
   nobody. It survives only in two **landed** memos —
   `2026-07-terminal-z-c3a-seam-and-audit-plan.md:655` (row 8, the authoritative hand-off) and
   `2026-07-terminal-z-c3a-impl-plan.md:538` — registered with owner **PM** and trigger *before the next
   plan-memo citing a CSS module, C-3b at the latest*. ⚠ **And that trigger cannot be relied on to force a
   look**: an earlier draft said "the C-3b lane is live", which memory contradicts —
   `active-lane-detail.md:142` records *"C-3b–e は **parallel-safe でない**ため lane 対象外"* and
   `project_layoutbox-trip-wire-in-ci-next.md:61` *"C-3b–e stays ruled out (not parallel-safe)"*. C-3b is
   **not scheduled**, so the deadline is unbounded in practice and a real re-homing is what the slot needs.

---

## §14 Provenance

Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (nine rounds; recoverable at
`git show 707b69cc^:docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md` — the SHA an earlier
umbrella revision named was destroyed by a rebase).

## §15 Re-derivation

Entry point unchanged: `docs/plans/2026-07-citation-hygiene-A-rederive.sh <block>` — now a **68-line
dispatcher** sourcing five parts (`-common` `-Ai` `-Aii` `-Aiii` `-B`), so every block name still resolves
through that one path whichever part defines it (§8), verified by running each block A-i cites: **`citations
keysets readers regions couplings budget`**, plus `lanes` in §13. `regions` is cited in §4.2; `lanes`
is author-local in the harness's sense (`AUTHOR_LOCAL="lanes staleclaims"`, excluded from `all` because it
reads the machine's worktree list), which does not bar a memo from citing it.

⚠ **`readers` is in neither `all` nor `AUTHOR_LOCAL`**, so a reviewer who runs `all` gets **5 of A-i's 6**
blocks and no notice of the sixth. The exclusion is correct — `readers` takes a required `<symbol>` argument
and has no meaningful zero-arg form — but it is undeclared. Run it per symbol: `readers _SPEC_LABEL_MAP`,
`readers COMMON_SHORTNAMES`, `readers SPEC_LABEL_REVERSE`, `readers label_for origin/main` (§4.1, §4.2).
