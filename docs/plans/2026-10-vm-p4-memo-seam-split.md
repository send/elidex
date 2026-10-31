# VM P4 umbrella memo — seam split plan (`#11-vm-p4-memo-section-seam-split`)

**Status**: plan, for `/elidex-plan-review`. **Subject**: `docs/plans/2026-07-vm-p4-es-language-completeness.md`
at `39cc365a` (3093 lines). **Rule**: CLAUDE.md *1000-line debt = touch-time split* — a cohesion
judgment, executed as a standalone, content-free commit before #506 next edits the memo's content.
Every line number below is a line of that blob (`git show 39cc365a:<memo>`), never of a working tree
(the umbrella memo is unchanged over `39cc365a..dcf43196`); a line number into any **other** file names
its revision. **Prerequisite S0 (§1.3) has landed**: PR #562, squash `e44ffce6` (2026-10-31), so the
split no longer waits on a tools change.

This memo is deliberately **not linked** from the umbrella: a link would put it in the checker's
population, and every id it spells would become a naming site of the umbrella's census.

## §0. A premise that does not hold, and what it forces

The slot's ledger note and the checker plan (`docs/plans/2026-08-plan-memo-umbrella-checker.md`
"Split decision") say #510 satisfied the precondition "the checker counts sibling memos' tables". Half
of that is true, measured in `.claude/tools/plan_memo_population.py`:

- **True — census is population-wide.** `Population.__init__` walks `Memo.linked_files()`
  transitively (BFS, first-link order, `seen` at schedule time). A sibling is a **relative `.md` link
  destination** (`plan_memo_sibling.sibling_path`: fragment/query stripped, percent-decoded, relative
  on every platform, `.md` suffix) from **any** block, table cells included — inline and reference
  links alike. A backticked path or a bare file name is **not** a link and adds nothing. `_declare`,
  `data_rows` and `declaring_rows` then read schema rows from **every** memo.
- **False (at `39cc365a`) — schema presence was entry-memo-only.** `plan_memo_population.py:103-108`
  at `39cc365a` built `matched` from `self.main.tables`: each of the **four** `SCHEMAS` (`citation`
  §0.5, `stub` §2.2, `slice` §5, `slot` §8 — not only §5/§8) had to have a table **in the memo the
  checker is invoked on**, else `FATAL [SCHEMA] … no table matched schema` and rc 2. The checker plan
  stated this itself (`:278` "schema presence is required of the main memo only"; since #562 that line
  carries "superseded 2026-10-28" and points to the Correction at `:380`).

**Measured** (scratch copy, split as in §1, checker at `39cc365a`): rc 2,
`no table matched schema 'slice'` and `… 'slot'`. So the seam layout below needed **one checker
change first** (§1.3) or a different layout (§1.4, rejected). That change is S0, landed as #562
(`e44ffce6`); with it the same scratch split is rc 0 (§2).

Two smaller premise corrections: the `Trigger` column is **§8's** (`Slot | Why deferred | Trigger |
Re-eval`) and names §5 rows; the §5→§8 direction is §5's **`Slot`** column (§4 below). And the
"~19 cross-links" figure does not reproduce under any class I can define; §3 gives the measured classes.

## §1. Seams

### §1.1 Layout (ranges at `39cc365a`; sizes = range + carve head of ~6-8 lines)

| File (`docs/plans/2026-07-vm-p4-…`) | Takes | Lines |
|---|---|---|
| `es-language-completeness.md` (entry, stays) | head + §0.5-§3 (1-697), §6 stub (2315-2351), §7 (2352-2397), §§10-18 stubs (3077-3093), + 4 new stubs | 797 + ~30 |
| `design-invariants.md` (new) | §4 I-1 … I-9 (698-1006) | 309 |
| `slice-plan.md` (new) | §5 conventions + per-row evidence + table + ordering rationale + per-row notes (1007-2123) | 1117 |
| `touch-time-split-record.md` (new) | §5's 1000-line touch-time check through the `Intl`/`Atomics` note (2124-2314) | 191 |
| `slot-ledger.md` (new) | §8 whole, its table included (2398-2786) | 389 |
| `open-decisions.md` (new) | §9 decisions 1-17 (2787-3076) | 290 |

The ranges partition 1-3093 together with the kept ranges (checked by §2.1's script: 2296 moved +
797 kept). Section numbers do **not** change: §4/§5/§8/§9 keep their numbers in their new files, as
§6 and §§10-18 did.

### §1.2 Why these seams, and the one file over 1000

- **§4** is a closed list of named invariants (I-1 … I-9) cited by name from every other section; it
  reads nothing in §5's table. **§8** carries its own table and the rules over it (Forward rule,
  reconciliation, cap accounting) — one unit. **§9** is the decision log. Each is its own subject.
- **§5's tail (2124-2314)** is a different subject from the table: a file-size discipline record
  (`vm/dispatch.rs` reversal, the watch list, this memo's own first carve). It is the passage the
  three existing carve heads point at ("the seam derivation … in the umbrella's §5").
- **`slice-plan.md` at 1117 lines is exempt as a cohesive unit**, and this re-affirms the memo's own
  verdict at 2291 ("Its conventions passage and its table are one unit"), now with the reason measured
  over the current text: the conventions are "stated once for the whole table, and nowhere else"
  (1416); the per-row evidence bullets carry **ordering specification** the Deps cells defer to
  ("Shared prerequisite of Slices 2 and 3 (an ordering fact, so it is stated once here …", 1766); the
  ordering rationale (1987) reads the table. Each internal candidate cut — 1568|1569 (the evidence
  block opens "Evidence found by **that** round", whose antecedent is 1557-1567), 1919|1920 and
  1985|1986 (table boundaries) — separates rows from the specification they defer to. 204 of the 416
  `above`/`below` deixes are in this range. A flat-table exemption is **not** claimed: the table is
  66 lines; the bulk is prose.
- **Parent pointer form** — the precedent (`## §6. … → **carved out**` at 2315; `## §§10-15 …` at
  3077): one stub per moved section, **in original section order**, heading `## §N. <title> → **carved
  out**`, body "Moved **verbatim** to `<path>` …; sub-names keep their numbers there". §5's stub names
  both files. Stub and carve-head text **name no declared id** (no row id, no `#11-` slug, no
  `I-n`), so they add no naming site (§2).
- **Each stub carries one inline link per file it names** (`[§N](2026-07-vm-p4-<file>.md)`, beside
  the backticked path; §5's stub names two files, so five links in four stubs). This departs from the precedent stubs, whose path is backticked only and whose
  population edge comes from the relinked `§6…` / `§1x` references. Here that is not enough: the
  relinked references give `design-invariants.md` one edge and `touch-time-split-record.md` one (2318,
  itself inside the §6 stub), and those two files hold no schema table, so losing that edge would drop
  them from the population at rc 0 (§1.3, §7). The stub link makes every child's membership
  structural. It is also the form §2's simulation measured.

### §1.3 Prerequisite S0 — schema presence over the population (**landed**: #562, `e44ffce6`, 2026-10-31)

**The landed contract** (PM judgment 52, rule C; 61; the cost accepted in 70). `matched` is built from
every memo of the population (`plan_memo_population.py:108` at `e44ffce6`, under a comment stating the
rule). A schema is present iff **≥ 1** memo of the population holds a table matching it. Zero memos is
`FATAL [SCHEMA] … no table matched schema`, rc 2, reported at the entry memo. **≥ 2** memos holding
the same schema is allowed: multi-memo census is supported. Duplicate rows across memos are guarded
by the existing id collision (`declared twice`, rc 2) for every schema with an id column. A duplicated
`stub` table (no id column) is not detected (PM 61). The withdrawn alternative was one home per
schema (exactly one memo).

- Controls: the five `(presence)` cases in `plan_memo_selftest_cases_sibling.py`. A schema in no memo
  gives rc 2. Only in a carved sibling gives rc 0. In two memos gives rc 0. A sibling citation table
  with ids of its own gives rc 0 (census). The same table re-declaring the entry's id gives rc 2.
- Mutants (`plan_memo_selftest_mutants.py`): `population: a schema's presence is read over the
  POPULATION (revert to the entry memo only)`, killed by the sibling-only control. Also the existing
  `gate: an unmatched schema is a schema miss`, which now also names the population-wide POSITIVE
  control. #562 reports 596 mutants, 0 survived, at its head `670aeda2`.
- #506's 4-memo population: the checker output at `dcf43196` is byte-identical to the pre-S0
  baseline (rc 0).

**What rule C costs this split.** #562's Known remainder (ii): a table whose header drifts from its
schema silently becomes a non-schema table. Under rule C, if another memo carries the same schema,
presence is still met, and the drifted table's rows leave the census at rc 0. The earlier draft of
this section said "a table that drifts … still leaves its schema matched nowhere → rc 2". Under rule C
that is **false in general**. It holds for this population only because, after the split, every schema
has exactly one table in exactly one memo:

| Schema | Memo after the split | Tables (rows) |
|---|---|---|
| `citation` | entry (§0.5) | 1 (29) |
| `stub` | entry (§2.2) | 1 (55) |
| `slice` | `slice-plan.md` (§5) | 1 (64) |
| `slot` | `slot-ledger.md` (§8) | 1 (52) |

No other memo of the population holds one. That covers the three existing carve-outs and the three
children without a schema table (`design-invariants.md`, `touch-time-split-record.md`,
`open-decisions.md`). So a header drift of any one of the four tables still leaves its schema matched
nowhere → rc 2. **Measured** on the scratch split of §2 with the checker at `dcf43196`: renaming one
header column of each table in turn gave rc 2 and `no table matched schema '<name>'`, for all four.
The silent case of rule C opens only if a later edit puts a second table of one schema in another
memo. The split does not do that. After the split, the check is this, run from the repo root at the
split head:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 - <<'EOF'
import collections, pathlib, sys
sys.path.insert(0, '.claude/tools')
from plan_memo_population import Population
p = Population(pathlib.Path('docs/plans/2026-07-vm-p4-es-language-completeness.md'))
homes = collections.defaultdict(list)
for m in p.memos:
    for t in m.tables:
        if t.schema is not None:
            homes[t.schema.name].append((m.path.name, len(t.rows)))
for k, v in sorted(homes.items()): print(k, v)
EOF
```

Expected: one entry per schema, matching the table above. At `dcf43196` (before the split) it prints
all four at the entry. A schema with two entries means its drift is no longer rc 2, and the split PR
must say so.

**A child the entry forgets to link.** `slice-plan.md` and `slot-ledger.md` hold the only `slice` and
`slot` tables, so losing their edge is still rc 2. The three children without a schema table are not
guarded by presence. Losing the edge to one of them drops it from the population at rc 0, and only the
naming-site counts move. That is caught by §2's count comparison, not by the exit status. It is why
each stub carries its link (§1.2).

### §1.4 Rejected: keep all four tables in the entry memo (no S0)

Entry = §0.5-§3 + §5's table block + §7 + §8's table block (~940 lines); conventions/evidence in
children. It needs no tools change, but it **reverses** the 2291 verdict by separating the table from
the specification its cells defer to, and splits §8 from its own rules. Cost-driven, so rejected.
(Moot since S0 landed; kept as the record of why the split waited on a tools change.)

## §2. Invariants, measured before and after

Baseline (saved full output, 868 lines; `PYTHONDONTWRITEBYTECODE=1 python3
.claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`):
rc **0**; `[UMBRELLA-MARK] 48` (`slice=33, slot=15`); `[CENSUS] 53`; `[KIND-UNDETERMINED] 5`;
`[UMBRELLA-CELL] 37`; footer `51 seed(s) and 715 reported naming site(s)`; population = the three
existing carve-outs.

**Predicted after** (measured on a scratch split of exactly §1.1's ranges, 4-line heads, linked stubs
(§1.2), parent `§N` relinked per §3; first with S0 applied by hand, re-measured 2026-10-31 with the
landed checker at `dcf43196`; ⚠ the simulation linked 2318's `§5` to `slice-plan.md`, not to
`touch-time-split-record.md` as §3 specifies, so on that one link it is not the planned split — the
after-run on the split head is the measurement): rc 0, 868 lines, every count above **identical**; **no table
row changes file attribution in the census** — the 64 §5 rows are read from `slice-plan.md`, the 52
§8 rows from `slot-ledger.md`, the §0.5/§2.2 rows from the entry. The full-output diff, after
mapping each `file:N` back to its `39cc365a` line (child: `a + N − h − 1`, `h` = head length; parent:
the stub offsets) and sorting, is **15 output lines on 12 source lines**, two classes only:

1. excerpt text of **11** entry lines whose bare `§N` became a link (157, 439, 443, 454-456, 458,
   475, 585, 647, 654);
2. the in-message locator of the one `row <no id> at :N (Function/eval)` ORDER-PROSE? finding (1985),
   which is child-relative (`:983` at a 4-line head).

Unsorted, the naming-site listing also reorders (population is BFS order). Anything else — a count,
an `[UMBRELLA-MARK?]` row, a seed — is a defect of the split. The baseline is the same command run
at `39cc365a` and saved before the split; after the split (S0 landed), at the split head:

```sh
M=docs/plans/2026-07-vm-p4-es-language-completeness.md
PYTHONDONTWRITEBYTECODE=1 python3 .claude/tools/plan-memo-umbrella-check.py $M > after.txt; echo rc=$?
grep -E '^\[(CENSUS|UMBRELLA-MARK|UMBRELLA-CELL|KIND-UNDETERMINED)\]|seed\(s\)' after.txt
```

### §2.1 Content-free proof (the split commit)

Byte-exact: each child's body is its range; the entry, with the new links undone, is the kept lines
plus stub-only insertions. Negative controls run: one edited char in the entry → 1 non-stub diff;
one in a child → `AssertionError`.

```python
import difflib, re, subprocess, sys
D, X = 'docs/plans', '2026-07-vm-p4-'; U = X + 'es-language-completeness.md'
M = [('design-invariants', 698, 1006), ('slice-plan', 1007, 2123), ('touch-time-split-record', 2124, 2314),
     ('slot-ledger', 2398, 2786), ('open-decisions', 2787, 3076)]
old = subprocess.run(['git', 'show', '39cc365a:' + D + '/' + U], capture_output=True, text=True,
                     check=True).stdout.split('\n')[:-1]
moved = set()
for name, a, b in M:
    body = open(f'{D}/{X}{name}.md').read().split('\n')[:-1]
    assert body[-(b - a + 1):] == old[a - 1:b], name
    moved |= set(range(a, b + 1))
kept = [l for i, l in enumerate(old, 1) if i not in moved]
link = re.compile(r'\[(§[4589])\]\(' + X + '(?:' + '|'.join(n for n, _, _ in M) + r')\.md\)')
par = [link.sub(r'\1', l) for l in open(f'{D}/{U}').read().split('\n')[:-1]]
bad = [o for o in difflib.SequenceMatcher(None, kept, par, autojunk=False).get_opcodes() if o[0] != 'equal'
       and (o[0] != 'insert' or not any('→ **carved out**' in l for l in par[o[3]:o[4]]))]
print('moved', len(moved), 'kept', len(kept), 'non-stub diffs', len(bad)); sys.exit(1 if bad else 0)
```

Expected: `moved 2296 kept 797 non-stub diffs 0`. (Sibling heads, CLAUDE.md and other docs are
separate commits, §3.)

## §3. Cross-link rewrite rule

Counts are raw tokens at `39cc365a`, outside headings and code spans (`§N` not followed by a digit or
`.digit`); regions: **P** = kept, **INV** §4, **PLAN** 1007-2123, **TT** 2124-2314, **LED** §8,
**DEC** §9. The governing rule is the precedent's: **the entry links, carve-outs stay verbatim**.

| Class | Count | Rule |
|---|---|---|
| bare `§4/§5/§8/§9` in the **entry** (prose + table cells) | 59 (§4 1, §5 26, §8 18, §9 14) | → `[§N](<file>)`; §5 refs whose referent is the touch-time passage (one: 2318, "§5's *And the verdict…*") link `touch-time-split-record.md`, the rest `slice-plan.md`. Spec-clause collisions excluded by a **named** list derived at split time (heuristic finds 0). |
| same, in **moved** text | INV 33, PLAN 267, TT 11, LED 163, DEC 26 | verbatim; `§N` names the family section, resolved through the entry's stubs (numbers are stable) |
| same, in the 3 **existing** carve-outs | 76 raw (10 / 31 / 35) | verbatim — their heads already declare them statements about the pre-split umbrella |
| `§N` inside code spans | 4 (PLAN) | verbatim (commands/quotations) |
| links to existing carve-outs `](2026-07-vm-p4-….md)` | 84 (P 11, moved 73) | unchanged; same directory, so they resolve from the new files and keep the population transitive |
| intra-file anchors `](#…)` | 0 | — |
| self line cites (`line N`, `L N`, `<this file>:N` forms) | 0 | revision-anchored ranges (`N-M at <rev>`) stay verbatim as statements about that revision |
| `<this file>` in commands | 31 (P 1 = 26, citation table, stays valid; moved 30) | verbatim; each carve head says `<this file>` there means `git show 39cc365a:<entry path>`, as the precedent heads do for figures |
| `I-n` names | 168 | verbatim; the §4 stub lists I-1 … I-9 → `design-invariants.md` |
| `above` / `below` | 416 (PLAN 204, LED 140, P 41) | verbatim; the carve heads state they read in family order, which the stubs preserve (known crossing: 1988 "see the 1000-line check below" → TT) |
| carve heads of the 3 existing siblings ("the umbrella's §5") | 3 | re-point to `touch-time-split-record.md` in the pointer commit |
| other docs: `2026-08-plan-memo-umbrella-checker.md:663` at `e44ffce6` (`:653` at `39cc365a`; #562 inserted lines above it) "main memo line 2716" | 1 (only line cite into the memo found) | anchor it: "line 2716 at `39cc365a`" |
| CLAUDE.md:55 (checker run on edit of the memo) | 1 | extend the trigger to the carve-outs (edit any → run on the entry) |

Commit order in the split PR: (1) the content-free move + entry relink + stubs + heads (§2.1 passes);
(2) pointer commit — sibling heads, the checker-plan anchor, CLAUDE.md:55.

## §4. §5 ↔ §8 row naming after the split

- §5 table `Slot` column: **43** of 64 rows name a §8 slug; §8 `Trigger` column: **40** bold §5-row
  mentions across the 52 rows (24 cells).
- Both tables move **verbatim**; no link is added to a cell. Resolution is by **id**, and the checker's
  `Population.ids` is one population-wide map, so a §5 cell in `slice-plan.md` naming a slug declared
  in `slot-ledger.md` resolves exactly as before; duplicate declarations stay a miss across files.
- Attribution: a declaring row is attributed to the file holding it, a naming site to the file holding
  the citing cell (`Population.display`, relative to the entry's directory). Measured: no census or
  site count moves (§2).
- Readers resolve a cell's `§N` / row id through the entry's stubs; a cross-file link per cell is
  rejected (it would edit 83 cells for navigation the stubs already give, and break §2.1's byte proof).

## §5. What plan-review should focus on

1. The S0 checker contract is **settled** by PM judgments 52, 61 and 70 and by #562; it is not up for
   review here. What is in scope is §1.3's claim that this split keeps header drift at rc 2, because
   every schema ends up with one table in one memo. Also in scope: whether the schema-homes check
   should be part of the split PR's verification, and the linked stubs (§1.2) as the guard for the
   three children without a schema table.
2. §1.2 — the cohesive-unit exemption for `slice-plan.md` (1117), against the three candidate cuts.
3. §1.2 — is the 2124-2314 tail a real seam, given it closes with the `Intl`/`Atomics` paragraph
   (2306-2311) that is about §8 rather than file size?
4. §3 — the "entry links, carve-outs stay verbatim" rule; whether 30 moved `<this file>` commands are
   adequately disarmed by a head declaration.
5. §2 — that the predicted diff classes are complete (stub/head text must name no id).

## §6. Ledger correction

The slot's ledger note (`project_open-defer-slots.md:17`, "SCHEMAS が §5/§8 両 table を単一 file から
読む") gave #510 Slice 1 as the precondition. The precondition was S0, now landed. The PM corrects the
ledger when S0 lands (2026-10-31); this memo does not edit it.

## §7. Spec coverage map

**No web spec is implemented.** The split moves Markdown text and changes no behaviour. It does
depend on three outside rules, and each is a row below. Which text is a link decides the checker's
population (CommonMark §6.3 Links). Which text is masked as code decides what the relink rewrites
(CommonMark §6.1 Code spans). A schema table moves verbatim only if it is still recognised as a table
(GFM §4.10 Tables). The section numbers and names are the ones the checker plan's §3
(`docs/plans/2026-08-plan-memo-umbrella-checker.md`) cites, CommonMark 0.31.2 and GFM 0.29. webref
carries no CommonMark or GFM extract, so they cannot be looked up there. This memo's own proofs are
the `none (this memo)` rows.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CommonMark §6.3 Links | inline link, relative `.md` destination | new: the entry's relinked `[§N](2026-07-vm-p4-<file>.md)` (§3, 59 tokens) and the five stub links (§1.2) — each one an edge of the population | `plan_memo_sibling.sibling_path` → `Population.__init__` (BFS over `Memo.linked_files()`) | ✓ one form: same directory, no fragment, no title | no |
| CommonMark §6.3 Links | inline link, relative `.md` destination | existing: the 84 links to the three carve-outs (P 11, moved 73), text unchanged | resolves from the new files because they sit in the same directory | ✓ counted in §3 | no |
| CommonMark §6.3 Links | not a link | a bare `§N`, a backticked path or a bare file name has no destination and adds no edge (§0) — so a stub's backticked path alone would not keep its file in the population | §1.2 linked stubs | ✓ the three forms §0 names | no |
| CommonMark §6.1 Code spans | relink exclusion | a `§N` inside a code span stays verbatim (4, all in PLAN); a heading is not relinked | §3 relink rule; §2.1's link-undo regex reverses exactly the relink form | ✓ | no |
| GFM §4.10 Tables | recognition (header row + delimiter row) | the four schema tables move verbatim, so each header still matches its `SCHEMAS` entry; a header that drifted would make a non-schema table (#562 Known remainder (ii)) | `plan_memo_tables.SCHEMAS`, `Memo.tables` | ✓ four tables, one memo each (§1.3) | no |
| GFM §4.10 Tables | cell content | the cells that name rows of the other table (§4: 43 §5 `Slot` cells, 24 §8 `Trigger` cells) are not edited; no per-cell link | §4 | ✓ | no |
| none (this memo) §2.1 | byte proof | each child = its range; the entry, relinks undone = kept lines + stub-only insertions | §2.1 script | ✓ negative controls on both sides | no |
| none (this memo) §2 | checker invariants | rc, `[UMBRELLA-MARK]`, `[CENSUS]`, `[KIND-UNDETERMINED]`, `[UMBRELLA-CELL]`, footer | §2 command at the split head | ✓ every count compared | no |
| none (this memo) §1.3 | schema homes | one table per schema, one memo per schema | §1.3 snippet at the split head | ✓ four schemas | no |

**Breadth**: K=2 (CommonMark 0.31.2, GFM 0.29) plus this memo's own proofs, M=9
**Split decision**: one PR, two commits (§3). K and M are under both thresholds. The edge-dense trigger
(PM judgment 47) is met by this plan-review, not by slicing further: the tools change already went as
its own PR (S0, #562), and what remains is one content-free move, checked mechanically by §2 and §2.1.

### §7.1 User-input touch audit

None. No opcode, native, DOM or ECS surface is touched, and no user-controlled input reaches anything
the split changes. The only input is repository Markdown written by the memo's authors, read by the
checker. Data flow: SoT = the umbrella memo at `39cc365a` → the split files (the §2.1 proof) → the
checker population (links, §7 rows 1-3) → census and findings (§2).
