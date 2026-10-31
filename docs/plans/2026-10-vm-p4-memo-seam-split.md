# VM P4 umbrella memo — seam split plan (`#11-vm-p4-memo-section-seam-split`)

**Status**: plan, revised after `/elidex-plan-review` R1 and R2. **Subject**:
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
  that does not depend on incidental prose. What makes a child **leaving** the population red is the
  family guard (§1.5 (i)); without the guard a child could still leave it silently. The guard pins
  membership, not the stub edge itself (§2.0 C4).

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
schema becomes a non-schema table. Under rule C, if another memo carries the same schema, presence is
still met, and the drifted table is then caught only by `Population._unbound_claims`
(`plan_memo_population.py:174` at `e44ffce6`): a table bound to **no** schema that has a cell carrying
a `KIND_PHRASES` phrase (marker / undetermined / pointer) is a miss, rc 2. A drifted table with **no**
such cell leaves the census at rc 0 — that is the silent case, and its reach depends on the table's
rows. So "a drifted header is rc 2" is **not** a property of the checker. In full it is a property of
a layout in which every schema has exactly one table in exactly one memo; short of that, it holds only
for a table whose rows carry a kind phrase. After the split this family has the one-table layout:

| Schema | Memo after the split | Tables (rows) | Rows with a kind phrase | Drift, one home | Drift, second home added |
|---|---|---|---|---|---|
| `citation` | entry (§0.5) | 1 (29) | 0 | rc 2 (presence) | **rc 0 — silent** |
| `stub` | entry (§2.2) | 1 (55) | 1 (:471) | rc 2 (presence) | rc 2 (unbound claim, `undetermined` at :471) |
| `slice` | `slice-plan.md` (§5) | 1 (64) | 41 | rc 2 (presence) | rc 2 (unbound claim, `marker`) |
| `slot` | `slot-ledger.md` (§8) | 1 (52) | 16 | rc 2 (presence) | rc 2 (unbound claim, `marker`) |

No other memo of the population holds one (the three existing carve-outs and the three children
without a schema table). **Measured** on the commit-(2) scratch split with the checker at `a45a42d1`
(`python3 vm-p4-artifacts/split-sim/r2-drift.py vm-p4-artifacts/split-sim/r2-drift/base`, the split
made by `sim2.py … 2`): "rows with a kind phrase" asks `Population._claims` of every cell, header
included — the predicate `_unbound_claims` applies; "drift" prefixes the first header cell with `Q`;
"second home added" also appends the same header and delimiter row (no body) to
`slice-0a-landing-record.md`, so presence is met elsewhere. So under rule C the silent case is today
exactly `citation`, and `stub` is one edited row away from it (its one kind-phrase cell, :471). The
one-table layout is what keeps all four at rc 2, and only while it holds: a later edit that puts a
second table of one schema in another memo opens the silent case for `citation` (and for any table
whose kind-phrase rows are later edited away), and nothing in the checker would say so. The family
guard (§1.5, assertion (ii)) is what turns that edit red. **A pin list of length one is that
guarantee**: updating a schema's pin to two memos is the edit that gives the guarantee up for that
schema, and the guard's docstring says so (§1.5).

**A child the entry forgets to link.** `slice-plan.md` and `slot-ledger.md` hold the only `slice` and
`slot` tables, so losing their edge is rc 2. Losing the edge to one of the five children without a
schema table drops it from the population at rc 0, and only naming-site counts move. The guard's
assertion (i) catches it — when the child actually leaves. Measured on the simulated commit-(2) head
(§1.5): deleting only the §4 stub's link keeps `design-invariants.md` reached through the one relinked
`§4`, and the guard stays green; deleting that relink too leaves the checker at rc 0 and turns (i) red
with exactly that file. (i) pins membership, not which edge provides it.

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
enforce. Line numbers into `.claude/tools/` files in this section are at `e44ffce6` (unchanged at
`08a2a68c`).

- **File**: `docs/plans/test_vm_p4_family_layout.py` (NEW) — beside the family it pins, **not** beside
  the checker. R1 placed it at `.claude/tools/test_plan_memo_vm_p4_family_layout.py`; there the
  checker's own `--self-test` goes red (R2, CRIT), and the reason that red is **right** decides the
  placement (next bullet).
- **The control, and why the guard is outside its subject.** `registry_membership_control`
  (`plan_memo_selftest_population.py:500`) states its property as: no module can leave a run
  unnoticed — every `.py` file beside the checker that the population imports, or that imports it, is
  in the population, and every population module is reached from a root (the entry point and the
  registry modules). Its probes, listed in the same docstring, are all **checker code the self-test
  would not run**: a helper a population module imports, a cases module nothing imports, a new empty
  checker file, a self-test-named helper. The registry's threat model (`plan_memo_selftest_registry.py`
  module docstring, "THE THREAT MODEL") is the same subject seen from the row side: a row, case or
  control that is never collected. "Beside the checker" is how the control recognises checker code:
  `_membership_of` reads `here.glob("*.py")` (`:487`), the checker's own directory, non-recursive, and
  a file there is presumed part of the checker. A file there that imports the population and is not
  in it is therefore reported, and for a family guard the report is correct — it would be a
  family-specific consumer sitting where checker code sits.
  The guard is not checker code, in each of the three senses the control and the threat model guard:
  (1) the population does not import it, and it changes no checker output — it reads `Population`
  attributes after construction and mutates nothing (C6 × C9: the checker output is byte-identical
  with it present); (2) it adds no row, case or control, so the self-test's manifest is untouched;
  (3) its own run cannot go unnoticed either: it is a member of `scripts/python-suites.sh`'s set
  (step 2, repo-wide `**/test_*.py`), it runs as its own root `docs/plans` (step 4), and step 5 fails
  the job unless it contributes ≥ 1 passing test. So the property the control protects still holds
  with the guard in the tree: nothing that takes part in a checker run is outside the self-test, and
  the guard's own run is guaranteed by the suite driver. Placing it outside the directory is not an
  escape from the control; placing it inside was a category error the control caught.
- **Rejected placements.** (b) *A population member*: membership needs a `plan_memo*.py` name
  (`plan_memo_selftest_harness.py:81` `GLOB`) and a path from a root, so the self-test would have to
  import it — this family's layout would become part of the checker's code, its manifest and its
  mutation proof, which is one-home re-introduced inside the checker, the boundary rule C draws. It
  also cannot be a `tools` set member under that name (`test_*.py` only), so it would need a wrapper;
  and CLAUDE.md's "lexer / population / assertion を触ったら control + mutant を同 commit で足す" would
  then ask for a mutant against a module that asserts nothing about checker behaviour — a mutant
  with no subject. (c) *A subdirectory of `.claude/tools/`*: the non-recursive glob would not see it,
  but it would sit inside the checker's tree and read as checker code; that is the confusion the
  control exists to prevent, so it is rejected even though the control would not fire.
- **Precedents for the form.** `.claude/tools/_webref/test_*.py` import their tool's modules from
  outside the tool's directory by `sys.path.insert(0, str(Path(__file__).resolve().parents[1]))`; and
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh` is a tracked,
  non-Markdown check living beside the plan family it checks.
- **Import form.** `TOOLS = normpath(join(HERE, "..", "..", ".claude", "tools"))`, inserted at
  `sys.path[0]` if absent, then `from plan_memo_population import Population` and
  `from plan_memo_tables import SCHEMAS` — the checker's own modules as a library, no parser of its
  own. The entry is `join(HERE, ENTRY)`, not cwd-relative. Step 4 runs the root `docs/plans` in its
  own interpreter (`sys.path[0]` = the root, cwd = the repository root) and no other set member has
  that dirname, so the checker modules are imported into no other member's process. It is not an
  entry-point wrapper (no subprocess, no `--self-test`), so step 3(b)'s naming clause does not apply;
  it reads only local files (offline regime). A checker refactor that renames what it reads
  (`Population.misses` / `.memos` / `.display`, `Memo.tables`, `Table.schema`, `SCHEMAS`, a schema's
  `.name`) turns the guard red in the `tools` job: the library dependency surfaces loudly, and that
  refactor's PR updates the guard.
- **Shape** (the production text is the scratch file
  `vm-p4-artifacts/split-sim/r2-guard/test_vm_p4_family_layout.py`, which commit (2) adds as is):
  one `unittest.TestCase`, `VmP4FamilyLayout`.
  - `setUpClass` builds one `Population` on the entry;
  - `setUp` asserts `self.p.misses == []` — a precondition of **every** test, not a test of its own
    that could fail while the others pass over an unscanned population;
  - **(i)** `test_population_is_the_family`: `{p.display(m.path) for m in p.memos}` equals the pinned
    set of nine names — the entry, the five new children, the three existing carve-outs. A **set**, not
    the BFS list: first-link order moves with ordinary prose edits;
  - **(ii)** `test_each_schema_has_one_home`: the pinned keys equal `{s.name for s in SCHEMAS}`
    (`plan_memo_tables.SCHEMAS`), so a fifth schema forces the pin to be revisited; and the map schema
    name → list of `p.display(m.path)`, one entry per table whose `t.schema` is that schema, equals
    `{"citation": [ENTRY], "stub": [ENTRY], "slice": ["2026-07-vm-p4-slice-plan.md"], "slot":
    ["2026-07-vm-p4-slot-ledger.md"]}`. One list of length one per schema pins both "exactly one
    table" and "in which memo". Row counts are **not** pinned.
- **When it goes red.** (i) moves when a memo enters or leaves the population. That includes an edit
  that looks like content: a relative link from any member to a `.md` outside the family brings that
  memo, and whatever it links, into the population — a structural change, and treated as one
  (measured below: one such link turns (i) red with the checker at rc 0). (ii) moves when a schema
  table moves to, or appears in, another memo, or a header drifts off its schema. Neither moves for
  prose, for rows added or removed inside a schema table, for links among the nine members, or for a
  link whose destination is not a relative `.md`. Separately, the `setUp` precondition makes **every**
  test red on **any** checker miss, content defects included — a row of the wrong width, an
  unresolved reference — not only on header drift; so for this family a memo defect the checker
  reports at rc 2 is also a red `tools` job (stated in CLAUDE.md:54 by commit (2), §3).
- **Docstring** (required content; the scratch file carries it): (a) this test does **not** restate
  or tighten the checker's presence rule (rule C: ≥ 1 memo, ≥ 2 allowed); it pins this family's
  layout. (b) *A pin list of length one is this family's guarantee that a header drifting off its
  schema is rc 2 (the schema is then in no memo); updating a pin to a list of length ≥ 2 gives that
  guarantee up for that schema* — under rule C the drifted table's rows would leave the census at
  rc 0 unless one of them carries a kind phrase (§1.3). (c) *When a structural change moves a schema
  table to another memo, adds a carve-out, or otherwise changes the layout, update this test in the
  same commit — that update is the intended change, not a workaround.* (d) It lives beside the family
  as a consumer of the checker's library, outside the checker self-test's membership rule, and runs
  as its own suite root. Without (a) and (c) the test would read as one-home re-introduced outside
  the checker; without (b) a reader updating a pin would not see what the update costs.
- **Measured, in production form.** A `git clone --local` of `vm-p4-plan-doc` at `08a2a68c`
  (`vm-p4-artifacts/split-sim/r2-repo`); commit (1) simulated as a real commit (`sim2.py <repo> 1`),
  commit (2) as a real commit on top (`r2-apply-c2.py <repo> r2-guard/test_vm_p4_family_layout.py`:
  every commit-(2) edit of §3, the CLAUDE.md drafts included, and the guard at its path). Hashes in
  `vm-p4-artifacts/split-sim/r2-commits.txt`.
  1. checker `--self-test` at c2 (`python3 .claude/tools/plan-memo-umbrella-check.py --self-test`):
     rc 0, `954 control(s)`, `all controls behaved as declared.`
  2. `bash scripts/python-suites.sh` at c2: rc 0, `GREEN -- 10 member(s), 120 test(s) passed,
     offline`; `root docs/plans (1 member(s))`, `docs/plans/test_vm_p4_family_layout.py: 2 test(s)
     passed`.
  3. The guard (`python3 -m unittest docs/plans/test_vm_p4_family_layout.py`): green at c2; red on the
     pre-split tree, both tests (population 4 memos; `slice` and `slot` at the entry) — which is why
     it lands in commit (2), after the move; the §4 stub's link alone deleted → green (the relinked
     `§4` still reaches the child, §1.3); that relink deleted too → checker rc 0, (i) red with exactly
     `design-invariants.md`; one link from `open-decisions.md` to the checker plan → checker rc 0, (i)
     red with `2026-08-plan-memo-umbrella-checker.md` and `…-rounds.md`; a stub-table row given a
     seventh cell → checker rc 2, both tests red through `setUp`.
  4. Negative control for the placement: the same file at `.claude/tools/test_vm_p4_family_layout.py`
     → `--self-test` rc 1, `FAIL [CONTROL] PROPERTY: no module can leave a run unnoticed … 1
     violation(s); test_vm_p4_family_layout.py imports the population but is not in it` — R2's CRIT,
     reproduced.
  Re-run: `vm-p4-artifacts/split-sim/r2-measure.sh <fresh clone> pre|crit|split` for 3 (pre) and 4,
  and for the c1/c2 heads the commands of §3.
- **Scope note for the split PR**: this puts tools code (a Python test) into #506, a plan PR. The
  split PR's description says so, and the `/pre-push` run for the split covers it: `mise run
  tools-test` / `mise run ci` run `scripts/python-suites.sh` once, under the first `python3` on `PATH`
  (its header, step 1). The `"3.9"` / `"3.x"` matrix exists only in GitHub's `tools` job, so the
  3.9 floor is measured by CI, not by `/pre-push`; the guard uses no syntax newer than 3.9.

### §1.6 Rejected: keep all four tables in the entry memo (no S0)

Entry = §0.5-§3 + §5's table block + §7 + §8's table block (~940 lines); conventions/evidence in
children. It needs no tools change, but it **reverses** the 2291 verdict by separating the table from
the specification its cells defer to, and splits §8 from its own rules. Cost-driven, so rejected.
(Moot since S0 landed; kept as the record of why the split waited on a tools change.)

## §2. Coupled invariants, and the measured checker diff

### §2.0 The invariants and their intersections

The split satisfies ten invariants at once:

- **C1 byte-exact move** — each child's body = its range; entry = kept lines + stub insertions (§2.1).
- **C2 entry relink** — 59 bare `§4/§5/§8/§9` in the entry become links (§3); 2318's `§5` →
  `touch-time-split-record.md`.
- **C3 no new naming site** — stub and head text name no declared id (§1.2, §1.4).
- **C4 population membership** — every child is a population member, and commit (1) gives each new
  child an edge that does not depend on incidental prose (its stub link). The guard keeps the first
  half afterwards (§1.5 (i)); the second half is a commit-time property of the stub form (§1.2,
  checked in §3 step 1), which the guard does not keep — it pins membership, not which edge provides it.
- **C5 schema home** — each of the four schemas has one table in one memo (§1.3).
- **C6 predicted checker diff** — §2.2's classes and nothing else, at both commit heads.
- **C7 family order** — `above`/`below` and section order read across files as before.
- **C8 pointer commit (2)** — three existing stub links, three existing head re-points, the
  checker-plan anchor, CLAUDE.md:54 and :55 (at `08a2a68c`), the guard (§3).
- **C9 family guard** — the `setUp` precondition, (i) and (ii) of §1.5.
- **C10 base revision** — the umbrella in commit (1)'s parent is the `39cc365a` blob
  (`git rev-parse <c1>^:<entry>` = `git rev-parse 39cc365a:<entry>`). Every line number, range and
  count in this plan, and the heads' "moved verbatim at `39cc365a`", are statements about that blob;
  if #506 edits the umbrella before the split, the plan's ranges are re-derived first, not applied.

| Pair | Intersection |
|---|---|
| C1 × C2 | the relink edits kept lines; §2.1 undoes exactly the relink form (`[§N](<new file>)`) before diffing, so a relink is the one edit to a kept line commit (1) allows |
| C1 × C3 | heads and stubs are outside the byte proof (head = text before the range; stubs = inserted chunks containing `→ **carved out**`), so C3 is checked by the checker output, not by §2.1 |
| C1 × C4 | stub links are in inserted chunks, so they pass §2.1 as stub-only insertions |
| C1 × C5 | the four schema tables are in moved or kept ranges, so §2.1 proves each moved byte-for-byte; header drift by the move is impossible |
| C1 × C6 | class 2 of §2.2 exists because a child's line numbers are its range shifted by the head length; §2.2 maps them back (`a + N − h − 1`) |
| C1 × C7 | the bodies are verbatim, so deixes keep their text; order is restored by stub order, not by editing them |
| C1 × C8 | commit (2) edits three **kept** entry lines (2317, 3079, 3089), so §2.1 is run with `<c1>` only (given `<c2>`, it stops at the C10 assertion, since `<c2>^` is `<c1>` — measured), and commit (2) has its own kept-line check (§3 step 2) |
| C1 × C9 | none: the guard reads structure, not bytes |
| C1 × C10 | §2.1 reads `old` from `<c1>^` and asserts first that it is the `39cc365a` blob; C1 is defined against that blob |
| C2 × C3 | a relink rewrites existing text into a link whose label is `§N`, never an id; class 1 of §2.2 is excerpt text only |
| C2 × C4 | each relink is an edge; it is not relied on for C4's second half because an incidental reference can be edited away — but it does count for membership, which is why (i) stays green when only a stub link goes (§1.5) |
| C2 × C5 | relinks edit cells of the entry's `stub` table (e.g. 439-475), never a header or delimiter row, so the table still matches its schema (measured: every count identical) |
| C2 × C6 | class 1 of §2.2: 11 entry lines whose excerpt now shows a link |
| C2 × C7 | none: deixes are untouched, links do not reorder text |
| C2 × C8 | commit (2) adds no relink; its entry edits are the three stub links only |
| C2 × C9 | relinks add edges among members only, so (i) is unchanged |
| C2 × C10 | the relink census (§3 step 1) compares `<c1>` with `<c1>^`: 59 links on 52 kept lines, per target as in §3 |
| C3 × C4 | the stub link's label is a section label, so the edge it adds names nothing (zero inserted lines in the output) |
| C3 × C5 | heads and stubs contain no table, so no schema table is added |
| C3 × C6 | zero head/stub lines in the output (measured: no checker line maps to an inserted line) |
| C3 × C7 | the heads state the family-order reading rule; the rule is text, not an id |
| C3 × C8 | the head re-points and stub links in commit (2) name only file paths, section labels and a revision |
| C3 × C9 | none |
| C3 × C10 | each head says its body moved verbatim at `39cc365a`; C10 is what makes that sentence true |
| C4 × C5 | `slice-plan.md` / `slot-ledger.md` leaving the population is rc 2 by presence; the other five leaving is rc 0 — the asymmetry (ii) and (i) are split along |
| C4 × C6 | the population header line changes (four → nine memos, BFS order) — class 0 of §2.2 |
| C4 × C7 | stub order = original section order = family order |
| C4 × C8 | the three existing stub links add edges to files already reached earlier in BFS; measured: population order and the whole output unchanged between the two heads |
| C4 × C9 | (i) is what makes a child **leaving** red at checker rc 0; it is silent while any edge still reaches the child (a lost stub link with a surviving relink, measured), so it keeps C4's first half only |
| C4 × C10 | none: membership is read at the heads |
| C5 × C6 | rows keep their file attribution: the 64 §5 rows read from `slice-plan.md`, the 52 §8 rows from `slot-ledger.md`; every count identical |
| C5 × C7 | none |
| C5 × C8 | commit (2) adds no table |
| C5 × C9 | (ii) pins the one-home layout that keeps drift at rc 2 under rule C; for `citation` (no kind-phrase row) and, one row-edit away, `stub`, nothing else does (§1.3) |
| C5 × C10 | the row counts (29 / 55 / 64 / 52) and the tables' ranges are those of the `39cc365a` blob |
| C6 × C7 | none: the checker reads no deixis |
| C6 × C8 | commit (2) must not move any output: measured, the commit-(2) output is **byte-identical** to the commit-(1) output |
| C6 × C9 | the guard is a separate test outside the checker's directory; it adds nothing to the checker output and nothing to its self-test (§1.5) |
| C6 × C10 | the baseline is the output on the `39cc365a` blob; if a checker file changed between `a45a42d1` and `<c1>^`, the baseline is re-taken at `<c1>^` before comparing (§2.2) |
| C7 × C8 | the head re-point names a file; it moves no text |
| C7 × C9 | none |
| C7 × C10 | none |
| C8 × C9 | the guard lands in commit (2), after the move, because at commit (1)'s parent it is red by design |
| C8 × C10 | commit (2)'s checks compare `<c2>` with `<c1>`, not with `<c1>^`; the checker-plan anchor it writes ("line 2716 at `39cc365a`") is true only under C10 |
| C9 × C10 | none: the guard reads the head |

### §2.1 Content-free proof (commit (1))

Byte-exact: each child's body is its range; the entry, with the new links undone, is the kept lines
plus stub-only insertions. Everything is read from revisions, not from the working tree: `old` from
`<c1>^`, after asserting C10; the bodies and the entry from `<c1>`. Run from the repository root as
`python3 proof.py <c1>`.

```python
import difflib, re, subprocess, sys
C1 = sys.argv[1]
D, X = 'docs/plans', '2026-07-vm-p4-'; U = X + 'es-language-completeness.md'
M = [('design-invariants', 698, 1006), ('slice-plan', 1007, 2123), ('touch-time-split-record', 2124, 2314),
     ('slot-ledger', 2398, 2786), ('open-decisions', 2787, 3076)]
git = lambda *a: subprocess.run(['git', *a], capture_output=True, text=True, check=True).stdout
assert git('rev-parse', f'{C1}^:{D}/{U}') == git('rev-parse', f'39cc365a:{D}/{U}'), 'C10: <c1>^ umbrella != 39cc365a'
show = lambda r, f: git('show', f'{r}:{D}/{f}').split('\n')[:-1]
old = show(C1 + '^', U)
moved = set()
for name, a, b in M:
    body = show(C1, f'{X}{name}.md')
    assert body[-(b - a + 1):] == old[a - 1:b], name
    moved |= set(range(a, b + 1))
kept = [l for i, l in enumerate(old, 1) if i not in moved]
link = re.compile(r'\[(§[4589])\]\(' + X + '(?:' + '|'.join(n for n, _, _ in M) + r')\.md\)')
par = [link.sub(r'\1', l) for l in show(C1, U)]
bad = [o for o in difflib.SequenceMatcher(None, kept, par, autojunk=False).get_opcodes() if o[0] != 'equal'
       and (o[0] != 'insert' or not any('→ **carved out**' in l for l in par[o[3]:o[4]]))]
print('base 39cc365a ok; moved', len(moved), 'kept', len(kept), 'non-stub diffs', len(bad)); sys.exit(1 if bad else 0)
```

Expected at commit (1): `base 39cc365a ok; moved 2296 kept 797 non-stub diffs 0`. **Measured** in the
production-form clone of §1.5 (`vm-p4-artifacts/split-sim/r2-proof-c1.py`, the text above): that line,
rc 0, at the simulated `<c1>`. Negative controls, each a commit on `08a2a68c` with the commit-(1)
split plus one appended character: in the entry (line 100) → `non-stub diffs 1`, rc 1; in a child
(`slot-ledger.md` line 200) → `AssertionError: slot-ledger`, rc 1; and the C10 control — the script
given the simulated `<c2>` → `AssertionError: C10: <c1>^ umbrella != 39cc365a`.

### §2.2 Checker output: baseline, commit (1), commit (2)

**Baseline** (`39cc365a`, saved: `vm-p4-artifacts/p506_check_pre.txt`, 868 lines; `a45a42d1` gives a
byte-identical output): rc **0**; `[UMBRELLA-MARK] 48` (`slice=33, slot=15`); `[CENSUS] 53`;
`[KIND-UNDETERMINED] 5`; `[UMBRELLA-CELL] 37`; footer `51 seed(s) and 715 reported naming site(s)`;
population = the three existing carve-outs.

**If the checker moved before the split.** The saved baseline is valid only for the checker at
`a45a42d1` (= `e44ffce6` for `.claude/tools/`). Before comparing, check
`git diff --quiet a45a42d1 <c1>^ -- .claude/tools/`; if it reports a difference, re-take the baseline
on `<c1>^` with the checker at `<c1>^`
(`git checkout <c1>^ && python3 .claude/tools/plan-memo-umbrella-check.py $M > base.txt`), re-run the
§2.2 prediction against `base.txt`, and report any class the new checker adds; do not compare against
`p506_check_pre.txt`.

**Prediction, measured on a scratch split of both heads** (`vm-p4-artifacts/split-sim/sim2.py
<dir> 1|2`, checker at `a45a42d1` = S0 landed; outputs `sim2_after_c1.txt` / `sim2_after_c2.txt`;
`norm2.py` maps each `file:N` back to its `39cc365a` line). The scratch split is this plan's layout:
§1.1 ranges, §1.4-shaped heads (12 lines, 14 for the touch-time file), the stub form of §1.2 in
original section order, 2318's `§5` linked to `touch-time-split-record.md`, and at stage 2 the three
existing stub links and head re-points. (The earlier `simulate.py` linked 2318 to `slice-plan.md`,
used 4-line heads, and put all five stubs before §6; `sim2.py` corrects all three.) Re-measured at
R2 on the production-form clone (§1.5), where the two heads are real commits and commit (2) carries
every edit of §3 step 2 including the CLAUDE.md drafts: the `<c1>` output is byte-identical to
`sim2_after_c1.txt`, and the `<c2>` output byte-identical to the `<c1>` output.

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
git diff --quiet a45a42d1 <c1>^ -- .claude/tools/ || echo 'checker moved: re-take the baseline at <c1>^'
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
| carve heads of the 3 existing siblings ("the umbrella's §5", line 5 of each) | 3 | commit (2): insert, right after `in the umbrella's §5` and before the closing parenthesis, same line, no reflow: `` ; the umbrella's second split (at `39cc365a`) moved that passage to `docs/plans/2026-07-vm-p4-touch-time-split-record.md`, and "pre-split" below still means before the first split ``. Each head says its body is "stated about the pre-split umbrella" (line 7 of each), meaning before **its** split; the insertion names the second split and its revision so the two cannot be read as one. **Backticked**, not linked — membership does not need it, and a link would add an edge for navigation only |
| other docs: line cites into the umbrella | 3, all in `docs/plans/2026-08-plan-memo-umbrella-checker.md` at `a45a42d1` (file identical to `e44ffce6`): `:663` "main memo line 2716"; `:1098` `` `190d2adb:…:1218` `` / `` `:2711` ``; `:1103` `` `190d2adb:…:2710` `` | commit (2): anchor `:663` as "line 2716 at `39cc365a`"; `:1098` / `:1103` are already anchored to `190d2adb` and stay. Complement measured (below). |
| CLAUDE.md:54 and :55 at `08a2a68c` (same text at `a45a42d1`; branch `vm-p4-plan-doc` — on main `e44ffce6` line 55 is the **Git** rule, the checker-run rule is added on this branch) | 2 | commit (2): :54 states the one exception to "memo 実行は手動 … CI job に入れない" — this family's guard runs in the `tools` job, and through its `setUp` precondition any rc-2 defect of a family memo makes that job red; :55 extends the trigger to every family memo and adds the guard-update rule. Insertions only, no reflow; drafts below |

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

**CLAUDE.md drafts for commit (2)** (insertions into the two lines; nothing on either line is deleted —
§3 step 2 checks that). On :54, after `` (`scripts/trip-wires.sh` の `REQUIRED_WIRES` には登録しない)。 ``:

> 例外は VM-P4 family (`docs/plans/2026-07-vm-p4-*.md`) で、その layout guard `docs/plans/test_vm_p4_family_layout.py` が checker を library として import し、population と schema table の置き場所を pin する test として同じ `tools` job で走る (memo 実行そのものではなく、checker の presence rule を締めるものでもない)。guard の前提 `misses == []` により、family の memo に checker が rc 2 を出す欠陥があれば (header drift に限らず、行幅・未解決 reference 等の内容欠陥も) `tools` job が赤になる。memo 編集で `tools` job が赤になる経路は、上記 `symbol_attribution_control` (全 `docs/plans/*.md`) とこの guard (VM-P4 family のみ) の 2 つ。

On :55, after `` `docs/plans/2026-07-vm-p4-es-language-completeness.md` を編集したら ``:

> (carve-out された family memo `docs/plans/2026-07-vm-p4-*.md` のどれを編集した場合も、entry であるこの memo に対して)

and at the end of :55, after `未走査 (clean ではない)。`:

> family の構造を変える編集 (memo の追加・削除、schema table の別 memo への移動・追加、family 外 `.md` への相対リンク追加) は `docs/plans/test_vm_p4_family_layout.py` の pin を同 commit で更新する。各 schema の pin が長さ 1 の list であることが、その schema の header drift を rc 2 にする保証そのもので、長さ 2 以上に更新するとその schema についてこの保証を手放す。

"The 2 routes" on :54 is measured, not asserted: the tracked `.py` files that read `docs/plans` at run
time are found by `git grep -nE "docs/plans|['\"]plans['\"]" -- '*.py'` at `08a2a68c` — every hit is a
docstring or comment except `plan_memo_selftest_records.py:304` (`_attribution_corpus`, which
`symbol_attribution_control` reads; it globs `*.md`, so the guard's `.py` beside the memos does not
enter it); the guard adds the second. The other set members that touch a `docs` path
(`.claude/tools/_webref/test_*.py`) build it under a temporary root.

**Commit order in the split PR.**

1. **Content-free commit**: the five children (heads per §1.4), the entry's 59 relinks, the four new
   stubs. Checks, all from the repository root:
   - C10 and C1: §2.1's script with `<c1>` → `base 39cc365a ok; moved 2296 kept 797 non-stub diffs 0`;
   - C2, the relink census at the real head — every `[§N](<child>.md)` on a **kept** line, as
     (`<c1>^` line, label, target), counted against §3's row:

     ```sh
     U=docs/plans/2026-07-vm-p4-es-language-completeness.md
     git show <c1>^:$U > old.md && git show <c1>:$U > new.md
     python3 relink_census.py old.md new.md
     ```

     where `relink_census.py` is `vm-p4-artifacts/split-sim/r2-relink-census.py` (§2.1's link regex
     and kept/moved partition; it maps each `equal` block of the kept-vs-undone diff back to its
     `<c1>^` line and lists the link tokens there). Expected — measured on the production-form clone:
     `relinks 59 lines 52`; `§4 -> design-invariants 1`, `§5 -> slice-plan 25`,
     `§5 -> touch-time-split-record 1` (line `[2318]`), `§8 -> slot-ledger 18`, `§9 -> open-decisions 14`;
   - C4's second half: each of the four new stubs carries its link(s) in the §1.2 form (five links);
   - C6: §2.2's commit-(1) prediction holds (after the baseline check of §2.2).
2. **Pointer commit**: the three existing stub links (§1.2), the three existing head re-points, the
   checker-plan `:663` anchor, CLAUDE.md:54 and :55 (drafts above), and the guard
   `docs/plans/test_vm_p4_family_layout.py` (§1.5). Checks:
   - `git diff --numstat <c1> <c2>` shows exactly: the entry `3 3`, each of the three carve-outs
     `1 1`, the checker plan `1 1`, CLAUDE.md `2 2`, `docs/plans/test_vm_p4_family_layout.py` `N 0`
     (80 for the scratch text), nothing else (measured on the production-form clone);
   - every changed line is its `<c1>` line plus the specified insertion and nothing else:
     `python3 c2_edits.py <c1> <c2>` (`vm-p4-artifacts/split-sim/r2-c2-edits.py`) asserts the set of
     changed files, an unchanged line count per file, and for each changed line of the entry, the
     carve-outs and the checker plan that it equals `old.replace(anchor, anchor + insertion, 1)` for
     exactly one of the specified (anchor, insertion) pairs, each pair used once — the real-head form
     of `sim2.py`'s `assert p in E[j]`; for CLAUDE.md (free text) it asserts two changed lines whose
     character diff is insertions only, and prints the insertions for review against the drafts.
     Measured on the production-form clone, verified 2026-10-31: `c2 edits ok: 6 files edited + 1 new`;
   - the checker output at `<c2>` byte-identical to `<c1>`'s (§2.2);
   - the guard green (`python3 -m unittest docs/plans/test_vm_p4_family_layout.py`);
   - checker `--self-test`: `all controls behaved as declared.` (the guard is outside the checker's
     directory, so `registry_membership_control` stays green — §1.5);
   - `mise run tools-test` green, with `root docs/plans (1 member(s))` in its output.

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

1. **The guard's placement against the checker's membership control (top focus, R2 CRIT).** §1.5
   moves the guard from `.claude/tools/` to `docs/plans/test_vm_p4_family_layout.py` and argues that
   this is not a way around `registry_membership_control`: the control's subject is checker code the
   self-test would not run, recognised by sitting in the checker's directory; the guard is a consumer
   that the population does not import, that changes no checker output and no self-test row, and
   whose own run the suite driver guarantees (steps 2, 4, 5). Review that argument against the
   control's docstring and the registry's threat model, the two rejected placements ((b) a population
   member, (c) a subdirectory of `.claude/tools/`), and the production-form measurements (self-test
   all as declared, `python-suites` GREEN with the `docs/plans` root, the R1 placement reproduced
   red).
2. The S0 checker contract is **settled** by PM judgments 52, 61 and 70 and by #562; not in scope.
   In scope: the rest of the guard (§1.5) as the thing that keeps §1.3's "drift is rc 2" and §1.2's
   "every child is a member" true after the split — the `setUp` precondition and the two assertions,
   which edits move them (a link out of the family is structural; any checker miss is red), the
   length-one pin as the drift guarantee and the docstring's statement of what a longer pin gives up,
   and §1.3's per-schema scope of the silent case (`citation` silent, `stub` one row away).
3. §2.0 — whether the ten invariants are the right set (C10 base revision is new at R2) and each
   intersection line is right; in particular C1 × C8 and C6 × C8 (commit (2) edits kept lines and
   must not move the output), C4 × C9 (the guard keeps membership, not the stub edge), and the
   CLAUDE.md:54 exception drafted in §3.
4. §1.2 — the one stub form, applied to the three existing stubs in commit (2).
5. §1.2 — the cohesive-unit exemption for `slice-plan.md` (1117), against the three candidate cuts.
6. §1.2 — is the 2124-2314 tail a real seam, given it closes with the `Intl`/`Atomics` paragraph
   (2306-2311) that is about §8 rather than file size?
7. §3 — the "entry links, carve-outs stay verbatim" rule; whether 30 moved `<this file>` commands are
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
the ones the checker plan cites, per row, at `a45a42d1` (file identical at `08a2a68c`): §6.3 Links
`:409`, `:411`, `:573`; §4.7 Link reference definitions `:412`, `:505`; §6.1 Code spans `:413`, `:571`;
§4.2 ATX headings `:500`; §6.5 Autolinks `:575`; GFM §4.10 Tables `:415-417`, `:510`; the WHATWG URL
row `:434`. Webref has no CommonMark or GFM
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
| CommonMark §4.7 Link reference definitions | definition | the umbrella at `39cc365a` has **0** definitions, so no reference link resolves in it and none can cross a file boundary (a definition is matched within its own document): `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md \| /usr/bin/grep -cE '^ {0,3}\[[^]]+\]:'` → 0 (the `][` sequences — 11 lines, 14 occurrences: `/usr/bin/grep -c '\]\['` → 11, `/usr/bin/grep -o '\]\[' \| wc -l` → 14 — are all inside code spans or fences) | — | ✓ | no |
| CommonMark §6.5 Autolinks | `<…>` URI autolink | an autolink's URI is absolute — a scheme of 2-32 characters, then `:` — so a relative `.md` destination can never be one. Measured in that form: same blob `\| /usr/bin/grep -cE '<[A-Za-z][A-Za-z0-9.+-]{1,31}:[^<> ]*>'` → 0, and the simulated heads and stubs (§2.2) and the relinks add no `<`. Independently, the checker lexes an autolink and masks it whole (checker plan `:575`, row §6.5), so even one would add no edge | — | ✓ | no |
| CommonMark §6.1 Code spans | relink exclusion | a `§N` inside a code span stays verbatim (4, all in PLAN); a backticked path is a code span, not a link | §3 relink rule; §2.1's link-undo regex reverses exactly the relink form | ✓ | no |
| CommonMark §4.2 ATX headings | heading line, then **local policy** | a heading's text is inline content to CommonMark; that a `§N` in a heading line is **not relinked** is §3's own count rule (local policy), not a consequence of §6.1 | §3 | ✓ | no |
| GFM §4.10 Tables | recognition (header row + delimiter row) | the four schema tables move verbatim, so each header still matches its `SCHEMAS` entry; a drifted header makes a non-schema table (#562 Known remainder (ii)), which is rc 2 only through presence or an unbound kind claim (§1.3) | `plan_memo_tables.SCHEMAS`, `Memo.tables`; guard (ii) | ✓ four tables, one memo each (§1.3) | no |
| GFM §4.10 Tables | cell content | the cells that name rows of the other table (§4: 43 §5 `Slot` cells, 24 §8 `Trigger` cells) are not edited; no per-cell link | §4 | ✓ | no |


This memo's own checks are not spec rows and are not counted in K or M:

- **§2.1 byte proof** (commit (1)): each child = its range; the entry, relinks undone = kept lines +
  stub-only insertions; negative controls on both sides.
- **§2.2 checker output** (both heads): rc, the four counts and the footer compared; commit (2)
  byte-identical to commit (1).
- **§1.5 family guard**: precondition `misses == []` in `setUp`; (i) population = the nine memos;
  (ii) one table per schema, in the pinned memo, with the `SCHEMAS` keys pinned.
- **§3 step 1 / step 2 checks**: the relink census (59 on 52 lines, per target) and the commit-(2)
  kept-line insertion check.

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
