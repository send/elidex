# VM P4 umbrella memo — seam split plan (`#11-vm-p4-memo-section-seam-split`)

**Status**: plan, for `/elidex-plan-review`. **Subject**: `docs/plans/2026-07-vm-p4-es-language-completeness.md`
at `39cc365a` (3093 lines). **Rule**: CLAUDE.md *1000-line debt = touch-time split* — a cohesion
judgment, executed as a standalone, content-free commit before #506 next edits the memo's content.
Every line number below is a line of that blob (`git show 39cc365a:<memo>`), never of a working tree.

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
- **False — schema presence is entry-memo-only.** `plan_memo_population.py:103-108` builds
  `matched` from `self.main.tables`: each of the **four** `SCHEMAS` (`citation` §0.5, `stub` §2.2,
  `slice` §5, `slot` §8 — not only §5/§8) must have a table **in the memo the checker is invoked on**,
  else `FATAL [SCHEMA] … no table matched schema` and rc 2. The checker plan states this itself
  (":278 … schema presence is required of the main memo only").

**Measured** (scratch copy, split as in §1, unmodified checker): rc 2,
`no table matched schema 'slice'` and `… 'slot'`. So the seam layout below needs **one checker
change first** (§1.3), or a different layout (§1.4, rejected).

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

### §1.3 Prerequisite S0 — schema presence over the population (tools, separate PR, before the split)

`matched = {t.schema.name for m in self.memos for t in m.tables if t.schema is not None}` at
`plan_memo_population.py:104`, plus: the module docstring's rules, the checker plan's ":278" sentence,
one CONTROL (a schema table only in a linked sibling → rc 0; absent from every memo → rc 2; present
only in an **unlinked** sibling → rc 2) and one mutant (revert to `self.main.tables`). Why this and
not a memo-side workaround: presence and census then read **one** population (#510's own charter),
and the fail-safe direction holds — a table that drifts, or a child the entry forgets to link, still
leaves its schema matched nowhere → rc 2. **Measured** with the one-line change on a scratch copy:
the existing self-test passes unchanged (`--self-test`: 949 controls, all as declared); mutants not
run. No existing sibling holds a schema-matching table today (headers grepped), so nothing else moves.

### §1.4 Rejected: keep all four tables in the entry memo (no S0)

Entry = §0.5-§3 + §5's table block + §7 + §8's table block (~940 lines); conventions/evidence in
children. It needs no tools change, but it **reverses** the 2291 verdict by separating the table from
the specification its cells defer to, and splits §8 from its own rules. Cost-driven, so rejected.

## §2. Invariants, measured before and after

Baseline (saved full output, 868 lines; `PYTHONDONTWRITEBYTECODE=1 python3
.claude/tools/plan-memo-umbrella-check.py docs/plans/2026-07-vm-p4-es-language-completeness.md`):
rc **0**; `[UMBRELLA-MARK] 48` (`slice=33, slot=15`); `[CENSUS] 53`; `[KIND-UNDETERMINED] 5`;
`[UMBRELLA-CELL] 37`; footer `51 seed(s) and 715 reported naming site(s)`; population = the three
existing carve-outs.

**Predicted after** (measured on a scratch split of exactly §1.1's ranges with S0 applied, 4-line
heads, parent `§N` relinked per §3): rc 0, 868 lines, every count above **identical**; **no table
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
| other docs: `2026-08-plan-memo-umbrella-checker.md:653` "main memo line 2716" | 1 (only line cite into the memo found) | anchor it: "line 2716 at `39cc365a`" |
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

1. §0/§1.3 — is S0 (presence over the population) the right checker contract, or should presence stay
   entry-only (§1.4)? Its fail-safe argument and the missing mutant run.
2. §1.2 — the cohesive-unit exemption for `slice-plan.md` (1117), against the three candidate cuts.
3. §1.2 — is the 2124-2314 tail a real seam, given it closes with the `Intl`/`Atomics` paragraph
   (2306-2311) that is about §8 rather than file size?
4. §3 — the "entry links, carve-outs stay verbatim" rule; whether 30 moved `<this file>` commands are
   adequately disarmed by a head declaration.
5. §2 — that the predicted diff classes are complete (stub/head text must name no id).

## §6. Ledger correction owed

`project_open-defer-slots.md:17`: "SCHEMAS が §5/§8 両 table を単一 file から読む" → "all four
schemas' presence is checked on the entry memo only"; the precondition is S0, not #510 Slice 1.
