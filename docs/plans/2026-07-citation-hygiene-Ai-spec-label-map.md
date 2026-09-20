# Plan — Slice A-i: one spec-label map in the generic tree

## §0 Status

**Umbrella**: `docs/plans/2026-07-citation-hygiene-umbrella.md`, slice **A-i**. **Branch**:
`webref-cite-audit-tool`. **Status**: **draft 5, implemented.** `/elidex-plan-review` closed the gate on merit
at this draft; `/elidex-review` then returned 0 CRIT / 7 IMP / 15 MIN on the implementation plus this memo,
and those dispositions are applied in the same commit set as this revision. **The draft number is
deliberately unchanged** — the **two** memory files that carry a draft number for this program
(`project_citation-hygiene-program.md`, `active-lane-detail.md`) both say draft 5 (checklist item 1), and
bumping it here would re-create the disagreement that item just cleared. ⚠ **This said "the three memory
files" and item 1 named `MEMORY.md` as the third until #501 gate 4**; measured,
`grep -oiE 'draft ?[0-9]+' MEMORY.md | wc -l` → **0** — it carries no draft number for this program at
all, so a three-file agreement was never checkable. Every quantity below is
re-measured at head.

⚠ **The memo is a record, not a specification.** Per the umbrella's *review cost tracks blast radius*, **the
canonical statement of what the code does is the diff and the tests**; quantities come from
the real gates listed in §15 — `preflight.py`, the unit suite, and the K2 trip-wire.

### §0.1 What A-i is

`origin/main` carries one spec-label enumeration **three** times — `coverage_map._SPEC_LABEL_MAP`,
`cli.COMMON_SHORTNAMES`, `preflight.SPEC_LABEL_REVERSE`. A-i creates `.claude/tools/_webref/spec_labels.py`,
**pinned map only**, and collapses **the two in the generic tree** onto it.

⚠ **`preflight.py`'s map is not touched** — it migrates in **A-ii**. The four-cell measurement and its
conclusion (**the gate's copy is not separable from the gate's failure semantics**) are stated once, in the
umbrella's A-i row, and not restated here. So K1 completes across A-i + A-ii. A-i is inside the generic tree
with **three stated exceptions**, derived rather than listed
(`git diff --name-only origin/main...HEAD` minus `.claude/tools/_webref/` and `.claude/tools/webref`): the
`preflight.py` comment naming a symbol A-i deletes (§4.1), the *comment* being the unit; the K2 trip-wire
`.claude/tools/webref-generic-core-trip-wire.sh`; and the one line registering it in
`scripts/trip-wires.sh`. ⚠ **This sentence said "one stated exception … touches no gate semantics, no CI
topology" until #501 gate 4.** It was written seven weeks before the harness drop added the wire, and the
last two exceptions are precisely gate semantics and CI topology. The resolution delta this produces is
stated once, in §5.

---

## §0.5 Spec citation table

A-i implements no spec logic. Both labels are pinned by `SPECS`, per the umbrella's *a slice may only cite
labels its own resolver maps*. Looked up with `.claude/tools/webref`, nothing from memory.
→ `python3 .claude/skills/elidex-plan-review/preflight.py <this memo>`; a single pair is
`.claude/tools/webref heading --exact html 4.10.21`.
⚠ **The gate does NOT read this table** — it anchors on `## §3 Spec coverage map` and reports
`heading line: 106`. These two rows are verified only because §3 repeats them. Measured at gate 4 by
perturbation: fabricating this table's §-number *and* its title leaves the run at EXIT 0, while the same
fabrication in §3 hard-fails (`❌ HARD FAIL — citation verification`). Two citation tables in one memo is
one home too many; §3 is the one with a gate behind it.

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
generic behavior free of elidex-specific file paths and put elidex policy in adapter commands or
documentation.* **The unit of this edit is the named artifact**, not the file and not the code branch: every
occurrence of a Slice-B artifact name, every elidex file path and every copy-count claim is either rewritten
or explicitly assigned, and the enumeration of those occurrences is **derived**, not authored (§4.1).

---

## §2 Coupled invariants

- **K1 — one enumeration in the generic tree.** After A-i, `coverage_map` and `cli` import rather than
  enumerate. `preflight`'s copy is A-ii's; K1 completes there.
- **K2 — the generic core names no elidex file path**, where *file path* means `.claude/(skills|tools)/` plus
  **two further segments** and *generic core* is **`.claude/tools/_webref/` plus the entry script
  `.claude/tools/webref`**. ⚠ **That is a by-DIRECTORY approximation of `DESIGN.md`'s by-RESPONSIBILITY
  split, and the two are not co-extensive** — this memo called it "`DESIGN.md`'s by-responsibility
  definition" until #501 gate 4. `DESIGN.md:28-37` divides by responsibility and names no directory;
  measured, two modules inside the approximation do adapter work (`commands/agent_brief.py`, which
  `DESIGN.md:47` itself describes as scanning *elidex* paths, and `cli.py`'s `--paths` default
  `["docs", "crates", "CLAUDE.md"]`, which is the "impacted `docs`/`crates` path heuristics" bullet as a
  literal). Both are pre-existing, so they are evidence about the definition rather than A-i defects — but a
  green from the wire is evidence about the directory, not about `DESIGN.md` compliance, and §12(3) must not
  be read as more. ✅ **Redrawn at #501 R36** (the
  plan-review altitude for this memo is this PR's converge): an earlier revision bound it to all of
  `.claude/tools/`, which measured bought zero evidence (both pre-existing sites lie inside `_webref/cli.py`
  and `webref`) and imported five other-lane trip-wire artifacts into §12(3). The
  tool's own invocation path `.claude/tools/webref` is one segment and occurs **22** times in `origin/main`'s
  `cli.py`; excluding it is intended — an install path is not a path into elidex's tree — and
  `.claude/tools/webref-generic-core-trip-wire.sh` carries the predicate in its header rather than leaving it
  implicit in a regex. An
  **absolute**, not a delta, and it has **two** pre-existing instances, not one:
  `git grep -noE '\.claude/(skills|tools)/[A-Za-z0-9_-]+/[A-Za-z0-9_.-]+' origin/main -- .claude/tools/` →
  `_webref/cli.py:78` **and** `.claude/tools/webref:5`, carrying the byte-identical string
  `.claude/skills/elidex-review/axes.md.`. A-i discharges **both** by the same by-role rewrite. The entry
  point is generic core by any reading — 16 lines at `origin/main`, a docstring plus
  `from _webref.cli import main`, the
  docstring being the site — and unlike `cli.py` it has no other routing at all.
  ✅ **The K2 trip-wire ranges over exactly the generic core** (`_webref/` + `webref`) **and decides over
  it**: **one** absolute, §2's K2 predicate itself (§12(3)). ⚠ **It carried a
  third, report-only "seed" scan until #501 R69** — a wide `<any top-level entry>/<something>` match that
  printed and asserted nothing. It is gone rather than demoted: it took four rounds of widening, the repo's
  rule is that such a predicate is a seed and widening it is the wrong repair, and a print with no consumer
  and no assertion is what CLAUDE.md calls dead code. The open class is covered by the diff (§12(3)).
- **K3 — the generic core names no Slice-B artifact.** `cite-audit` and `_catalog` are absent from the
  generic core (`_webref/` + `webref`; matching `origin/main`, measured 0 at both refs — `.claude/skills/` is
  the **adapter** by `DESIGN.md`'s split and is no longer in K3's range, #501 R36); `webref_data`
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
| K1 × K4 | omitting the aliases must leave the map byte-identical, which is what makes it safe in a refactor slice (the unit suite's S5/S6) |
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
`git show origin/main:.claude/tools/_webref/cli.py | grep -F cite-audit | wc -l` is **0**.

The consequence is a lineage decision. Measured, exactly one commit carries the whole `.claude/`
implementation — `git log --oneline origin/main..HEAD -- .claude/` → `b3a7d469 tools(webref): carve the
cite-audit detector out of the citation sweep` (`git show --stat`: eight paths, +945 / −64). **It is dropped
from A-i's lineage.**

**The content is recoverable**, by §14's rule: `b3a7d469` is an ancestor of this PR's head, so
`refs/pull/501/head` keeps it reachable after the squash merge deletes the branch. ⚠ **An earlier revision
of this sentence offered a second location instead** — "branch `domform-submittable-category` (worktree
`elidex-wt-submittable`) carries a byte-identical copy", witnessed by
`git diff --stat HEAD domform-submittable-category -- .claude/tools/_webref/
.claude/skills/elidex-plan-review/preflight.py` printing nothing. The diff is still empty, but the location
is not one a reader of `main` can reach: `git ls-remote origin domform-submittable-category` prints **0
refs** — that branch exists only on the authoring machine, which is a weaker pointer than the SHA it was
introduced to shore up, not a stronger one. ⚠ Both paths are load-bearing — seven of the
eight live under `_webref/` and `preflight.py` is the eighth — and widening to `-- .claude/` instead is *not*
the fix: measured, that form prints six unrelated files.

⚠ **The decision surfaces an already-recorded debt, and A-i states it as owed rather than done.** B's and
A-ii's memos were authored while Slice A was one merged slice; the 2026-08-01 re-slice changed B's base and
neither memo was re-derived. `memory/project_citation-hygiene-program.md` records it, verbatim: *"the
long-standing debt recorded as \"B's memo re-slice edits, delegated once, agent died on a spend limit, B's
memo untouched\""*. ⚠ **This quotation was a paraphrase until #501 gate 4** — reflowed, with a `▶ OWED:`
prefix the record does not carry and "an account monthly spend limit" for "a spend limit". §13.1's own rule
is that **the quoted sentence is the locator**, and a paraphrase returns nothing to `grep -F`. Measured, B's memo is written as repairs to a tree that exists at its base, and that
tree is `b3a7d469`'s. So the re-introductions are **owed, not done** — sites in §13.1, so the next author
need not re-derive the census. A-i does **not** re-derive them here: two memos' worth of another slice's
content, which is the decision-surface duplication this program exists to remove. (An earlier revision gave
their sizes as literal line counts; both were stale, and this document carried two different values for one
of them, four hundred lines apart.) **The gates are B's and
A-ii's own plan-reviews**, neither of which passes on a memo whose base is wrong.

⚠ The obligation is not *created* by this decision: `cite_audit.py` cannot survive at A-i's head under any
lineage route, **K3** forbidding it. Dropping `b3a7d469` changes when the debt is visible, not that it
exists.

### §4.1 The rule this slice is built on

⚠ Four rounds running, the root finding has been *a read whose write-path the draft changes, without
reconciling the other readers of that state.* It is now a command: **before writing an edit-set row for a
piece of state, run `git grep -nw <symbol> -- .claude docs` and assign every line it prints** — the census ranges
over a ref (default `origin/main`) and separates code from prose, because every failed edit set assigned code
and left prose. Census for the two symbols A-i removes, and for the API it adds:

| symbol | code | prose |
|---|---|---|
| `_SPEC_LABEL_MAP` | `coverage_map.py` :13 :30 :31 | `preflight.py:48` — the "keep in sync" comment. **A-i's**, below |
| `label_for` / `shortname_for` | **none at `origin/main`** — the module is new, so `git grep -nw label_for origin/main -- .claude` is empty, which is the correct signal here and not a clean bill of health | none |
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
that census. → `git grep -nw <symbol> -- .claude docs` (the census this line describes; §15).

### §4.2 What changes, by named artifact

Rows are re-derived against `origin/main`, where `_spec_label` is two statements, no docstring (measured).
The A/B region boundaries the `spec_labels.py` rows rest on → `git diff origin/main...HEAD -- .claude/tools/_webref/spec_labels.py`.

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
| `webref-generic-core-trip-wire.sh` | **new** — the K2 enforcement, registered in `scripts/trip-wires.sh`'s `REQUIRED_WIRES` (one added line, the second file outside the generic tree). ⚠ **Both were missing from this supposedly exhaustive table until #501 R68.** They are not a post-boundary addition: they are where K2's enforcement went when the harness left A-i (§15), the same obligation relocated. **One absolute**: §2's K2 predicate. ⚠ **It was two until #501 R70** — a `grep -F` pin on the host path A-i removed, plus the predicate. Measured, the removed path *is* a `.claude/(skills\|tools)/<a>/<b>` string, so the pin was strictly contained: no input fires it without firing K2. Two checks where one contains the other are a second decision surface, and this pair cost a real control — the pin's own fixture was caught by the K2 arm, so deleting the pin's verdict left that control green. The historical fact it carried is now in the wire's header and its failure message. **One control per verdict the wire can reach**, each asserted on its own **exit status** over a fixture tree (green; K2 firing under `skills`, under `tools`, on the removed path, inside **binary** content and on a **symlink's stored target**; an empty scope, an unreadable file, an unsearchable directory and an entry that is none of file/symlink/directory all failing closed) — an earlier shape inspected internal variables instead and three mutations survived it. ⚠ **The binary one is not hypothetical**: `grep -I` is `--binary-files=without-match`, so a file holding a NUL was reported as *not matching* rather than as unscannable and was certified while its content went unread (#501 R71; two empty `__init__.py` in this very tree are already classified binary by `file --mime`). Scanning is `-a` now. **Mutation-verified 21 ways.** ⚠ Four rounds (R70–R73) each found a different way for the wire to certify K2 over something it had not examined — a file it could not read, a file whose content `grep -I` skipped, a subtree `find` could not descend into, and a **symlink**, which `-type f` excludes although git stores its target string as the blob. The first three were one defect: the count and the scan were **two independently derived quantities**, now one (the count *is* what the scan read to the end), so "counted but not scanned" is not representable. The fourth was a second defect of the same family in the *population*: the walk enumerated regular files rather than entries. It now prints every entry and each lands in exactly one arm — symlink read with `readlink`, regular file with `grep`, directory skipped (no stored text), **anything else an error**, because a wire that cannot say what an entry holds must not certify over it. Both corrections are by construction; neither adds a rule about which entries may be skipped. ⚠ **R74 then found four more, all in the walk's mechanics rather than its shape**: `find -print` + `read` splits a filename containing a newline into fragments (`-print0` / `read -d ''`); `-name __pycache__ -prune` pruned a regular *file* of that name (`-type d`); the segment class excluded `@` and non-ASCII (broadened — see §12(3) for what stays out); and `$0` without a slash resolved the controls' self-invocation through `PATH` (canonicalised once). One mutant is equivalent under the registered driver and recorded as such: reverting that last fix is only observable when the script is launched as a bare name from its own directory, measured both ways (`$0` → controls 127, exit 1; `$SELF` → exit 0). **Shell and grep only, bash 3.2**: `.github/workflows/ci.yml` runs the driver with no setup step ("the wires are grep-only") and CLAUDE.md rests the ungated-job decision on that, so a wire needing a toolchain would break the premise for all five. ⚠ **This wire used `python3` until #501 R69** and did exactly that | derive with `git diff --name-only origin/main...HEAD` — three trees, not two |
| `test_spec_labels.py` | **new** — **13 tests**, derived and re-counted, not inherited (⚠ **15 until the third design re-gate**, which moved S7's first clause and S8 out of the generic suite — see §7). **9 carry a §6 pin**: one each for S1, S2, S3, S3b, S4, S5, S6 and T-net, and **one for S7** (its third clause; the artifact-name scan and the `webref_data` clause are separate tests). **4 carry no pin**, one per claim A-i's own comments make: case/space tolerance, unknown → `None`, the empty-`SPECS` re-exec pinning the comprehension form, and **both directions composing into a round trip** — the fourth was *claimed by drafts 1-5 and absent from the suite*, so it is added here rather than dropped from the derivation (measured: it holds over all 12 rows, both ways). ⚠ Drafts 1-4 said "8 tests", a residue of the dropped `TestSharedSpecLabelMap`, whose 8 A-i tests reached S1/S2/S3 only; draft 5 said 10 pins + 4 extras = 14, which matched the file only because S7's double-count offset the missing round-trip. Under §4's lineage the suite is **authored**, so §6 governs and the arithmetic is **9 + 4 = 13**, which is what the file measures (`grep -c 'def test_'` → 13; `python3 -m unittest _webref.test_spec_labels` → `Ran 13`). ⚠ **This clause read `11 + 4 = 15` until Codex R58**: the count at the head of this row was re-derived when the two duplicated slice-boundary tests left the suite, and the arithmetic clause further down the same row was not, so one row asserted both numbers. `test_coverage_map_fallback_round_trips` is B's; A-i does not author it. No prose in it names `cite_audit`. ⚠ **Two descriptions in this row were of the retired layout until #501 R68**: the artifact-name scan is **no longer a test at all** (R55 deleted it; only S7's `webref_data` module-shape clause remains), and this row ended *"no test asserts over parse aliases, since A-i ships none"* — `test_the_eight_omitted_parse_aliases_are_inert` (S4) asserts over exactly those eight, which is *how* it shows that omitting them is a refactor rather than a behaviour change. Re-derive from the shipped suite: `grep -n 'def test_' .claude/tools/_webref/test_spec_labels.py` |

Each row is scoped to **every occurrence** in the named artifact, not to a bullet list inside it.

⚠ **The round-trip fix `b3a7d469` carried, which A-i reverts and Slice B owns.** Its third `coverage_map.py`
hunk is not a move: it replaced `_spec_label`'s last resort `shortname.upper().replace("-", " ")` with
`label_for(shortname) or shortname`, plus an 11-line docstring giving the reason — `coverage-map css-text-3`
renders `CSS TEXT 3`, which `shortname_for` cannot read back, putting *"generator and plan-review gate out of
round-trip for every spec outside the pinned set"*. A-i reverts it: **K4** requires identical resolution and
A-i is a pure refactor, whereas this is a behaviour change whose correctness runs through the catalog
fall-through that decides what a non-pinned shortname resolves to. **Owner: Slice B**, with that
fall-through. It is the same round-trip defect class this program exists to fix, so it is named rather than
left to vanish. (B's §4.1 round-trip clause already reasons *from* it and quotes a docstring absent at B's base — §13.1.)

**Copy-count statements — five, all authored by A-i.** `origin/main` carries **no** copy-count claim anywhere
under `.claude/` (measured; the near hit `.claude/tools/_webref/sources/webref_data.py:57` "No hand-maintained alias map" carries no count),
so each is new prose and the constraint is on **wording**: `spec_labels.py`'s module
docstring, its `SPECS` header comment, `cli.py`'s derivation comment, `DESIGN.md`'s bullet, and
`test_spec_labels.py`'s class docstring — each saying **two in the generic tree** and naming only
`coverage_map` and `cli`'s blurb.

A-i's verbatim `DESIGN.md` bullet, stated here because Slice C shares the file:

> `spec_labels.py` is the single source in the generic tree for spec shortname ↔ display label. It replaced
> the two hand-maintained copies there — `commands/coverage_map.py`'s label map and `cli.py`'s help blurb —
> which nothing bound together.

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
**A-ii**, when the gate's copy migrates. → `cd .claude/tools && python3 -m unittest _webref.test_spec_labels`
(S5 asserts every gate spelling still resolves)

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
| **S7** | K3 by scan. ⚠ **Only the third clause is a suite pin now** — the artifact-name scan is a one-off diff check (§15), and the two tree-scanning tests were **deleted** at the third design re-gate (Codex R55) because they were a second copy of a scan the K2 trip-wire already makes over a strictly larger range (the package **plus** the `webref` entry script). What the suite pins is `webref_data` absent from `spec_labels.py` — a property of the module's shape, not of any slice: a literal label map has no business importing the upstream fetcher | no — `origin/main` satisfies it (measured 0), which is the point |
| **S8** | K2 as an **absolute**, under §2's predicate: no `.claude/(skills\|tools)/` + two-further-segments path anywhere in the **generic core** (`_webref/` + the `webref` entry script). ⚠ **Enforced by `.claude/tools/webref-generic-core-trip-wire.sh`**, registered in `scripts/trip-wires.sh`'s `REQUIRED_WIRES` and therefore run on every PR to `main`; it covers the package *and* the entry script, so the suite's package-only copy was the nested one and was removed (§7). ⚠ **Between #501 R55 and gate 4 this cell was FALSE**: R55 deleted the suite's copy as "a second copy of a scan the wire already makes", R62 then demoted that scan to report-only, and nobody restored the assertion — so a file naming `.claude/skills/elidex-plan-review/preflight.py` passed the wire GREEN. Gate 4 restored it by asserting §2's predicate itself; R70 then collapsed the older removed-path pin into it (strictly contained — see §4.2) and made every control assert the wire's exit status. Mutation-verified twelve ways. ✅ The containment question is closed at #501 R36 — the scope is the generic core and nothing outside it | **yes** — `origin/main` has **two** (`_webref/cli.py:78`, `.claude/tools/webref:5`), and the wire's pin sees **both** |
| **T-net** | **the import path** is inert: under `subprocess.run` and `urlopen` poisoned, both modules re-execute and answer. ⚠ Scoped to the import, not "across A-i's suite" — measured, it is one `patch(` block in one of 13 test methods (`grep -rn 'def test_' .claude/tools/_webref/test_spec_labels.py | wc -l` → 13, verified 2026-09-20), and that is the right scope: the module load is the thing the gate pays for on every citation, and re-executing it once under the poison is what exercises it | no |

**UNCHECKED, marked not omitted**: that `shortname_for` and `origin/main`'s `shortname_from_label` are
equivalent *functions* (`shortname_for` calls `.strip()`; unreachable through the gate, and the gate is
A-ii's).

---

## §7 Layering

**VM host / ECS-native**: not applicable, no `crates/**` diff. **Generic core vs adapter**: A-i is generic —
`spec_labels.py`, `coverage_map.py`, `cli.py`, the `.claude/tools/webref` entry point, `DESIGN.md`,
`test_spec_labels.py` — **plus §4.1's one adapter comment**, `preflight.py:47-49`, prose naming a symbol this
slice deletes, moving no behaviour.

**K2 being an absolute** makes §12(3) a plain scan of the whole generic core (`_webref/` + the `webref`
entry script) rather than a delta of base against head, so a violation authored anywhere in the tree is
caught wherever it came from. ⚠ **This paragraph described a `comm -13` set difference and "the block"
until #501 R68** — that was the dropped harness's `couplings`, which computed one for a *reported* line.
The wire computes no set difference at all: it scans, and it fails on the pin and on §2's K2 predicate.
The canonical account of what is absolute and what is review-only is **§12(3)**; this section does not
restate it.
→ `bash .claude/tools/webref-generic-core-trip-wire.sh`

⚠ **The unit suite scans no tree at all, and that is the end state, not an interim one.**
`test_spec_labels.py` imports the package and asserts over it; the two tests that once walked a tree were
deleted at the third design re-gate (below). The reasoning that kept the walk *inside* the package while it
existed — a repo-root lookup would have been the first in the generic tree, and would have put five
other-lane trip-wire artifacts inside a webref unit test's blast radius — is retired history now that the
walk is gone. What the wire owns is the tree; what the suite owns is the module's shape. **K2's**
entry-script assertion therefore lives in `.claude/tools/webref-generic-core-trip-wire.sh`, which is where
assertions outside the package belong. ⚠ **This sentence said "K2's *and K3's*" until #501 gate 4**, and the
wire never carried a K3 needle (`grep -niE 'cite.?audit|_catalog|K3' <the wire>` → no match). §12(2) and §15
state the truth: K3 is a time-limited fact, not an invariant, and **deliberately gets no mechanism**.

⚠ **The two ranges are nested, not a partition** — an earlier draft of this section said the trees were
*split* between the instruments, and §6's S7/S8 said "split by tree" and "the harness scans the rest".
Measured at the time, walking each tree under the rule both scanners used (`__pycache__` skipped,
undecodable files skipped) and with the same two regexes on both sides, the package range was a **strict
subset** of the generic-core range: the same files **plus** the `webref` entry script. (An earlier revision
ranged over all of `.claude/tools/`, pulling in five other-lane trip-wire artifacts, plus `.claude/skills/`;
redrawn at #501 R36.) Verified by planting a violation in each range: a package plant turned the suite red
**and** the tree-wide scan red, so the suite's two tree-scanning tests discriminated **nothing** the
tree-wide scan would miss; an entry-script plant was caught by the tree-wide scan alone. A `.claude/skills/`
plant is, by design, nobody's — the adapter is outside K2/K3.

⚠ **At the third design re-gate those two tests were DELETED from the generic suite** (Codex R55). Having
measured them as discriminating nothing, keeping them was two homes for one decision — and the copy in the
package additionally had to be *removed by Slice B* in order for B to add `cite_audit.py`, i.e. a passing
unit test that a downstream slice must delete to add functionality. `DESIGN.md:3-5,33-37` puts review/plan
workflow policy in the elidex adapter, not in a package meant to be extractable. `.claude/tools/webref-generic-core-trip-wire.sh` is now
the single home, and it is a **registered** wire (`scripts/trip-wires.sh`, `REQUIRED_WIRES`) rather than a
memo-owned script, so no later slice is owed a re-homing — §13 item 2 records that discharge. **What the
suite still adds is S7's third clause** (`webref_data` in `spec_labels.py`, which the wire does not check)
and the schedule it runs on — not range. That clause is not slice policy: a literal label map
has no business importing the upstream fetcher whichever slice is landing, so it moved to a class named for
the module's own shape. The containment stays: the scope is §2's generic core, decided at #501 R36.

⚠ **The entry script's docstring says what the tool does and nothing else, and that is deliberate.** Two
revisions of it named a review recipe — the first by path, the second by the skill's invocation name after
Codex R56 objected to the path. Only the path moved; the wording `DESIGN.md:33-37` is actually about stayed,
which Codex R60 then flagged. The third revision replaced the recipe with a note *explaining* the rule, and
Codex R65 flagged that too: a note about the host's review history is itself host wording. The rule lives in
`_webref/DESIGN.md`, the enforcement in the wire, and the history here. Nothing about it belongs in the
generic file.

**One-issue-one-way**: the label enumeration goes three sites → one, two of the three in this slice.

---

## §8 Line-count budget

**This section states no digit about the tree as it stands.** It states the command and the two things its output has to show. (The ⚠ note below carries three: two are historical — a line count this section used to be, and the authoring band the dropped harness used — and one is CLAUDE.md's fixed 1000-line threshold. None of the three moves with a commit, which is the property that matters.)

```sh
git diff origin/main...HEAD --numstat -- .claude/ && wc -l .claude/tools/_webref/*.py
```

1. **Both consumers lose lines on balance** (`cli.py`, `commands/coverage_map.py`) — the shape the
   de-duplication predicts: the enumeration leaves them and lands once in `spec_labels.py`.
   ⚠ **This outcome was FALSE at one head (#501 R70)**: `coverage_map.py` stood at +24/−20, a net *gain*
   of four, because a ⚠ comment this loop kept extending outgrew the enumeration it replaced. The criterion
   is load-bearing on the commentary budget, not only on the refactor — if it goes red again, the reading is
   "the prose here has outgrown its subject", not "restate the criterion".
2. **No file is near CLAUDE.md's 1000-line touch-time split threshold.**

⚠ A previous revision of this section carried a per-file Δ table and, before that, a `wc -l` figure for this
memo. Both went stale inside the same review loop that wrote them — the second time (Codex R65) by a
one-line docstring rewrap in the round immediately before. A digit in a memo is a hostage to the next commit
to the thing it counts, and this section's whole subject is quantities that move on every commit, so it
holds none.

⚠ **This section used to be 88 lines**, almost all of it the per-commit line-count history of the
re-derivation harness and the 700-800 *authoring band* this program used for it. That harness is not in this
PR (§15), so its budget is not A-i's to state. What is A-i's is the table above and the judgement below.

⚠ **The third design re-gate asked whether this memo should split: no.** CLAUDE.md's
touch-time split discipline is scoped to files **over 1000 lines** with a real cohesion seam. Splitting a
memo one review round from landing would also re-create, across two documents, the figure-with-two-homes
defect that same gate had just removed from the umbrella's slice table.


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
claim), no network dependency (`SPECS` is pinned; the catalog fall-through is B's), and no file near the
touch-time split threshold (§8), so **no defer slot** is owed at landing.
⚠ **There IS a scheduling gap, and this sentence claimed there was none until #501 R68.** Between A-i
landing and A-iii wiring the suites into CI, `test_spec_labels.py` is run by no required check (measured:
`mise.toml`'s tasks are cargo + shell trip-wires only; `.github/workflows/` and `scripts/trip-wires.sh`
invoke no python), so a regression that re-inlines either consumer can merge in that interval. It is **not**
a defer slot: the receiving site is named and dated — Slice A-iii, §15's table row — and per
`feedback_defer_cap_policy` an obligation with a named receiving slice is a hand-off, not a deferral. What
it is not is absent, which is what this paragraph used to imply. §12(1) states the same gap at the point a
reader checks the exit criterion. Owed *actions*
are a different category and are §13's — after this commit set: the umbrella's four scope-grant clauses plus
the `@lru_cache` row, the re-homing of `#11-preflight-css-module-labels`, and the re-derivations B and A-ii
owe.

---

## §12 Exit criterion

Every diff check names an explicit ref.

1. **Green**: `test_spec_labels.py` passes (**13 tests**, and the whole `_webref` suite is **25** — both re-measured after the third design re-gate removed the two duplicated slice-boundary tests). ⚠ **This is a check a reader runs, not a gate that runs itself** — measured at gate 4, nothing invokes it: `mise.toml`'s 17 tasks contain no python, `.github/workflows/` contains none, `scripts/trip-wires.sh` contains none, and `git grep test_spec_labels -- ':!docs/plans'` returns only the file. The three pre-existing `_webref` suites are in the same state, so this is structural rather than a regression — but it is an **asymmetry this PR created**: it judged CI wiring in scope for K2's wire and left the pins that carry K1/K4, the slice's actual thesis, to **Slice A-iii** (§15, the suite scheduler). By the wire's own quoted standard — `scripts/trip-wires.sh:105-112`, *"a check nobody re-runs is a transcript, not a gate"* — these 13 are a transcript until A-iii lands, and §15's table says so rather than implying otherwise;
   `git diff origin/main...HEAD -- crates/` **empty** (measured **0** lines);
   `git diff origin/main...HEAD -- .claude/skills/` is **exactly the one `preflight.py:47-49` comment** of
   §4.1 — not empty, and any other hunk under `.claude/skills/` fails this check. Measured at head: **17**
   diff lines, one file, one hunk, +3 / −3. ⚠ An earlier draft recorded **89** lines here (+20 / −30) and
   called the check "currently red, A-i unimplemented" — that was `b3a7d469`'s `preflight.py` change, which
   §4 drops; both statements are now stale and the check is **green**.
2. **K3**: at A-i's head,
   `git grep -hE 'cite.?audit|_catalog' -- .claude/tools/_webref/ .claude/tools/webref | wc -l` → **0**. A
   time-limited fact rather than an invariant (§15): Slice B's detector makes it false by design, so it is
   a diff-review item for this PR and gets no standing gate.
3. **K2 — stated as two claims, because only one of them is mechanised.** A-i discharges the two
   instances that existed at its base (`cli.py` and the `webref` entry script both named
   `.claude/skills/elidex-review/axes.md`; measured 1 each at `44cd165d`, 0 at HEAD).
   - **Closed part, absolute**: `bash .claude/tools/webref-generic-core-trip-wire.sh` → PASSED. It FAILS
     if either removed path returns, in a tracked *or* untracked file. Registered in
     `scripts/trip-wires.sh`'s `REQUIRED_WIRES`, so it runs on **every PR to `main`**, ungated by the CI
     path filter. ⚠ **And the PASSED is earned**: before it reads the real tree the wire **re-invokes
     itself over a fixture tree and asserts its own exit status** — one fixture per verdict it can reach
     (green; K2 firing under `skills`, under `tools`, on the path A-i removed, inside binary content and
     on a symlink's stored target; an empty scope, an unreadable file, an unsearchable directory and an
     entry that is none of file/symlink/directory all failing closed). The subject is
     the exit code, not an internal variable: an earlier shape inspected variables and three mutations
     survived it. `scripts/trip-wires.sh:105-112` records why hand-verification does not close this gap —
     a check nobody re-runs is a transcript, not a gate — and the general form of that rule is what
     rounds R70-R73 kept finding: **a green over something the wire did not read — or never enumerated —
     is a transcript too.**
     ⚠ **This row described a `PINNED` string plus a negative sibling until #501 R72.** That pin was
     measured to be strictly contained in §2's predicate and collapsed into it at R70 (§4.2); an emptied
     `PINNED` is no longer a mutation this wire can have.
   - **Open part, reviewed not gated**: "no *other* host path is named here" is not something this wire
     decides, and its header says so with the four failed attempts named. ⚠ An earlier revision of this
     row claimed it did — "**0** paths that resolve inside this repo are named anywhere" — which was the
     resolve-on-disk predicate Codex R59 had already falsified, left standing here for four more rounds.
     What covers the open part is `git diff origin/main...HEAD -- .claude/`: finite, and every line
     entering this tree passes review. ⚠ **The wire used to print a report-only seed "to make that reading
     cheaper"; that scan is gone at #501 R69** — it asserted nothing, so it was a print with no consumer,
     and keeping it invited exactly the reading this row exists to forbid.
   ⚠ **A segment containing WHITESPACE is outside the closed part** (`.claude/skills/team name/rule.md`).
   §2 admits it; no predicate over running text can delimit it without quoting rules, and this repo's rule
   is that such a population belongs to a seed rather than to an absolute — so it is review-covered, like
   the rest of the open half. `@`-prefixed and non-ASCII segments **are** covered: they were excluded by an
   `[A-Za-z0-9_.-]` class until #501 R74, which was a defect rather than a boundary.
   ⚠ Also outside the wire's predicate: a BARE top-level name with no separator (`"docs"`, `"crates"`,
   `"CLAUDE.md"` as standalone tokens), and two such instances pre-exist at this
   slice's base.
4. **K1/K4**: S3, S3b and S5 green — and S3 is green *as a perturbation*, not as an agreement (§6), so K1's
   `coverage_map` half is now actually pinned.

Checks 2 and 3 are scans for prose occurrences, not for file assignments. **Every check named here has been
run against a deliberately planted violation** and observed red **and non-zero** — the plant matrix is
package-internal / entry-script (`webref`, a file root) / untracked-under-`_webref/` / unplanted, plus
the shadowed-`python3` and three-cwd cases of item 3. A pin that cannot witness its own negation is not a
check; neither is one that witnesses it and then exits 0, which is what this commit set found.

---

## §13 Coordination

| Lane | Overlap | Ordering |
|---|---|---|
| **A-ii** | takes `preflight.py`'s copy **and** its failure semantics, plus `SPEC_LABEL_REVERSE`'s full census (§4.1). ⚠ The hand-offs below are **owed**, not routed — none has a receiving site in A-ii today, and that now includes `#11-preflight-css-module-labels` (checklist item 4). ⚠ **Plus one gate 4 added: a drift assertion between the two live maps.** Measured, `preflight.SPEC_LABEL_REVERSE` and the suite's `_VENDORED_CONSUMER_REVERSE` are byte-identical today (15 pairs each), S5 asserts one direction only, and **nothing asserts the converse** — that every label `SPECS` can emit resolves in the gate. `#11-preflight-css-module-labels` is scheduled to extend `SPEC_LABEL_REVERSE` with CSS modules, which diverges them **with both gates green**. The receiving site exists: `.claude/skills/elidex-plan-review/` has its own suite (`test_grep_pass.py`) and needs no repo-root lookup to import its neighbour — and A-i may not touch it (§12(1) allows one comment) | **A-i first** |
| **A-iii** | none | after A-ii |
| **Slice B** | takes every row marked B in §4.2, the fall-through, and the `@lru_cache` below | after A-ii |
| **Slice C** | shares `DESIGN.md`; A-i states its bullet verbatim, C owns the reported-class contract | after B |
| **PR-A0 (`elidex-wt-submittable`)** | touches the same `_webref` files, and its tree is still byte-identical there. ⚠ **Not a recovery location** — §4 withdrew that role (`git ls-remote origin domform-submittable-category` → 0 refs; author-machine only). Recovery is `refs/pull/501/head`, §14 | after A/B/C; it rebases |
| **PR #496 / #497** | **none** — both landed (`gh pr view 496 --json state` → MERGED 2026-08-02), and their `.claude/` files are disjoint from A-i's | none |
| **`scripts/trip-wires.sh` — the live contention** | ⚠ **A-i now edits a file two unlanded branches also edit.** `stale-claim-detector` adds `claim-provenance-trip-wire.sh` to the same `REQUIRED_WIRES` list; `layout-trip-wire-ci` edits the same file. `scripts/trip-wires.sh:70-84` makes this load-bearing — *"a wire that ARRIVES must be registered here in the same commit … disagreement either way is a FAIL"* — so whichever lands second takes a textual merge, not a decision | derive with `git branch \| xargs -I{} git diff --name-only origin/main...{} -- scripts/trip-wires.sh`, not over the `.claude/` trees |

→ `git worktree list` plus `git log --oneline origin/main..<branch> -- <file>` per contended file.

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

⚠ **The two rows below carry no line numbers, deliberately.** Those memos live on
`citation-hygiene-slice-memos` (#514), so a coordinate recorded here indexes a file this checkout does not
carry: it cannot be checked by a #501 reviewer and it rots on every edit to that branch. Earlier revisions
did carry them, and they were measured wrong twice over — A-ii's two anchors were blank on that branch and
B's had already moved before the carve, which is how a reviewer found them (#501 R12, R67). **The quoted
sentence is the locator.** Each cell reproduces the text it is about, which survives an edit anywhere above
it; nothing here asks a reader to trust a number this PR cannot re-derive.

| memo | sites | why it is false at the new base |
|---|---|---|
| **B** | its opening *"Slice A lands first and B rebases onto it"* and *"Branch: new, cut from Slice A's landed head"*; the carve description saying `cite_audit.py`, `spec_labels.py` and the rest moved "onto this branch **unchanged**"; its size table baselining the two `.py` files at line counts taken before the re-slice; its **17** line-anchored `<file>.py:<n>` edits — 11 into `cite_audit.py`, 6 into `spec_labels.py`, measured on that branch at the carve with `grep -coE '(cite_audit\|spec_labels)\.py:[0-9]'` — concentrated in its §4.1's nine under-report paths; its §4.1 round-trip clause (quoted in §13 below); and its §10 Q3 `@lru_cache` reasoning together with the docstring/`--help` disagreement it files as *pre-existing*. ⚠ **And two sites where A-i's own work is what is at risk**, located by content (the coordinates moved twice while this row was frozen — Codex R12 — so none are carried): **the `test_spec_labels.py` row of B's size table**, reading `\| — \| ~110 \| new (S1-S5) \|`; and **B's `**test_spec_labels.py** (new):` heading** followed by B's **own S1–S5**, which mean different things than A-i's S1–S8 (B's S1 is a round-trip over 948 catalog entries, B's own figure under that heading; A-i's S1 is `shortname_for` over `SPECS`). Its §6 pin table cites the same file under the same numbering | measured, `git cat-file -e origin/main:.claude/tools/_webref/commands/cite_audit.py` **fails**, as does the same test for `spec_labels.py`. B does not *repair* those files at its base; it **creates** them. For those two sites the consequence is sharper than staleness: an author working from B authors a fresh ~110-line file under a **colliding pin numbering** and drops A-i's S3, S3b, S4, S5, S6, S7, S8 and T-net — the only mechanical enforcement of K2 and K3 in the tree. Measured, A-i ships that file with **13 tests** (`grep -rn 'def test_' .claude/tools/_webref/test_spec_labels.py | wc -l` → 13, verified 2026-09-20), not `—`; ⚠ this row said **15** until Codex R62 — R58 re-derived the count in §4.2 and this Slice-B handoff, four hundred lines away, kept the pre-re-gate figure, so the two rows handed B incompatible baselines; its line count is §8's to state — an earlier revision carried a literal here that the review rounds outgrew (Codex R30). ✅ **Both sites discharged on #514 (Codex R14, before the R47–R50 carve moved B's memo there)**: B's heading now reads *A-i's file — B appends, does not create*, its pins are S9–S14 (continuing A-i's S1–S8), and the size-table row baselines on A-i's landed size |
| **A-ii** | its routing row handing A-i *"`spec_labels.py`, the three consumers, `DESIGN.md`"* marked **landed** — which double-books `preflight.py`, claimed by its own next row; and the two later sites premised on *"the asymmetry / the in-process reach is **created by A-i** moving the map"*, one of them classing the resulting deferral as **own**. Its own size baseline is stale at the new base for the same reason | A-i has not moved `preflight.py`'s map since draft 3, and §12(1) now forbids it. A-i's `preflight.py` touch is one comment and adds no `_webref` import, so the asymmetry — and the deferral that row classes as **own** — are created by **A-ii** |

⚠ **Two obligations whose receiving site was a FILE, not a memo — §13's forcing function does not reach
that shape.** Everything else in this section is discharged by B's or A-ii's own `/elidex-plan-review`, which
reads a memo. These two were not, and **both are now discharged** (below) rather than owed — so nothing here
is deferred and nothing needed a ledger slot. ⚠ **This paragraph said "Two **owed** obligations" and
justified not registering them by "A-i already carries two … and the per-PR deferral cap is three" until
#501 gate 4** — an accounting that contradicted §11's *"Zero own deferrals"* and §13 item 4's *"does not
count against A-i's per-PR carve cap"*, i.e. the same two slots counted as 2-of-3 here and 0-of-3 twice
elsewhere. The cap question does not arise: neither row is a deferral.

1. ✅ **DISCHARGED at the third design re-gate, not deferred.** This row read "S7's first clause must be
   RETIRED when B lands, not extended" — an obligation on a permanent tool-tree file that no plan-review
   round reads. The clause is **gone from the suite**: it was a second copy of the tree-wide scan the K2
   trip-wire makes, so deleting it removes the obligation instead of scheduling it. Nothing is owed to B
   here any more.
2. ✅ **DISCHARGED, not owed.** This row read "K2/K3's entry-script half is enforced from `docs/plans/`,
   and Slice C retires that … C must re-home the assertion". Both halves stopped being true in the same
   commit that dropped the harness: the enforcement is now
   `.claude/tools/webref-generic-core-trip-wire.sh`, a **registered permanent wire** in
   `scripts/trip-wires.sh`'s `REQUIRED_WIRES` that runs on every PR to `main`, and its scope is
   `_webref/` **plus** the `webref` entry script — the limb this row worried about. Nothing is owed to C;
   leaving the row standing would have sent C to re-home an assertion that already has a canonical home,
   which is the duplicated decision surface this program exists to remove.
   ⚠ What the wire does **not** decide is stated in its own header, not here: it asserts the removed-path
   pin and §2's K2 predicate, and reaches neither the policy half of `DESIGN.md`'s closing rule nor the
   two classes no grep predicate can see (bare top-level names, interpolation).

**Owed to Slice B — three assertions that pin the round-trip defect GREEN.** `test_spec_labels.py`'s S6
test — `test_spec_label_covers_pinned_and_non_pinned_shortnames`, named rather than numbered because a line
number in the shipped file goes stale on the next edit to it, and this one already had (#501 R67) —
carries `_spec_label("css-text-3") == "CSS TEXT 3"`,
`_spec_label("cssom-view-1") == "CSSOM VIEW 1"` and `shortname_for("CSS TEXT 3") is None`. They are correct
for A-i — §4.2's ⚠ explains why the last resort stays `origin/main`'s verbatim — and they are exactly what
B's §4.1 changes: under *"`label_for` must return a label that round-trips, or the shortname"* the first two
become **false**, so **B must delete them**, and nothing records that today. ⚠ The third is a different case
and is stated separately rather than folded in: `"CSS TEXT 3"` is neither a catalog title nor a shortname, so
under B's reverse index it plausibly still returns `None` — it does not become false, it becomes
**vestigial**, because the output it was pinning as unreadable is no longer the output. B disposes of it
either way; A-i does not assume which.

**Owed to Slice B — `partition` and its `_catalog`.** The block that exercised the catalog fall-through
travels with B's memo on `citation-hygiene-slice-memos`; the API it calls, `spec_labels._catalog()`, is B's
to introduce. Nothing is owed to B from this PR for it.

⚠ **The 19 lines that stood here narrated the re-derivation harness's own defect history** — an
`AttributeError` since `6be73a82`, an eight-entry degraded roster, which block's exit status propagated.
That harness is not in this PR (§15), so none of it is A-i's to state, and while it was here the passage
also contradicted §8 about whether `all` exited 0 or 1.

⚠ **Reproduce a remote-less checkout in a throwaway CLONE, never in a worktree.** `git remote remove origin`
writes to the **shared** `$GIT_COMMON_DIR/config`, and `git update-ref -d refs/remotes/origin/main` deletes a
**shared** ref — a worktree isolates neither. Run inside a `git worktree add --detach` of the real repo, that
recipe strips `origin` from **every worktree of that repo at once** and leaves every command that resolves
`origin/main` — §12's diff checks, §15's table, this memo's own re-derivations — failing everywhere until
someone re-adds the remote. ⚠ **This sentence said "every `$MAIN` block above" until #501 gate 4**; those
blocks were the dropped harness's and left with it (`grep -c '\$MAIN'` over both memos → 1, this line). Use
`git clone --local <repo> <tmp> && cd <tmp> && git remote remove origin`, which is isolated and, on this
repo's 36 MB object store, effectively free.

✅ **K2/K3 REDRAWN at #501 R36** (the converge of this PR is the plan-review altitude for this memo; Codex
R36 declined to hand an acknowledged mis-drawn exit criterion forward). A Trigger-B root-cause pass on the
Step 4.5 fix had found that §2 bound *generic core* to `.claude/tools/`, while `DESIGN.md` defines the generic
core by responsibility and lists modules all under `_webref/`; its only occurrences of `.claude/tools/` are
invocation examples. **Measured, the widening bought zero evidence** — `git grep -oE '<PATHRE>' origin/main --
.claude/tools/` and the same restricted to `.claude/tools/_webref/ .claude/tools/webref` both return **2**, the
identical two sites — while importing five other-lane artifacts (`layout-box-reader-allowlist.tsv`,
`layout-box-reader-trip-wire.sh`, and three `*-trip-wire.sh`) into A-i's §12(3) exit criterion, so a Layout-lane
edit with no webref content could fail it. K3 was mis-drawn a second way: its headline said *the generic core*
names no Slice-B artifact, but its body ranged over `.claude/skills/`, the **adapter**. The correction — K2/K3
bound to `_webref/` plus the `.claude/tools/webref` entry script, one enforcement point — costs no evidence
and removes all cross-lane coupling. ⚠ **A-ii and A-iii cite the dropped harness's block by name**, so the
memo PR that carries them (#514) has to be swept for it before it lands; the umbrella records that
obligation. No CI wiring is involved here, so `#11-layoutbox-trip-wire-not-in-ci` is untouched.

**Frozen literals.** S5's 15 `SPEC_LABEL_REVERSE` pairs **and** S3b's vendored `COMMON_SHORTNAMES` blurb text
are both `origin/main` snapshots taken at vendoring time and refreshed never — which is what makes them pins
rather than mirrors (K4). ✅ **A-ii must not refresh either — routed (#501 R42)**: A-ii §12(1) requires both
literals byte-identical to A-i's landing, measured by `git diff` over the two literals, not by prose.

**Known hole → B (T10) + a registered sweep slot (#501 R42).** Not an A-i defect: K4 asserts identity with `origin/main`, never completeness,
and A-i must not "fix" the map. The pinned label for `webidl` is **`Web IDL`, unprefixed**, though webref
reports `organization=WHATWG` for it and `xhr` is pinned `WHATWG XHR` — so under the frozen map a
`WHATWG`-prefixed spelling returns `None`. ⚠ **The failing spelling is `WHATWG WebIDL`, no space** — and
the receiving pins are B's **T10** (the spelling is *reported* under `UNKNOWN-SPEC`, never dropped) and the
ledger slot `#11-webidl-label-spelling-sweep` (the five sites below re-spelled to the pinned `Web IDL`, owner =
the cite-sweep program, trigger = B's `cite-audit` listing them on `main`). Measured,
`git grep -lI 'WHATWG Web IDL' -- . ':!docs/plans/2026-07-citation-hygiene*' | wc -l` → **0** files;
`git grep -lI 'WHATWG WebIDL' … | wc -l` → **5**, all in `crates/script/elidex-js/`: `src/vm/error.rs:33`,
`src/vm/host/fetch/mod.rs:258`, `src/vm/host/request_response/mod.rs:188`,
`src/vm/tests/tests_events_misc.rs:400`, `src/vm/tests/tests_worker.rs:832`. The `Web ?IDL` regex matches
both spellings, which is how the count 5 is right while the spaced spelling it was attached to would key a
remedy closing **0** of them. Found by Axis 4.

⚠ **And "found by an axis" makes it a SEED, not an inventory — the slot was named after the one instance
the axis returned.** Measured at gate 4, the same class (a **pinned** spec cited under a spelling
`shortname_for` returns `None` for) is live for two more pinned specs:

```sh
for p in 'WHATWG IDL §' 'WHATWG WebIDL' 'WHATWG Selectors §' 'Selectors Level 4 §' \
         'W3C Geometry §' 'Geometry Interfaces §'; do
  printf '%-22s %s\n' "$p" \
    "$(git grep -cI "$p" -- . ':!docs/plans/2026-07-citation-hygiene*' | awk -F: '{s+=$NF}END{print s+0}')"
done
git grep -cI 'Selectors L4 §' … ; git grep -cI 'CSS Selectors L4 §' …   # 8 and 6 → 2 non-resolving
```

→ webidl **6**, selectors-4 **6+2**, geometry-1 **4**: the slot's own five are under a third of the class,
and `CSS Selectors L4` / `Geometry Interfaces L1` / `Web IDL` are the only spellings that resolve. A sweep
keyed to the slot's *name* repairs the webidl sites and leaves the other two specs authoritative under a
slot nobody re-opens. ✅ **The slot is therefore re-scoped by CLASS, not by instance** — *every pinned
spec's non-resolving spellings, population derived by the command above* — same owner (cite-sweep program),
same trigger (B's `cite-audit` on `main`).

**Second known hole → B (S15) (#501 R42).** Same class, recorded the same way and for the same reason: pre-existing,
byte-identical to `origin/main`, so **K4 forbids A-i touching the map**. `webcrypto` is pinned as the
**series** label `Web Cryptography API`, unlevelled. Measured, `.claude/tools/webref specs Cryptography`
resolves to **`webcrypto-2  Web Cryptography API Level 2`** (the other two hits are
`webcrypto-modern-algos` and `webcrypto-secure-curves`; there is no bare `webcrypto` spec). So a memo row
written as `Web Cryptography API §N` verifies against **L2's numbering under a level-free label**, and will
silently re-target when the series advances to L3 — the same defect class as B's `CSSOM`→`cssom-1` /
`Selectors`→`selectors-4` re-pointing, arriving through the pinned map instead of the catalog. A-i ships the
map unchanged; **B re-points the pinned `webcrypto` entry to `webcrypto-2`** in the same edit as its other
level re-pointings (umbrella "Cross-lane coordination"; B §6 **S15** pins it).

**Owed to Slice B — B *adds* it.** `sources/webref_data.py`'s `@lru_cache(maxsize=None)` on `try_fetch_data`
(**+9 / −0**) rides on `b3a7d469` and leaves A-i's lineage with it. A real optimization by its own
docstring — *"60 lookups were 60 identical HTTP GETs at ~46 ms, 18.6s of a 47.4s run"* — sitting on
`heading`'s fetch path, so it is **routed, not dropped**: B owns the catalog fall-through and is the
many-lookups-per-spec consumer. ⚠ **B's memo does not support this routing and must not be cited as if it
did**, and the umbrella does not carry it either (`grep -F 'lru_cache' …-umbrella.md | wc -l` → **0**, so the
matching row is owed). B's §10 Q3 reasons from the decorator being *already present*, which at B's new
base it is not, and a later row files the resulting docstring/`--help` disagreement as a "**pre-existing** defect not
owned by this PR", a classification that inverts once B is the commit that adds it. Both fold into §13.1's
owed re-derivation.

**Landing checklist**

1. Update `project_citation-hygiene-program.md` and `active-lane-detail.md` with A-i's outcome, collapsing to
   the program memo with pointers, frontmatter included. ⚠ **The draft-number disagreement this item used to
   flag is now a no-op** — verified against the live files, `project_citation-hygiene-program.md` (frontmatter
   and its two draft-number rows) and `active-lane-detail.md` — the two files that carry one — both say
   **draft 5**; `MEMORY.md` carries no draft number for this program (measured, 0 hits). The action stands; the
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
   to do*, which is what (b)-(e) state. (a) **was** a status register — the umbrella carried "901 lines …
   Whichever slice next touches it splits it first" about the re-derivation harness. ✅ **Moot twice over**:
   A-i split that file, and the harness then left this PR entirely (§15), so the register is gone from the
   umbrella — measured, the phrase is absent from it at this head. (f) the *review cost tracks
   blast radius* bullet reasoned from "A-i has one invariant", which **the same commit-set falsifies**
   (`grep -cE '^- \*\*K[0-9]'` → 4, `grep -cE '^\| K[0-9] × K[0-9]'` → 5) and which A-i's own §9 no longer
   claims; it now reasons from blast radius, as it always should have. Neither changes A-i's scope by a line.
   Also corrected there: the umbrella's claim that `ee2d0dc0` "no longer exists (`git cat-file -e` fails)" —
   measured, `git cat-file -e` returns **0** and the blob still reads 1196 lines, so the ground is
   **unreachability** (`git branch -a --contains ee2d0dc0` → empty), not non-existence. The corrected
   sentence was subsequently dropped along with the passage that held it; `ee2d0dc0` appears nowhere in the
   umbrella at this head. ⚠ **The conclusion
   this round drew from that — prefer `<commit-that-deleted-it>^` — was itself falsified at R51** and is
   restated correctly in §14: the `^` spelling confers no durability, and under squash merge the branch is
   not a permanent ref at all.
3. **Harness edits.** ✅ **Moot.** Every item under this heading was an edit to
   `docs/plans/2026-07-citation-hygiene-A-rederive*.sh` — a path-filter widening, a roster that named two
   things a reader could not run, a block moved between parts. Those files are not in this PR (§15). K2's
   enforcement, the one obligation of the set that outlives them, is now
   `.claude/tools/webref-generic-core-trip-wire.sh`, registered in `scripts/trip-wires.sh`.

4. Register nothing **new** — A-i introduces no slots. The in-process collapse of `preflight.verify_citation`
   is **A-ii's in-slice work** (umbrella constraint revised at #501 R36; no slot). ⚠ **`#11-preflight-css-module-labels` is a different case and an earlier draft got
   it wrong**: this memo asserted it was A-ii's, but measured, A-ii's memo mentions it **once**, in a
   *reader-census row* about `SPEC_LABEL_REVERSE`'s four plan-memo readers — not an obligation. A-ii's §11 lists **one** own slot and its landing checklist registers only that one. So the
   slot is **owed, not routed** — the same label §13's other two hand-offs carry, and for the same reason:
   at the time of that reading it was absent from the SoT (`grep -F '#11-preflight-css-module-labels'
   memory/project_open-defer-slots.md | wc -l` → **0** then, **1** now — see the ✅ below) and owned by
   nobody. It survived only in two **landed** memos —
   `2026-07-terminal-z-c3a-seam-and-audit-plan.md:655` (row 8, the authoritative hand-off) and
   `2026-07-terminal-z-c3a-impl-plan.md:538` — registered with owner **PM** and trigger *before the next
   plan-memo citing a CSS module, C-3b at the latest*. ⚠ **And that trigger cannot be relied on to force a
   look**: an earlier draft said "the C-3b lane is live", which memory contradicts —
   `active-lane-detail.md` records *"C-3b–e は **parallel-safe でない**ため lane 対象外"* and
   `project_layoutbox-trip-wire-in-ci-next.md` *"C-3b–e stays ruled out (not parallel-safe)"*. C-3b is
   **not scheduled**, so the deadline is unbounded in practice and a real re-homing is what the slot needs.
   ✅ **Re-homed at this landing (Codex R14)**: the slot is registered in `project_open-defer-slots.md` under
   this program, **owner = Slice B** — B's §4.1.7/§4.1.8 catalog widening *is* the mechanism, and B §6 **P-CSS**
   (`test_preflight.py`: a memo citing `CSS Text 3 §4.1.3` passes the gate) is its **one** closing pin (T3/T3b
   pin the library side and cannot witness the gate) — with **A-ii as prerequisite** (A-ii routes
   `preflight.py` through `shortname_for`, which is what puts the catalog in the gate's path). Inherited from
   C-3a row 8 and *homed*, not newly introduced, so it does not count against A-i's per-PR carve cap.

---

## §14 Provenance

Carved from `2026-07-citation-hygiene-A-enforcement-plumbing.md` (nine rounds). The merged memo is
recoverable — 1196 lines, round-by-round index intact — but only through a ref that keeps `707b69cc`
reachable, which after this PR lands is neither this branch nor `main`:

```sh
git fetch origin refs/pull/501/head
git show 707b69cc^:docs/plans/2026-07-citation-hygiene-A-enforcement-plumbing.md
```

⚠ **This is the single site of the rule that pointer has now failed three times to satisfy.** A SHA is
durable exactly while some *permanent* ref keeps it reachable; no spelling of the SHA confers that. Revision
1 named `ee2d0dc0`, orphaned by a rebase four commits later — `git branch -a --contains ee2d0dc0` prints
nothing. (The object is dangling, not absent: `git cat-file -e` returns 0 and the blob still reads 1196
lines. Unreachability is the operative fact, since it is what a fresh clone cannot resolve.) Revision 2
named `707b69cc^` on the ground that `<commit-that-deleted-it>^` "survives rewriting" — true of a rebase
*within* the branch, false of what actually happens to the branch: CLAUDE.md lands PRs by **squash merge**,
so no commit of this branch ever enters `main`, and the branch is deleted afterwards. What survives both is
GitHub's `refs/pull/<n>/head`. Measured against a PR that has already been through that path — #508,
squash-merged, branch deleted (`git ls-remote origin layout-inline-seam3` → 0 refs): `refs/pull/508/head`
still resolves to `803d35a0`, `git merge-base --is-ancestor 803d35a0 origin/main` **rejects** it, and both it
and its parent `42936f4f` still answer `gh api repos/send/elidex/commits/<sha>` — while an all-zero SHA
answers 422, so the probe discriminates rather than always succeeding.

## §15 Re-derivation

Every claim this memo makes about the tree it ships is re-derivable by a command, and every command is one
this repo already runs. ⚠ **The sentence said "every claim" until #501 gate 4**, and the paragraph below
gives line counts for four files that are in no tracked tree — they are derivable only from `b746f400^`,
reachable after the squash merge through `refs/pull/501/head` (§14). A line count of an absent file is a
measurement like any other; it is kept here, with its commit named, because the drop decision is the
subject.

| Claim | Command | Runs in |
|---|---|---|
| The map has ONE source; both consumers derive from it | `cd .claude/tools && python3 -m unittest _webref.test_spec_labels` | Slice A-iii wires the suites into CI |
| §2's K2 predicate holds over the generic core, and every file in it was readable (K2, **the closed part**) | `bash .claude/tools/webref-generic-core-trip-wire.sh` | `trip-wires`, **every PR to `main`**, ungated by the path filter |
| A *new* host path named in the generic core, in a shape §2's predicate does not cover (K2, **the open part**) | `git diff origin/main...HEAD -- .claude/` — read it | code review |
| The §3 gate resolves both labels this slice cites (§0.5) | `python3 .claude/skills/elidex-plan-review/preflight.py docs/plans/2026-07-citation-hygiene-Ai-spec-label-map.md` | the gate every lane runs before its plan-review |
| A §-number matches its title | `.claude/tools/webref heading --exact <spec> <section>` | per CLAUDE.md § "Spec citation" |
| What this slice changed, and where | `git diff --name-only origin/main...HEAD` — **read its output; this row states no list.** ⚠ Three revisions of it did, and each undercounted: `.claude/` only (missing `scripts/`), then "three trees" (missing the `webref` entry script and the `preflight.py` comment, both of which §0.1 and §4.2 do account for). A hand-kept enumeration of a diff is a measurement, and §8's rule applies to it | — |

⚠ **This section used to be 55 lines describing a 1463-line bespoke harness** under
`docs/plans/2026-07-citation-hygiene-A-rederive*.sh`, whose blocks re-derived the figures above. It was
**dropped from this PR** rather than fixed. A `/code-review max` pass over it returned 15 findings plus ~20
more past the display cap, and the operative sentence was that **`rederive all` exited 0 and `selfcheck`
printed GREEN while four of the gates this memo named as its exit criteria were false-GREEN** — among them
a `RETURNS` predicate that blessed `exit`, so a rostered block could kill the shell mid-roster and the
roll-up line simply never printed; a host-vocabulary regex using `\b`, which git's ERE takes literally, so it
caught 2 of the 5 classes it enumerated; and a coverage check that silently dropped any §0.5 row its own
regex missed — the very defect it had been added to fix, one round earlier.

The decision is not "that harness had bugs". It is that **this program had already ruled against the
mechanism**: `memory/project_citation-hygiene-program.md` records the rule ratified 2026-08-23 —
*plan memo は測定値も自己測定装置も持たない。測定が要るものは test / CI* — in English, *a plan memo holds no measurements and no self-measuring apparatus; what needs measuring goes in a test or
CI* — scoped "A-ii onwards" only because this harness predated it. Sibling PR #505 was closed unmerged for
the same fixed point, memo and harness generating review surfaces for each other. Measured on the dropped
files: 662 of 1463 lines were comment, and 61 of those narrated individual review rounds. The table above is
what the rule asks for instead.

⚠ **K3** — "the generic core names no Slice-B artifact" — deliberately gets no mechanism. It is not an
invariant but a **time-limited fact**: it stops being true the day Slice B lands its detector, by design.
`git grep -hE 'cite.?audit|_catalog' -- .claude/tools/_webref/ .claude/tools/webref | wc -l` returns 0 at this head,
which is a diff-review item for this PR, not something to gate in perpetuity.