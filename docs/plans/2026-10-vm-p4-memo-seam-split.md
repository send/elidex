# VM P4 umbrella memo — seam split plan (`#11-vm-p4-memo-section-seam-split`)

**Status**: plan, revised after `/elidex-plan-review` R1. **Subject**:
`docs/plans/2026-07-vm-p4-es-language-completeness.md` at `39cc365a` (3093 lines). **Rule**: CLAUDE.md
*1000-line debt = touch-time split* — a cohesion judgment, executed inside #506 as a content-free
commit (1) plus a pointer commit (2) before #506 next edits the memo's content. Every line number
below is a line of that blob (`git show 39cc365a:<memo>`), never of a working tree (the umbrella memo
is unchanged over `39cc365a..a45a42d1`); a line number into any **other** file names its revision.
**Prerequisite S0 (§1.3) has landed**: PR #562, squash `e44ffce6` (2026-10-31).

This memo is deliberately **not linked** from the umbrella: a link would put it in the checker's
population, and every id it spells would become a naming site of the umbrella's census. The carve
heads name its path in backticks (§1.4), which is not a link and adds no edge.

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
change first** (§1.3) or a different layout (§1.6, rejected). That change is S0, landed as #562
(`e44ffce6`); with it the scratch split is rc 0 (§2).

Two smaller premise corrections: the `Trigger` column is **§8's** (`Slot | Why deferred | Trigger |
Re-eval`) and names §5 rows; the §5→§8 direction is §5's **`Slot`** column (§4 below). And the
"~19 cross-links" figure does not reproduce under any class I can define; §3 gives the measured classes.

## §1. Seams

### §1.1 Layout (ranges at `39cc365a`)

| File (`docs/plans/2026-07-vm-p4-…`) | Takes | Body lines | Simulated file |
|---|---|---|---|
| `es-language-completeness.md` (entry, stays) | head + §0.5-§3 (1-697), §6 stub (2315-2351), §7 (2352-2397), §§10-18 stubs (3077-3093), + 4 new stubs (§1.2) | 797 kept | 821 |
| `design-invariants.md` (new) | §4 I-1 … I-9 (698-1006) | 309 | 321 |
| `slice-plan.md` (new) | §5 conventions + per-row evidence + table + ordering rationale + per-row notes (1007-2123) | 1117 | 1129 |
| `touch-time-split-record.md` (new) | §5's 1000-line touch-time check through the `Intl`/`Atomics` note (2124-2314) | 191 | 205 |
| `slot-ledger.md` (new) | §8 whole, its table included (2398-2786) | 389 | 401 |
| `open-decisions.md` (new) | §9 decisions 1-17 (2787-3076) | 290 | 302 |

"Simulated file" is the scratch split of §2 (12-line heads, 14 for `touch-time-split-record.md`; four
6-line stubs in the entry). The ranges partition 1-3093 together with the kept ranges (checked by
§2.1's script: 2296 moved + 797 kept). Section numbers do **not** change: §4/§5/§8/§9 keep their
numbers in their new files, as §6 and §§10-18 did.

### §1.2 Why these seams, and the one file over 1000

- **§4** is a closed list of named invariants (I-1 … I-9) cited by name from every other section; it
  reads nothing in §5's table. **§8** carries its own table and the rules over it (Forward rule,
  reconciliation, cap accounting) — one unit. **§9** is the decision log. Each is its own subject.
- **§5's tail (2124-2314)** is a different subject from the table: a file-size discipline record
  (`vm/dispatch.rs` reversal, the watch list, this memo's own first carve). It is the passage the
  three existing carve heads point at ("the seam derivation … in the umbrella's §5").
- **`slice-plan.md` at 1117 body lines is exempt as a cohesive unit**, and this re-affirms the memo's
  own verdict at 2291 ("Its conventions passage and its table are one unit"), now with the reason
  measured over the current text: the conventions are "stated once for the whole table, and nowhere
  else" (1416); the per-row evidence bullets carry **ordering specification** the Deps cells defer to
  ("Shared prerequisite of Slices 2 and 3 (an ordering fact, so it is stated once here …", 1766); the
  ordering rationale (1987) reads the table. Each internal candidate cut — 1568|1569 (the evidence
  block opens "Evidence found by **that** round", whose antecedent is 1557-1567), 1919|1920 and
  1985|1986 (table boundaries) — separates rows from the specification they defer to. 204 of the 416
  `above`/`below` deixes are in this range. A flat-table exemption is **not** claimed: the table is
  66 lines; the bulk is prose. The exemption is a judgment about today's shape: if a later content
  edit grows the file, that edit applies the same touch-time rule then, against the seams it has.
- **Parent pointer form — one stub form for all eight carved sections.** The precedent
  (`## §6. … → **carved out**` at 2315; `## §§10-15 …` at 3077) sets the heading and the order: one
  stub per moved section, **in original section order** (the §4 and §5 stubs where §4 stood, the §8 and
  §9 stubs after §7), heading `## §N. <title> → **carved out**`. The body's first sentence is, for
  every stub, new and existing alike:

  ``Moved **verbatim** to `docs/plans/<file>` ([§N](<file>)) …``

  — the backticked path a reader copies, followed by one inline link to the same file, label = the
  section label the heading carries (`§4`, `§5`, `§8`, `§9`, `§6`, `§§10-15`, `§§16-18`). §5's stub
  names two files (`slice-plan.md`, and the touch-time check to `touch-time-split-record.md`), each in
  this form, so five links in four new stubs. The three **existing** stubs (2317, 3079, 3089) get the
  same parenthesised link inserted after their backticked path, on the same line, without reflow
  (pointer commit (2), §3). Before this revision they had none: their files' edges came only from
  incidental links in kept text (`slice-0a-landing-record.md` 4, `slice-1a-1b-call-spread-detail.md`
  5, `umbrella-review-rounds.md` 2; the §6 stub's own one link is to `umbrella-review-rounds.md`), so
  the family would have kept two stub forms, and a reader or a later carve would have to ask which one
  to write. Stub and carve-head text **name no declared id** (no row id, no `#11-` slug, no `I-n`), so
  they add no naming site (§2: zero inserted lines in the checker output).
- **Why the stub carries a link at all.** The relinked `§N` references alone give
  `design-invariants.md` one edge and `touch-time-split-record.md` one (2318, itself inside the §6
  stub), and those two files hold no schema table: losing that edge would drop them from the
  population **at rc 0** — the exit status does not see it. The stub link gives every child an edge
  that does not depend on incidental prose. What makes a lost edge **red** is the family guard (§1.5);
  without the guard a child could still leave the population silently.

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
presence is still met, and the drifted table's rows leave the census at rc 0. So "a drifted header is
rc 2" is **not** a property of the checker; it is a property of a layout in which every schema has
exactly one table in exactly one memo. After the split this family has that layout:

| Schema | Memo after the split | Tables (rows) |
|---|---|---|
| `citation` | entry (§0.5) | 1 (29) |
| `stub` | entry (§2.2) | 1 (55) |
| `slice` | `slice-plan.md` (§5) | 1 (64) |
| `slot` | `slot-ledger.md` (§8) | 1 (52) |

No other memo of the population holds one (the three existing carve-outs and the three children
without a schema table). **Measured** on the commit-(2) scratch split of §2.2 with the checker at `a45a42d1`:
renaming one header column of each table in turn gave rc 2 and `no table matched schema '<name>'`,
for all four. That holds only while the layout holds. A later edit that puts a second table of one
schema in another memo opens rule C's silent case, and nothing in the checker would say so. The
family guard (§1.5, assertion (ii)) is what turns that edit red; without it, the layout — and with it
the rc 2 on drift — could disappear unnoticed.

**A child the entry forgets to link.** `slice-plan.md` and `slot-ledger.md` hold the only `slice` and
`slot` tables, so losing their edge is rc 2. Losing the edge to one of the five children without a
schema table drops it from the population at rc 0, and only naming-site counts move. The guard's
assertion (i) catches it (measured: deleting both links to `design-invariants.md` from the scratch
entry leaves rc 0 and turns (i) red with exactly that file).

### §1.4 Carve heads (commit (1))

Each new child opens with a head of about twelve lines, written in commit (1). §2.1 checks only that
the body ends with its range, so the head is free text, constrained as follows:

- it states what the file is a carve-out of (the entry path, **backticked**), that the body moved
  **verbatim** at `39cc365a`, and that every figure, line anchor and revision reference in it is a
  statement about the umbrella at `39cc365a`;
- it names **this plan's path in backticks** — `docs/plans/2026-10-vm-p4-memo-seam-split.md` — as
  where the seam derivation, the ranges and the content-free proof live, and says why it is not a link
  (a link would make this plan a population member). This split's derivation exists only here, so
  without the pointer a reader of a child has no route to it;
- it states the reading rules the precedent heads state: an unqualified section number names that
  section of the family, located through the entry's stubs; `<this file>` in a command means
  `git show 39cc365a:<entry path>`; `above` / `below` read in family section order;
- `touch-time-split-record.md` adds a **tense** note: the passage "It is discharged here" (in the
  "Owner and discharge" paragraph, 2296-2302) is the first carve's statement, written in the umbrella
  about that carve; it moved verbatim, and this second carve is discharged by the split PR this plan
  describes (§6);
- it **names no declared id**: no row id, no `#11-` slug (this plan's own slot included), no `I-n`,
  and no bold numeral. Measured in §2: zero head or stub lines appear in the checker output.

### §1.5 Family guard — a tracked test (pointer commit (2))

PM judgment 99 (1). Rule C (≥ 1, ≥ 2 allowed) is unchanged; the guard pins the facts **this family**
asserts about its own layout, which §1.2 and §1.3 depend on and the checker deliberately does not
enforce.

- **File**: `.claude/tools/test_plan_memo_vm_p4_family_layout.py` (NEW). It is a member of the
  `tools` job's set (`scripts/python-suites.sh` step 2: tracked `**/test_*.py`); its root is
  `.claude/tools`, so step 4's loader puts that directory at `sys.path[0]` and
  `from plan_memo_population import Population` resolves the checker's own module — no parser of its
  own. It is not an entry-point wrapper (no subprocess, no `--self-test`), so step 3(b)'s naming
  clause does not apply; it reads only local files (offline regime, step 5 needs ≥ 1 passing test).
  The entry path is built from the file's own directory
  (`os.path.join(HERE, "..", "..", "docs", "plans", ENTRY)`), not from the cwd.
- **Shape**: one `unittest.TestCase`, `VmP4FamilyLayout`, building one `Population` on the entry:
  - precondition: `p.misses == []` — a population with misses is unscanned, and the two sets below
    would mean nothing;
  - **(i)** `test_population_is_the_family`: `{p.display(m.path) for m in p.memos}` equals the pinned
    set of nine names — the entry, the five new children, the three existing carve-outs. A **set**, not
    the BFS list: first-link order moves with ordinary prose edits;
  - **(ii)** `test_each_schema_has_one_home`: the map schema name → list of `p.display(m.path)`, one
    entry per table whose `t.schema` is that schema, equals
    `{"citation": [ENTRY], "stub": [ENTRY], "slice": ["2026-07-vm-p4-slice-plan.md"], "slot":
    ["2026-07-vm-p4-slot-ledger.md"]}`; and the pinned keys equal `{s.name for s in SCHEMAS}`
    (`plan_memo_tables.SCHEMAS`), so a fifth schema forces the pin to be revisited. One list of length
    one per schema pins both "exactly one table" and "in which memo". Row counts are **not** pinned.
- **When its values change**: a content edit (prose, rows added or removed, links among members)
  changes neither assertion. Only a structural change does — a memo entering or leaving the
  population, a schema table moving to, or appearing in, another memo, a header drifting off its
  schema.
- **Docstring** (required wording, in substance): this test does **not** restate or tighten the
  checker's presence rule (rule C: ≥ 1 memo, ≥ 2 allowed); it pins this family's layout. *When a
  structural change moves a schema table to another memo, adds a carve-out, or otherwise changes the
  layout, update this test in the same commit — that update is the intended change, not a workaround.*
  Without that sentence the test would read as one-home re-introduced outside the checker.
- **Measured** (prototype, `vm-p4-artifacts/split-sim/guard_proto.py`, same logic): green on the
  commit-(2) scratch split; red on the current pre-split tree (population 4 memos, all four schemas
  at the entry) — which is why it lands in commit (2), after the move; red with exactly
  `design-invariants.md` when that child's links are removed.
- **Scope note for the split PR**: this puts tools code into #506, a plan PR. The split PR's
  description says so, and the `/pre-push` run for the split covers it (`mise run tools-test` /
  `mise run ci` run it in both matrix legs).

### §1.6 Rejected: keep all four tables in the entry memo (no S0)

Entry = §0.5-§3 + §5's table block + §7 + §8's table block (~940 lines); conventions/evidence in
children. It needs no tools change, but it **reverses** the 2291 verdict by separating the table from
the specification its cells defer to, and splits §8 from its own rules. Cost-driven, so rejected.
(Moot since S0 landed; kept as the record of why the split waited on a tools change.)

## §2. Coupled invariants, and the measured checker diff

### §2.0 The invariants and their intersections

The split satisfies nine invariants at once:

- **C1 byte-exact move** — each child's body = its range; entry = kept lines + stub insertions (§2.1).
- **C2 entry relink** — 59 bare `§4/§5/§8/§9` in the entry become links (§3); 2318's `§5` →
  `touch-time-split-record.md`.
- **C3 no new naming site** — stub and head text name no declared id (§1.2, §1.4).
- **C4 population membership** — every child is reached by an edge of its own (the stub link).
- **C5 schema home** — each of the four schemas has one table in one memo (§1.3).
- **C6 predicted checker diff** — §2.2's classes and nothing else, at both commit heads.
- **C7 family order** — `above`/`below` and section order read across files as before.
- **C8 pointer commit (2)** — three existing stub links, three existing head re-points, the
  checker-plan anchor, CLAUDE.md:55 (at `a45a42d1`), the guard (§3).
- **C9 family guard** — (i) and (ii) of §1.5.

| Pair | Intersection |
|---|---|
| C1 × C2 | the relink edits kept lines; §2.1 undoes exactly the relink form (`[§N](<new file>)`) before diffing, so a relink is the one edit to a kept line commit (1) allows |
| C1 × C3 | heads and stubs are outside the byte proof (head = text before the range; stubs = inserted chunks containing `→ **carved out**`), so C3 is checked by the checker output, not by §2.1 |
| C1 × C4 | stub links are in inserted chunks, so they pass §2.1 as stub-only insertions |
| C1 × C5 | the four schema tables are in moved or kept ranges, so §2.1 proves each moved byte-for-byte; header drift by the move is impossible |
| C1 × C6 | class 2 of §2.2 exists because a child's line numbers are its range shifted by the head length; §2.2 maps them back (`a + N − h − 1`) |
| C1 × C7 | the bodies are verbatim, so deixes keep their text; order is restored by stub order, not by editing them |
| C1 × C8 | commit (2) edits three **kept** entry lines (2317, 3079, 3089), so §2.1 is run at the commit-(1) head; at the commit-(2) head it reports exactly those 3 replacements (measured), and commit (2) has its own check (§3) |
| C1 × C9 | none: the guard reads structure, not bytes |
| C2 × C3 | a relink rewrites existing text into a link whose label is `§N`, never an id; class 1 of §2.2 is excerpt text only |
| C2 × C4 | each relink is an edge; it is not relied on for membership (C4 uses the stub link) because an incidental reference can be edited away |
| C2 × C5 | relinks edit cells of the entry's `stub` table (e.g. 439-475), never a header or delimiter row, so the table still matches its schema (measured: every count identical) |
| C2 × C6 | class 1 of §2.2: 11 entry lines whose excerpt now shows a link |
| C2 × C7 | none: deixes are untouched, links do not reorder text |
| C2 × C8 | commit (2) adds no relink; its entry edits are the three stub links only |
| C2 × C9 | relinks add edges among members only, so (i) is unchanged |
| C3 × C4 | the stub link's label is a section label, so the edge it adds names nothing (zero inserted lines in the output) |
| C3 × C5 | heads and stubs contain no table, so no schema table is added |
| C3 × C6 | zero head/stub lines in the output (measured: no checker line maps to an inserted line) |
| C3 × C7 | the heads state the family-order reading rule; the rule is text, not an id |
| C3 × C8 | the head re-points and stub links in commit (2) name only file paths and section labels |
| C3 × C9 | none |
| C4 × C5 | `slice-plan.md` / `slot-ledger.md` leaving the population is rc 2 by presence; the other five leaving is rc 0 — the asymmetry (ii) and (i) are split along |
| C4 × C6 | the population header line changes (four → nine memos, BFS order) — class 0 of §2.2 |
| C4 × C7 | stub order = original section order = family order |
| C4 × C8 | the three existing stub links add edges to files already reached earlier in BFS; measured: population order and the whole output unchanged between the two heads |
| C4 × C9 | (i) is what makes a lost edge red at rc 0 |
| C5 × C6 | rows keep their file attribution: the 64 §5 rows read from `slice-plan.md`, the 52 §8 rows from `slot-ledger.md`; every count identical |
| C5 × C7 | none |
| C5 × C8 | commit (2) adds no table |
| C5 × C9 | (ii) pins the one-home layout that keeps drift at rc 2 under rule C |
| C6 × C7 | none: the checker reads no deixis |
| C6 × C8 | commit (2) must not move any output: measured, the commit-(2) output is **byte-identical** to the commit-(1) output |
| C6 × C9 | the guard is a separate test; it adds nothing to the checker output |
| C7 × C8 | the head re-point names a file; it moves no text |
| C7 × C9 | none |
| C8 × C9 | the guard lands in commit (2), after the move, because at commit (1)'s parent it is red by design |

### §2.1 Content-free proof (commit (1))

Byte-exact: each child's body is its range; the entry, with the new links undone, is the kept lines
plus stub-only insertions. Run at the **commit-(1)** head. Negative controls run: one edited char in
the entry → 1 non-stub diff; one in a child → `AssertionError`.

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

Expected at commit (1): `moved 2296 kept 797 non-stub diffs 0` (measured on the scratch split). At
commit (2) the same script reports 3 (the stub-link lines) — expected, and not the check for (2).

### §2.2 Checker output: baseline, commit (1), commit (2)

**Baseline** (`39cc365a`, saved: `vm-p4-artifacts/p506_check_pre.txt`, 868 lines; `a45a42d1` gives a
byte-identical output): rc **0**; `[UMBRELLA-MARK] 48` (`slice=33, slot=15`); `[CENSUS] 53`;
`[KIND-UNDETERMINED] 5`; `[UMBRELLA-CELL] 37`; footer `51 seed(s) and 715 reported naming site(s)`;
population = the three existing carve-outs.

**Prediction, measured on a scratch split of both heads** (`vm-p4-artifacts/split-sim/sim2.py
<dir> 1|2`, checker at `a45a42d1` = S0 landed; outputs `sim2_after_c1.txt` / `sim2_after_c2.txt`;
`norm2.py` maps each `file:N` back to its `39cc365a` line). The scratch split is this plan's layout:
§1.1 ranges, §1.4-shaped heads (12 lines, 14 for the touch-time file), the stub form of §1.2 in
original section order, 2318's `§5` linked to `touch-time-split-record.md`, and at stage 2 the three
existing stub links and head re-points. (The earlier `simulate.py` linked 2318 to `slice-plan.md`,
used 4-line heads, and put all five stubs before §6; `sim2.py` corrects all three.)

- **commit (1) head vs baseline**: rc 0, 868 lines, every count above **identical**; **no table row
  changes file attribution**. Sorted, after mapping locators back, the diff is **15 output lines on
  12 source lines**, in three classes:
  0. the population header line (nine memos, BFS order) — normalised away by the mapping;
  1. excerpt text of **11** entry lines whose bare `§N` became a link (157, 439, 443, 454-456, 458,
     475, 585, 647, 654);
  2. the in-message locator of the one `row <no id> at :N (Function/eval)` ORDER-PROSE? finding (1985),
     which is child-relative (`:991` at a 12-line head).
  No checker line maps to a head or stub line. Unsorted, the naming-site listing also reorders
  (population is BFS order).
- **commit (2) head vs commit (1) head**: **byte-identical** output. The three entry stub-link
  insertions and the three carve-out head re-points are single-line edits without reflow, so no
  file's line count changes (no locator moves), none of those lines is a naming site, and the edges
  they add point at memos BFS has already scheduled (population order unchanged).

Anything else — a count, an `[UMBRELLA-MARK?]` row, a seed, any difference between the two heads — is
a defect of the split. At the real heads:

```sh
M=docs/plans/2026-07-vm-p4-es-language-completeness.md
git checkout <c1> && PYTHONDONTWRITEBYTECODE=1 python3 .claude/tools/plan-memo-umbrella-check.py $M > after1.txt; echo rc=$?
git checkout <c2> && PYTHONDONTWRITEBYTECODE=1 python3 .claude/tools/plan-memo-umbrella-check.py $M > after2.txt; echo rc=$?
cmp after1.txt after2.txt
grep -E '^\[(CENSUS|UMBRELLA-MARK|UMBRELLA-CELL|KIND-UNDETERMINED)\]|seed\(s\)' after2.txt
```

## §3. Cross-link rewrite rule

Counts are raw tokens at `39cc365a`, outside headings and code spans (`§N` not followed by a digit or
`.digit`); regions: **P** = kept, **INV** §4, **PLAN** 1007-2123, **TT** 2124-2314, **LED** §8,
**DEC** §9. The governing rule is the precedent's: **the entry links, carve-outs stay verbatim**.
A heading is not relinked (local policy of this count, not a Markdown rule; §7).

| Class | Count | Rule |
|---|---|---|
| bare `§4/§5/§8/§9` in the **entry** (prose + table cells) | 59 (§4 1, §5 26, §8 18, §9 14) | commit (1): → `[§N](<file>)`; §5 refs whose referent is the touch-time passage (one: 2318, "§5's *And the verdict…*") link `touch-time-split-record.md`, the rest `slice-plan.md`. Spec-clause collisions excluded by a **named** list derived at split time (heuristic finds 0). |
| same, in **moved** text | INV 33, PLAN 267, TT 11, LED 163, DEC 26 | verbatim; `§N` names the family section, resolved through the entry's stubs (numbers are stable) |
| same, in the 3 **existing** carve-outs | 76 raw (10 / 31 / 35) | verbatim — their heads already declare them statements about the pre-split umbrella |
| `§N` inside code spans | 4 (PLAN) | verbatim (commands/quotations) |
| links to existing carve-outs `](2026-07-vm-p4-….md)` | 84 (P 11, moved 73) | unchanged; same directory, so they resolve from the new files |
| existing stubs (2317, 3079, 3089) | 3 | commit (2): insert `([§6](…slice-1a-1b-call-spread-detail.md))`, `([§§10-15](…umbrella-review-rounds.md))`, `([§§16-18](…slice-0a-landing-record.md))` after the backticked path, same line, no reflow — the one stub form (§1.2) |
| intra-file anchors `](#…)` | 0 | — |
| self line cites (`line N`, `L N`, `<this file>:N` forms) | 0 | revision-anchored ranges (`N-M at <rev>`) stay verbatim as statements about that revision |
| `<this file>` in commands | 31 (P 1 = 26, citation table, stays valid; moved 30) | verbatim; each carve head says `<this file>` there means `git show 39cc365a:<entry path>` (§1.4), as the precedent heads do for figures |
| `I-n` names | 168 | verbatim; the §4 stub says the named invariants keep their names in `design-invariants.md`, **without spelling any** (C3) |
| `above` / `below` | 416 (PLAN 204, LED 140, P 41) | verbatim; the carve heads state they read in family order, which the stubs preserve (known crossing: 1988 "see the 1000-line check below" → TT) |
| carve heads of the 3 existing siblings ("the umbrella's §5", line 5 of each) | 3 | commit (2): append `, now \`docs/plans/2026-07-vm-p4-touch-time-split-record.md\`` inside the parenthesis, same line, no reflow; **backticked**, not linked — membership does not need it, and a link would add an edge for navigation only |
| other docs: line cites into the umbrella | 3, all in `docs/plans/2026-08-plan-memo-umbrella-checker.md` at `a45a42d1` (file identical to `e44ffce6`): `:663` "main memo line 2716"; `:1098` `` `190d2adb:…:1218` `` / `` `:2711` ``; `:1103` `` `190d2adb:…:2710` `` | commit (2): anchor `:663` as "line 2716 at `39cc365a`"; `:1098` / `:1103` are already anchored to `190d2adb` and stay. Complement measured (below). |
| CLAUDE.md:55 at `a45a42d1` (branch `vm-p4-plan-doc`; on main `e44ffce6` line 55 is the **Git** rule — the checker-run rule is added on this branch) | 1 | commit (2): extend the trigger to the carve-outs (edit any family memo → run on the entry) |

Complement for the "other docs" row — every tracked file outside the family and this plan that
mentions the umbrella, then every line-cite form in those files:

```sh
for f in $(git grep -lE 'es-language-completeness|#506.s memo|main memo' a45a42d1 -- . \
    ':!docs/plans/2026-07-vm-p4-*' ':!docs/plans/2026-10-vm-p4-memo-seam-split.md' | sed 's/^a45a42d1://'); do
  git show a45a42d1:$f | /usr/bin/grep -nE 'es-language-completeness\.md`?:[0-9]|main memo line|…:[0-9]|memo.{0,12}line [0-9]{3,4}' | sed "s|^|$f:|"
done
```

→ the three lines above, nothing else (the selector matches 9 files: six under `.claude/tools/`,
CLAUDE.md, the checker plan and its rounds record).

**Commit order in the split PR.**

1. **Content-free commit**: the five children (heads per §1.4), the entry's 59 relinks, the four new
   stubs. Checks: §2.1 passes; §2.2's commit-(1) prediction holds.
2. **Pointer commit**: the three existing stub links (§1.2), the three existing head re-points, the
   checker-plan `:663` anchor, CLAUDE.md:55, and the guard `.claude/tools/test_plan_memo_vm_p4_family_layout.py`
   (§1.5). Checks: `git diff --numstat <c1> <c2>` shows the entry `3 3`, each of the three carve-outs
   `1 1`, the checker plan `1 1`, CLAUDE.md `1 1`, and the new test file, nothing else; checker output
   byte-identical to commit (1)'s (§2.2); the guard green; `mise run tools-test` green.

## §4. §5 ↔ §8 row naming after the split

- §5 table `Slot` column: **43** of 64 rows name a §8 slug; §8 `Trigger` column: **40** bold §5-row
  mentions across the 52 rows (24 cells).
- Both tables move **verbatim**; no link is added to a cell. Resolution is by **id**, and the checker's
  `Population.ids` is one population-wide map, so a §5 cell in `slice-plan.md` naming a slug declared
  in `slot-ledger.md` resolves exactly as before; duplicate declarations stay a miss across files.
- Attribution: a declaring row is attributed to the file holding it, a naming site to the file holding
  the citing cell (`Population.display`, relative to the entry's directory). Measured: no census or
  site count moves (§2.2).
- Readers resolve a cell's `§N` / row id through the entry's stubs; a cross-file link per cell is
  rejected (it would edit 83 cells for navigation the stubs already give, and break §2.1's byte proof).

## §5. What plan-review should focus on

1. The S0 checker contract is **settled** by PM judgments 52, 61 and 70 and by #562; not in scope.
   In scope: the family guard (§1.5) as the thing that keeps §1.3's "drift is rc 2" and §1.2's
   "every child is a member" true after the split — its two assertions, the claim that content edits
   never change them, and the docstring boundary against rule C.
2. §2.0 — whether the nine invariants are the right set and each intersection line is right; in
   particular C1 × C8 and C6 × C8 (commit (2) edits kept lines and must not move the output).
3. §1.2 — the one stub form, applied to the three existing stubs in commit (2).
4. §1.2 — the cohesive-unit exemption for `slice-plan.md` (1117), against the three candidate cuts.
5. §1.2 — is the 2124-2314 tail a real seam, given it closes with the `Intl`/`Atomics` paragraph
   (2306-2311) that is about §8 rather than file size?
6. §3 — the "entry links, carve-outs stay verbatim" rule; whether 30 moved `<this file>` commands are
   adequately disarmed by a head declaration (§1.4).

## §6. Discharge, ledger, and hand-off

- **Discharge.** The split discharges the slot `#11-vm-p4-memo-section-seam-split`. It is closed when
  #506 is squash-merged with commits (1) and (2) in it; not at commit time, not at plan approval.
- **Ledger.** The slot's ledger note (`project_open-defer-slots.md:17`, "SCHEMAS が §5/§8 両 table を
  単一 file から読む") gave #510 Slice 1 as the precondition; the precondition was S0, now landed.
  The ledger belongs to the PM: the driver reports the landing (squash hash, the two commit hashes)
  and the PM records the discharge and the corrected precondition. This memo does not edit the ledger.

## §7. Spec coverage map

**No web spec is implemented.** The split moves Markdown text and changes no behaviour. It depends
on outside rules in three places, each a row below. Which destination is a sibling decides the
checker's population — the WHATWG URL Standard's parser decides that a destination has no scheme, and
the rest is the checker's local policy (the checker plan's §3 row on `plan_memo_sibling.sibling_path`,
`docs/plans/2026-08-plan-memo-umbrella-checker.md:434` at `a45a42d1`, says CommonMark and GFM say
nothing about siblings on disk). Which text is a link, and which is masked as code, is CommonMark.
Whether a schema table still is a table is GFM. URL Standard numbers and names are webref lookups
(`.claude/tools/webref heading url 4.4 --exact` → §4.4 URL parsing `#url-parsing`; `heading url 1.3
--exact` → §1.3 Percent-encoded bytes; `dfn url 'scheme start state'` → §4.4 `#scheme-start-state`;
`dfn url percent-decode` → §1.3 `#string-percent-decode`). CommonMark 0.31.2 / GFM 0.29 numbers are
the ones the checker plan's §3 cites (rows at `:409-421`, `:758`); webref has no CommonMark or GFM
extract, and the vendored `.claude/tools/commonmark-0.31.2-{block,inline}-examples.json` carry
section **names** only — Link reference definitions, Code spans, Links, Autolinks all appear there
under those names.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG URL §4.4 URL parsing | scheme start state (`#scheme-start-state`), then **local policy** | every new destination begins `2026-…`: step 1 needs an ASCII alpha, so step 2 → *no scheme state* and the destination is relative; the rest is the checker's local policy (checker plan `:434`): relative on every platform, `.md`, joined beside the memo — all new destinations are bare file names in the same directory | `plan_memo_sibling.sibling_path` stages (a), (c)-(e) → `Population.__init__` | ✓ one destination shape (§3) | no |
| WHATWG URL §1.3 Percent-encoded bytes | percent-decode on a string (`#string-percent-decode`) | no new destination contains `%`; decoding is the identity | `plan_memo_sibling.sibling_path` stage (b) | ✓ | no |
| CommonMark §6.3 Links | inline link, relative `.md` destination | new: the 59 relinks (commit (1)), the five new stub links (commit (1)), the three existing stub links (commit (2)) — each an edge | `Memo.linked_files()` → `Population.__init__` (BFS) | ✓ one form: same directory, no fragment, no title | no |
| CommonMark §6.3 Links | inline link, relative `.md` destination | existing: the 84 links to the three carve-outs (P 11, moved 73), text unchanged | resolve from the new files (same directory) | ✓ counted in §3 | no |
| CommonMark §6.3 Links | not a link | a bare `§N` and a bare file name have no destination and add no edge (§0) — so a stub's backticked path alone would not keep its file in the population, and the heads' backticked plan path adds none | §1.2 stub link; §1.4 | ✓ | no |
| CommonMark §4.7 Link reference definitions | definition | the umbrella at `39cc365a` has **0** definitions, so no reference link resolves in it and none can cross a file boundary (a definition is matched within its own document): `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md \| /usr/bin/grep -cE '^ {0,3}\[[^]]+\]:'` → 0 (the 11 `][` sequences are all inside code spans or fences) | — | ✓ | no |
| CommonMark §6.5 Autolinks | `<…>` URI autolink | 0 `<…md>` forms: same blob `\| /usr/bin/grep -cE '<[^<> ]*\.md([#?][^<> ]*)?>'` → 0; no autolink edge exists or is added | — | ✓ | no |
| CommonMark §6.1 Code spans | relink exclusion | a `§N` inside a code span stays verbatim (4, all in PLAN); a backticked path is a code span, not a link | §3 relink rule; §2.1's link-undo regex reverses exactly the relink form | ✓ | no |
| CommonMark §4.2 ATX headings | heading line, then **local policy** | a heading's text is inline content to CommonMark; that a `§N` in a heading line is **not relinked** is §3's own count rule (local policy), not a consequence of §6.1 | §3 | ✓ | no |
| GFM §4.10 Tables | recognition (header row + delimiter row) | the four schema tables move verbatim, so each header still matches its `SCHEMAS` entry; a drifted header makes a non-schema table (#562 Known remainder (ii)) | `plan_memo_tables.SCHEMAS`, `Memo.tables`; guard (ii) | ✓ four tables, one memo each (§1.3) | no |
| GFM §4.10 Tables | cell content | the cells that name rows of the other table (§4: 43 §5 `Slot` cells, 24 §8 `Trigger` cells) are not edited; no per-cell link | §4 | ✓ | no |


This memo's own checks are not spec rows and are not counted in K or M:

- **§2.1 byte proof** (commit (1)): each child = its range; the entry, relinks undone = kept lines +
  stub-only insertions; negative controls on both sides.
- **§2.2 checker output** (both heads): rc, the four counts and the footer compared; commit (2)
  byte-identical to commit (1).
- **§1.5 family guard**: (i) population = the nine memos; (ii) one table per schema, in the pinned
  memo, with the `SCHEMAS` keys pinned.

**Breadth**: K=3 (WHATWG URL, CommonMark 0.31.2, GFM 0.29), M=11.
**Split decision**: one PR, two commits (§3). The edge-dense trigger (PM judgment 47) is met by this
plan-review, not by slicing further: the checker change already went as its own PR (S0, #562), and
what remains is one content-free move plus one pointer commit, each checked mechanically (§2.1, §2.2,
§1.5).

### §7.1 User-input touch audit

None. No opcode, native, DOM or ECS surface is touched, and no user-controlled input reaches anything
the split changes. The only input is repository Markdown written by the memo's authors, read by the
checker and by the guard. Data flow: SoT = the umbrella memo at `39cc365a` → the split files (§2.1)
→ the checker population (links, §7 rows 1-7) → census and findings (§2.2), and the guard's two
assertions over the same population (§1.5).
