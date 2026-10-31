# VM P4 umbrella memo — seam split plan (`#11-vm-p4-memo-section-seam-split`)

**Status**: plan, revised after `/elidex-plan-review` R1, R2 and R3 (R3's MINs, then the kept-text
statements the split falsifies, §3). **Subject**:
`docs/plans/2026-07-vm-p4-es-language-completeness.md` at `39cc365a` (3093 lines). **Rule**: CLAUDE.md
*1000-line debt = touch-time split* — a cohesion judgment, executed inside #506 as a content-free
commit (1) plus a pointer commit (2) before #506 next edits the memo's content.
**Prerequisite S0 (§1.3) has landed**: PR #562, squash `e44ffce6` (2026-10-31).

**Revisions (defined once; the body names them only where it must).** `39cc365a` — the umbrella blob:
every umbrella line number, range and count below is a line of `git show 39cc365a:<memo>` (C10).
`e44ffce6` — S0's squash: line numbers into `.claude/tools/`, the checker plan and
`.github/workflows/ci.yml` are at it (all three unchanged on this branch). `08a2a68c` — CLAUDE.md
:54/:55 (:55 exists only on this branch) and the base of the production-form clone
(`vm-p4-artifacts/split-sim/r2-repo`; the commits measured in it are listed in
`vm-p4-artifacts/split-sim/r3/r4-commits.txt` — R3's `<c1>`, and a `<c2>` rebuilt from the current
edit spec; R3's own `<c2>` is in `r3/r3-commits.txt`).

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
  this form, so five links in four new stubs. **Each new stub is exactly six lines**, which §2.1
  compares byte for byte: the moved section's own heading line at `39cc365a` with ` → **carved out**`
  appended; a blank line; the one body line
  ``Moved **verbatim** to `docs/plans/2026-07-vm-p4-<f>.md` ([§N](2026-07-vm-p4-<f>.md)); sub-headings keep their numbers there.``
  (for §5: the `slice-plan` reference, then `, and its 1000-line touch-time check to ` and the
  `touch-time-split-record` reference, before the `;`); a blank line; `---`; a blank line. The three
  **existing** stubs (2317, 3079, 3089) get the same parenthesised link inserted after their backticked path, on the same line, without reflow
  (pointer commit (2), §3). Before this revision they had none: their files' edges came only from
  incidental links in kept text (`slice-0a-landing-record.md` 4, `slice-1a-1b-call-spread-detail.md`
  5, `umbrella-review-rounds.md` 2; the §6 stub's own one link is to `umbrella-review-rounds.md`), so
  the family would have kept two stub forms, and a reader or a later carve would have to ask which one
  to write. Stub and carve-head text **name no declared id** (no row id, no `#11-` slug, no `I-n`), so
  they add no naming site (§2: zero inserted lines in the checker output).
- **Why the stub carries a link at all.** The relinked `§N` references alone give
  `design-invariants.md` **no** edge (the entry's one bare `§4`, at 7, is not the umbrella's — §3) and
  `touch-time-split-record.md` one (2318, itself inside the §6 stub), and those two files hold no
  schema table: without the stub link they would be outside the population **at rc 0** — the exit
  status does not see it. The stub link gives every child an edge that does not depend on incidental
  prose. What makes a child **leaving** the population red is the
  family guard (§1.5 (i)); without the guard a child could still leave it silently. The guard pins
  membership, not the stub edge itself (§2.0 C4).

### §1.3 Prerequisite S0 — schema presence over the population (**landed**: #562, `e44ffce6`, 2026-10-31)

**The landed contract** (PM judgment 52, rule C; 61; the cost accepted in 70). `matched` is built from
every memo of the population (`plan_memo_population.py:108`, under a comment stating the
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
(`plan_memo_population.py:174`): a table bound to **no** schema that has a cell carrying
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
without a schema table). **Measured** on the commit-(2) scratch split with the checker as landed
(`python3 vm-p4-artifacts/split-sim/r2-drift.py vm-p4-artifacts/split-sim/r2-drift/base`, the split
made by R2's `sim2.py … 2`, which differs from the production form only by 7's `§4` link — no table
cell): "rows with a kind phrase" asks `Population._claims` of every cell, header
included — the predicate `_unbound_claims` applies; "drift" prefixes the first header cell with `Q`;
"second home added" also appends the same header and delimiter row (no body) to
`slice-0a-landing-record.md`, so presence is met elsewhere. So under rule C the silent case is today
exactly `citation`, and `stub` is one edited row away from it (its one kind-phrase cell, :471). The
one-table layout is what keeps all four at rc 2, and only while it holds: a later edit that puts a
second table of one schema in another memo opens the silent case for `citation` (and for any table
whose kind-phrase rows are later edited away), and nothing in the checker would say so. The family
guard (§1.5, assertion (ii)) is what turns that edit red. The guarantee has two layers, and the pin
length decides only the first. **Checker layer**: while a schema's pin has length one, a drifted
header leaves the schema in no memo, so the checker itself exits 2 — a manual run shows it. **CI
layer**: the `tools` job is red on that drift at **any** pin length — at length one through the
guard's `setUp` precondition (the presence miss), at length ≥ 2 through (ii), because the drifted
table drops out of the observed homes, which then no longer equal the pin (compared as sorted lists,
§1.5). Updating a pin to length ≥ 2 therefore gives up the checker-layer signal for that schema (the
checker can then exit 0, as the table above shows), not the red `tools` job; the guard's docstring
says so (§1.5).

**A child the entry forgets to link.** `slice-plan.md` and `slot-ledger.md` hold the only `slice` and
`slot` tables, so losing their edge is rc 2. Losing the edge to one of the five children without a
schema table drops it from the population at rc 0, and only naming-site counts move. The guard's
assertion (i) catches it — when the child actually leaves. Measured at the production-form `<c2>`
(§1.5): deleting only the §4 stub's link — `design-invariants.md`'s one edge — leaves the checker at
rc 0 and turns (i) red with exactly that file; deleting only the touch-time link in the §5 stub keeps
`touch-time-split-record.md` reached through the relinked 2318, and the guard stays green. (i) pins
membership, not which edge provides it.

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
  `git show 39cc365a:<entry path>`; `above` / `below` read in family section order. The three
  existing heads say instead that any other unqualified `§N` "refers to the **umbrella**", except two
  named carve-outs — false once §4/§5/§8/§9 leave the entry — so commit (2) rewrites that sentence in
  each to this rule (one reading rule for the family; §3, existing heads row);
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
  with no subject. (c) *A subdirectory of `.claude/tools/`*: the control's reach is the checker's
  directory, non-recursive (`:487`). A subdirectory is outside that reach by glob depth only, so the
  guard would be outside the control for a reason unrelated to what it is: it would still sit in the
  checker's tree, importing the population, and the natural tightening of a membership control — a
  recursive glob — would turn it red for R2's reason. `docs/plans/` is outside the control's subject
  (checker code), not merely outside its glob, so no tightening of the control reaches it.
- **Precedents, and what is new.** For the import form: `.claude/tools/_webref/test_*.py` put a
  parent directory on `sys.path` (`sys.path.insert(0, str(Path(__file__).resolve().parents[1]))`) — but
  they are tests inside their tool's own package, in shape closer to (c) than to this placement. For
  a tracked non-Markdown file beside a plan family:
  `docs/plans/2026-09-citation-hygiene-k2-fgit-hermetic-cr2-plan-finalhead-check.sh` — but it is a
  one-off evidence script that no CI job runs. Every current `tools` set member sits in the tree of
  the code it tests (`.claude/tools/`, `.claude/skills/elidex-plan-review/`, `scripts/`), so **as a CI
  suite member under `docs/`, and as one whose subject is documents rather than code beside it, the
  guard is the repository's first.** The split PR's description says so.
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
  `vm-p4-artifacts/split-sim/r3/test_vm_p4_family_layout.py`, which commit (2) adds as is):
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
    ["2026-07-vm-p4-slot-ledger.md"]}`, **each list compared sorted** — the observed lists come in BFS
    order, which a reordering of first links would move, so an unsorted compare of a list of length ≥ 2
    would go red on a prose edit. One list of length one per schema pins both "exactly one table" and
    "in which memo". Row counts are **not** pinned.
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
  layout. (b) *The guarantee by layer* (§1.3): while a pin has length one, a header drifting off its
  schema makes the checker itself exit 2 and the test red through `setUp`; a pin of length ≥ 2 keeps
  the test — and the `tools` job — red on that drift through (ii), and gives up only the checker's own
  signal (a manual run can exit 0 under rule C unless a drifted row carries a kind phrase). (c) *When a
  structural change moves a schema table to another memo, adds a carve-out, or otherwise changes the
  layout, update this test in the same commit — that update is the intended change, not a
  workaround*; and when (i) is red because a member links a `.md` outside the family, the default
  remedy is to drop the link (a backticked path adds no edge), while widening the family means
  updating `FAMILY` and CLAUDE.md's `docs/plans/2026-07-vm-p4-*.md` glob in the same commit. (d) It lives beside the family
  as a consumer of the checker's library, outside the checker self-test's membership rule, and runs
  as its own suite root. Without (a) and (c) the test would read as one-home re-introduced outside
  the checker; without (b) a reader updating a pin would not see what the update costs.
- **Measured, in production form.** A `git clone --local` of `vm-p4-plan-doc` at `08a2a68c`
  (`vm-p4-artifacts/split-sim/r2-repo`); commit (1) as a real commit (`r3/sim3.py <repo> 1`), commit
  (2) as a real commit on top (`r3/apply_c2.py <repo> <this plan> r3/test_vm_p4_family_layout.py`: §3's
  edit spec and the guard at its path). Hashes in `r3/r4-commits.txt`.
  1. checker `--self-test` at `<c2>` (`python3 .claude/tools/plan-memo-umbrella-check.py --self-test`):
     rc 0, `954 control(s)`, `all controls behaved as declared.` (`r3/r4-c2-selftest.txt`).
  2. `bash scripts/python-suites.sh` at `<c2>`: rc 0, `GREEN -- 10 member(s), 120 test(s) passed,
     offline`; `root docs/plans (1 member(s))`, `docs/plans/test_vm_p4_family_layout.py: 2 test(s)
     passed` (`r3/r4-c2-python-suites.txt`).
  3. The guard (`python3 -m unittest docs/plans/test_vm_p4_family_layout.py`) under one-edit mutations
     of the `<c2>` tree (`r3/guard-controls.sh`, output `r3/r4-guard-controls.txt`, equal to R3's
     `r3/guard-controls.txt` but for timings): green as built; the §4
     stub's link deleted → checker rc 0, (i) red with exactly `design-invariants.md`; the §5 stub's
     touch-time link deleted → green (2318 still reaches it); one link from `open-decisions.md` to the
     checker plan → checker rc 0, (i) red with `2026-08-plan-memo-umbrella-checker.md` and `…-rounds.md`;
     a stub-table row given a seventh cell → checker rc 2, both tests red through `setUp`; the
     entry's `citation` header drifted (pin length 1) → checker rc 2, both red through `setUp`; the
     `citation` pin lengthened to the entry plus `slice-0a…`, which gets a header-only `citation`
     table, then the entry's header drifted → **checker rc 0, (ii) red** (the CI layer of §1.3); the
     `slot` pin lengthened to `[slot-ledger, slice-0a…]` against BFS order `[slice-0a…, slot-ledger]`
     → green with the sorted compare, (ii) red with an unsorted one. Red on the pre-split tree, both
     tests (population 4 memos; `slice` and `slot` at the entry) — which is why it lands in commit
     (2), after the move.
  4. Negative control for the placement: the same file at `.claude/tools/test_vm_p4_family_layout.py`
     → `--self-test` rc 1, `FAIL [CONTROL] PROPERTY: no module can leave a run unnoticed … 1
     violation(s); test_vm_p4_family_layout.py imports the population but is not in it` — R2's CRIT,
     reproduced.
  Re-run: `vm-p4-artifacts/split-sim/r3/measure.sh <clone at 08a2a68c> pre|crit` for the pre-split
  red and 4 (output `r3/measure-run.txt`), and §3's commands for the `<c1>`/`<c2>` heads.
- **Scope note for the split PR**: this puts tools code (a Python test) into #506, a plan PR. The
  split PR's description says so, and the `/pre-push` run for the split covers it: `mise run
  tools-test` / `mise run ci` run `scripts/python-suites.sh` once, under the first `python3` on `PATH`
  (its header, step 1). The `"3.9"` / `"3.x"` matrix exists only in GitHub's `tools` job, so the
  3.9 floor is measured by CI, not by `/pre-push`; the guard uses no syntax newer than 3.9. The guard
  adds suite cost, so the split PR also applies the `tools` job's timeout rule (§3 step 2, last check).

### §1.6 Rejected (record only): all four tables kept in the entry, no S0

It reversed the 2291 verdict (table separated from the specification its cells defer to) and split
§8 from its own rules, to avoid a tools change; moot since S0 landed.

## §2. Coupled invariants, and the measured checker diff

### §2.0 The invariants and their intersections

The split satisfies ten invariants at once:

- **C1 byte-exact move** — each child's body = its range; entry = kept lines + exactly the four
  six-line stubs of §1.2, in section order (§2.1).
- **C2 entry relink** — 58 bare `§4/§5/§8/§9` in the entry become links (§3; one named exclusion,
  7's `§4`); 2318's `§5` → `touch-time-split-record.md`.
- **C3 no new naming site** — stub and head text name no declared id (§1.2, §1.4).
- **C4 population membership** — every child is a population member, and commit (1) gives each new
  child an edge that does not depend on incidental prose (its stub link). The guard keeps the first
  half afterwards (§1.5 (i)); the second half is a commit-time property of the stub form (§1.2,
  proved by §2.1's byte compare of the stubs), which the guard does not keep — it pins membership, not
  which edge provides it.
- **C5 schema home** — each of the four schemas has one table in one memo (§1.3).
- **C6 predicted checker diff** — §2.2's classes and nothing else, at both commit heads.
- **C7 family order** — `above`/`below` and section order read across files as before.
- **C8 pointer commit (2)** — three existing stub links, six kept entry statements the split
  falsifies, the three existing heads (re-point and reading rule), the checker-plan anchor,
  CLAUDE.md:54 and :55, the guard (§3) — each line edit byte-fixed by §3's edit spec.
- **C9 family guard** — the `setUp` precondition, (i) and (ii) of §1.5.
- **C10 base revision** — the umbrella in commit (1)'s parent is the `39cc365a` blob
  (`git rev-parse <c1>^:<entry>` = `git rev-parse 39cc365a:<entry>`). Every line number, range and
  count in this plan, and the heads' "moved verbatim at `39cc365a`", are statements about that blob;
  if #506 edits the umbrella before the split, the plan's ranges are re-derived first, not applied.

| Pair | Intersection |
|---|---|
| C1 × C2 | the relink edits kept lines; §2.1 undoes exactly the relink form (`[§N](<new file>)`) before diffing, so a relink is the one edit to a kept line commit (1) allows |
| C1 × C3 | heads are outside the byte proof (head = text before the range), so C3 for heads is checked by the checker output; the stubs are inside it — §2.1 compares them byte-exact with the §1.2 form, which names no id |
| C1 × C4 | the stub links are part of the stub text §2.1 compares byte-exact, so §2.1 proves each new child's stub edge (five links, section order) |
| C1 × C5 | the four schema tables are in moved or kept ranges, so §2.1 proves each moved byte-for-byte; header drift by the move is impossible |
| C1 × C6 | class 2 of §2.2 exists because a child's line numbers are its range shifted by the head length; §2.2 maps them back (`a + N − h − 1`) |
| C1 × C7 | the bodies are verbatim, so deixes keep their text; order is restored by stub order, not by editing them |
| C1 × C8 | commit (2) edits nine **kept** entry lines (the stubs 2317, 3079, 3089 and the statements 20, 2325, 2334, 2339, 2340, 2362), so §2.1 is run with `<c1>` only (given `<c2>`, it stops at the C10 assertion, since `<c2>^` is `<c1>` — measured), and commit (2) has its own kept-line check (§3 step 2) |
| C1 × C9 | none: the guard reads structure, not bytes |
| C1 × C10 | §2.1 reads `old` from `<c1>^` and asserts first that it is the `39cc365a` blob; C1 is defined against that blob |
| C2 × C3 | a relink rewrites existing text into a link whose label is `§N`, never an id; class 1 of §2.2 is excerpt text only |
| C2 × C4 | each relink is an edge; it is not relied on for C4's second half because an incidental reference can be edited away — but it does count for membership, which is why (i) stays green when only the §5 stub's touch-time link goes (2318 still reaches it) and goes red when the §4 stub's link goes (no relink reaches `design-invariants.md`) (§1.3) |
| C2 × C5 | relinks edit cells of the entry's `stub` table (e.g. 439-475), never a header or delimiter row, so the table still matches its schema (measured: every count identical) |
| C2 × C6 | class 1 of §2.2: 11 entry lines whose excerpt now shows a link |
| C2 × C7 | none: deixes are untouched, links do not reorder text |
| C2 × C8 | commit (2) adds no relink and no bare `§N` to the entry: its entry edits are the three stub links and six kept-text replacements whose new text holds no `§`, so C2's entry-links rule holds at `<c2>` as at `<c1>` |
| C2 × C9 | relinks add edges among members only, so (i) is unchanged |
| C2 × C10 | the relink census (§3 step 1) compares `<c1>` with `<c1>^`: 58 links on 51 kept lines, per target as in §3 |
| C3 × C4 | the stub link's label is a section label, so the edge it adds names nothing (zero inserted lines in the output) |
| C3 × C5 | heads and stubs contain no table, so no schema table is added |
| C3 × C6 | zero head/stub lines in the output (measured: no checker line maps to an inserted line) |
| C3 × C7 | the heads state the family-order reading rule; the rule is text, not an id |
| C3 × C8 | commit (2)'s head edits, kept-text replacements and stub links name only file paths, section labels, counts and a revision — no declared id (2362's replacement leaves the row id the line already spells and adds none); measured: output byte-identical (§2.2) |
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
| C5 × C9 | (ii) pins the one-home layout; at pin length one that layout is what makes drift a checker rc 2, and at any pin length (ii) keeps the `tools` job red on drift — for `citation` (no kind-phrase row) and, one row-edit away, `stub`, nothing else does (§1.3) |
| C5 × C10 | the row counts (29 / 55 / 64 / 52) and the tables' ranges are those of the `39cc365a` blob |
| C6 × C7 | none: the checker reads no deixis |
| C6 × C8 | commit (2) must not move any output: measured, the commit-(2) output is **byte-identical** to the commit-(1) output |
| C6 × C9 | the guard is a separate test outside the checker's directory; it adds nothing to the checker output and nothing to its self-test (§1.5) |
| C6 × C10 | the baseline is the output on the `39cc365a` blob; if a checker file changed between `e44ffce6` and `<c1>^`, the baseline is re-taken at `<c1>^` before comparing (§2.2) |
| C7 × C8 | the reading-rule rewrite gives the three existing heads the new heads' rule (section numbers read through the entry's stubs), so C7 has one statement across the family; every head edit is a same-line replacement, so no text moves; and 2334's "in the order they are listed above" — out of step with its own counts already at `39cc365a` (umbrella 29, detail file 21, rounds record 2, `slice-0a…` 0, against a list on 2325-2326 that names the rounds record first and omits `slice-0a…`) — gives way to an explicit file order, so commit (2)'s text adds no cross-file deixis |
| C7 × C9 | none |
| C7 × C10 | none |
| C8 × C9 | the guard lands in commit (2), after the move, because at commit (1)'s parent it is red by design |
| C8 × C10 | commit (2)'s checks compare `<c2>` with `<c1>`, not with `<c1>^`; the checker-plan anchor it writes ("line 2716 at `39cc365a`") is true only under C10, as is 2325's "the five files carved from it at `39cc365a`"; the counts it writes at 2334 and 2339-2340 are measured at `<c2>` (§3), not at `39cc365a` |
| C9 × C10 | none: the guard reads the head |

### §2.1 Content-free proof (commit (1))

Byte-exact, with **no free insertion**: each child's body is its range; the entry, with the relinks
undone, is the kept lines with exactly the four six-line stubs of §1.2 inserted where the moved
sections stood (§4 and §5 after 697, §8 and §9 after 2397); and those stub lines, links kept, equal the
§1.2 text byte for byte — which fixes the chunk count, each chunk's length and text, the five stub
links and the section order in one comparison (C1, and C4's second half). Everything is read from
revisions, not from the working tree: `old` from `<c1>^`, after asserting C10; the bodies and the entry
from `<c1>`. Run from the repository root as `python3 proof.py <c1>`
(`vm-p4-artifacts/split-sim/r3/proof.py`, the text below).

```python
import re, subprocess, sys
C1 = sys.argv[1]
D, X = 'docs/plans', '2026-07-vm-p4-'; U = X + 'es-language-completeness.md'
M = [('design-invariants', 698, 1006), ('slice-plan', 1007, 2123), ('touch-time-split-record', 2124, 2314),
     ('slot-ledger', 2398, 2786), ('open-decisions', 2787, 3076)]
git = lambda *a: subprocess.run(['git', *a], capture_output=True, text=True, check=True).stdout
assert git('rev-parse', f'{C1}^:{D}/{U}') == git('rev-parse', f'39cc365a:{D}/{U}'), 'C10: <c1>^ umbrella != 39cc365a'
show = lambda r, f: git('show', f'{r}:{D}/{f}').split('\n')[:-1]
old, new = show(C1 + '^', U), show(C1, U)
for name, a, b in M:  # C1: each child's body is its range
    assert show(C1, f'{X}{name}.md')[-(b - a + 1):] == old[a - 1:b], name
ref = lambda f, lab: f'`{D}/{X}{f}.md` ([{lab}]({X}{f}.md))'
def stub(a, *files):  # §1.2 stub form: original heading + marker, one body line, separator
    body = 'Moved **verbatim** to ' + ', and its 1000-line touch-time check to '.join(ref(f, lab) for f, lab in files)
    return [old[a - 1] + ' → **carved out**', '', body + '; sub-headings keep their numbers there.', '', '---', '']
S45 = stub(698, ('design-invariants', '§4')) + stub(1007, ('slice-plan', '§5'), ('touch-time-split-record', '§5'))
S89 = stub(2398, ('slot-ledger', '§8')) + stub(2787, ('open-decisions', '§9'))
P1, P2, P3 = old[:697], old[2314:2397], old[3076:]  # kept ranges 1-697, 2315-2397, 3077-3093
link = re.compile(r'\[(§[4589])\]\(' + X + '(?:' + '|'.join(n for n, _, _ in M) + r')\.md\)')
undo = lambda ls: [link.sub(r'\1', l) for l in ls]
want = P1 + undo(S45) + P2 + undo(S89) + P3            # the entry, links undone: kept + exactly the 4 stubs
assert len(new) == len(want), ('entry length', len(new), len(want))
bad = [i + 1 for i, (g, w) in enumerate(zip(undo(new), want)) if g != w]
assert not bad, ('entry lines off kept+stubs', bad[:5], len(bad))
i45, i89 = len(P1), len(P1) + len(S45) + len(P2)       # the stubs, links kept: byte-exact, in section order
assert new[i45:i45 + len(S45)] == S45 and new[i89:i89 + len(S89)] == S89, 'stub text/links/order'
nlinks = sum(len(link.findall(l)) for l in S45 + S89)
print('base 39cc365a ok; moved', sum(b - a + 1 for _, a, b in M), 'kept', len(P1 + P2 + P3),
      'stubs 4 links', nlinks, 'entry = kept + stubs')
```

Expected at commit (1): `base 39cc365a ok; moved 2296 kept 797 stubs 4 links 5 entry = kept + stubs`.
**Measured** at the production-form `<c1>` (`r3-commits.txt`): that line, rc 0; also at R2's `<c1>`
(`17bbcc13`, `r2-commits.txt`), whose stubs are the same text. Negative controls
(`vm-p4-artifacts/split-sim/r3/proof-controls.sh`; each a commit whose parent is `<c1>^` and whose tree
is `<c1>` plus one edit; output `r3/proof-controls.txt`), with R2's proof (`r2-proof-c1.py`, which
allowed any inserted chunk containing a `→ **carved out**` line) for contrast:

| Edit to `<c1>` | R2 proof | this proof |
|---|---|---|
| the I-1 paragraph (39cc365a:700-704) left behind inside the §4/§5 stub chunk | rc 0 — **passes** | rc 1, `entry length 827, 821` |
| the §4 stub's link deleted | rc 0 — **passes** | rc 1, entry line 700 |
| the §4/§5 stubs swapped | rc 0 — **passes** | rc 1, entry lines 698, 700, 704, 706 |
| one character appended to entry line 100 | rc 1 | rc 1, entry line 100 |
| one character appended to `slot-ledger.md` line 200 | rc 1 | rc 1, `slot-ledger` |
| the script given `<c2>` (C10) | — | rc 1, `C10: <c1>^ umbrella != 39cc365a` |

### §2.2 Checker output: baseline, commit (1), commit (2)

**Baseline** (`39cc365a`, saved: `vm-p4-artifacts/p506_check_pre.txt`, 868 lines, checker at
`e44ffce6`): rc **0**; `[UMBRELLA-MARK] 48` (`slice=33, slot=15`); `[CENSUS] 53`;
`[KIND-UNDETERMINED] 5`; `[UMBRELLA-CELL] 37`; footer `51 seed(s) and 715 reported naming site(s)`;
population = the three existing carve-outs.

**If the checker moved before the split.** The saved baseline is valid only for the checker at
`e44ffce6`. Before comparing, check `git diff --quiet e44ffce6 <c1>^ -- .claude/tools/`; if it reports a difference, re-take the baseline
on `<c1>^` with the checker at `<c1>^`
(`git checkout <c1>^ && python3 .claude/tools/plan-memo-umbrella-check.py $M > base.txt`), re-run the
§2.2 prediction against `base.txt`, and report any class the new checker adds; do not compare against
`p506_check_pre.txt`.

**Prediction, measured at the production-form heads** (§1.5; `<c1>` built by
`vm-p4-artifacts/split-sim/r3/sim3.py <repo> 1`, `<c2>` by `r3/apply_c2.py` from §3's edit spec;
outputs `r3/r3-after1.txt` / `r4-after2.txt` — `r4-after1.txt`, the `<c1>` output re-run beside it, is byte-identical to `r3-after1.txt`; `norm2.py r3/origin-c1.json` maps each `file:N` back to
its `39cc365a` line). The layout is this plan's: §1.1 ranges, §1.4-shaped heads (12 lines, 14 for the
touch-time file), the §1.2 stubs, the 58 relinks (2318's `§5` → `touch-time-split-record.md`, 7's
`§4` excluded), and in `<c2>` every edit of §3's edit spec. R2's `<c1>` (`17bbcc13`, which also
linked 7's `§4`) gave the same output up to BFS order: sorted, the two differ only in the population
header line, where `design-invariants.md` is now scheduled after `…-rounds.md`.

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
- **commit (2) head vs commit (1) head**: **byte-identical** output (measured). Every commit-(2)
  edit is a same-line substitution (§3's edit spec), so no file's line count changes (no locator
  moves), none of the edited lines is a reported naming site (2362 spells a row the census does not
  seed; its replacement keeps that spelling and adds no id), and the edges the three stub links add point at
  memos BFS has already scheduled (population order unchanged).

Anything else — a count, an `[UMBRELLA-MARK?]` row, a seed, any difference between the two heads — is
a defect of the split. At the real heads:

```sh
M=docs/plans/2026-07-vm-p4-es-language-completeness.md
git diff --quiet e44ffce6 <c1>^ -- .claude/tools/ || echo 'checker moved: re-take the baseline at <c1>^'
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
| bare `§4/§5/§8/§9` in the **entry** (prose + table cells) | 59 raw; **58 relinked** (§4 0, §5 26, §8 18, §9 14) | commit (1): → `[§N](<file>)`; §5 refs whose referent is the touch-time passage (one: 2318, "§5's *And the verdict…*") link `touch-time-split-record.md`, the rest `slice-plan.md`. **Exclusion list, fixed: one entry** — 7's `§4` in "`project_vm-p4-es-language-gaps.md` §2/§4" is that memory file's §4, not the umbrella's, and stays bare. Spec-clause collisions: **none** (`python3 vm-p4-artifacts/split-sim/r3/collisions.py <git show 39cc365a:<entry>>` → `bare tokens 59 spec-preceded 0`; each of the 59 read by hand, which found the line-7 foreign-document reference the spec heuristic cannot see). |
| same, in **moved** text | INV 33, PLAN 267, TT 11, LED 163, DEC 26 | verbatim; `§N` names the family section, resolved through the entry's stubs (numbers are stable) |
| same, in the 3 **existing** carve-outs | 76 raw (10 / 31 / 35) | verbatim — resolved through the entry's stubs under the reading rule commit (2) gives their heads (row below) |
| `§N` inside code spans | 4 (PLAN) | verbatim (commands/quotations) |
| links to existing carve-outs `](2026-07-vm-p4-….md)` | 84 (P 11, moved 73) | unchanged; same directory, so they resolve from the new files |
| existing stubs (2317, 3079, 3089) | 3 | commit (2): the parenthesised link after the backticked path, same line, no reflow — the one stub form (§1.2); exact text in the edit spec below |
| intra-file anchors `](#…)` | 0 | — |
| self line cites (`line N`, `L N`, `<this file>:N` forms) | 0 | revision-anchored ranges (`N-M at <rev>`) stay verbatim as statements about that revision |
| `<this file>` in commands | 31 (P 1 = 26, citation table, stays valid; moved 30) | verbatim; each carve head says `<this file>` there means `git show 39cc365a:<entry path>` (§1.4), as the precedent heads do for figures |
| `I-n` names | 168 | verbatim; they stay in `design-invariants.md`, and no stub or head spells one (C3) |
| `above` / `below` | 416 (PLAN 204, LED 140, P 41) | verbatim; the carve heads state they read in family order, which the stubs preserve (known crossing: 1988 "see the 1000-line check below" → TT) |
| heads of the 3 existing carve-outs | 13 lines (5 / 5 / 4 at `<c1>`; head = lines 1-18 of each) | commit (2), same-line replacements, no line added or removed (a moved line would move every naming site below it — 95 of `<c1>`'s output lines are in these files). What becomes false once §4/§5/§8/§9 leave the entry, read from the three heads: (a) line 5 of each, "the seam derivation and its measurements are in the umbrella's §5" — that passage is now in `touch-time-split-record.md`, and "pre-split" (line 7) becomes ambiguous between two splits: an insertion names the second split and its revision; (b) the reading rule — "Any other unqualified `§N` refers to the **umbrella**, except" two named carve-outs, "the umbrella's two other carve-outs" (`slice-0a…` 14-17, `slice-1a-1b…` 14-16, `…-rounds` 16-18) — now names a set that misses five files: rewritten to the new heads' rule (§1.4: through the entry's stubs); (c) `slice-1a-1b…` 9, "**Scheduling stays in the umbrella.**" — §5/§8/§9 left the entry: "…in the umbrella family." Nothing else in the heads is false: `slice-0a…` 9-10 and the rest of `slice-1a-1b…` 9-10 name §5/§8/§9, which the new rule resolves; the archive and record descriptions are about the files themselves. **Backticked**, not linked — membership does not need it |
| statements in the entry's **kept** text (and the existing carve-outs' bodies) whose truth the split changes | 6 false after the split, all in kept text (20, 2325, 2334, 2339, 2340, 2362); carve-out bodies 0 | commit (2), same-line replacements (a moved line would move the locators below it), exact text in the edit spec below; the census, the criterion and the statements found true are listed after this table |
| other docs: line cites into the umbrella | 3, all in `docs/plans/2026-08-plan-memo-umbrella-checker.md`: `:663` "main memo line 2716"; `:1098` `` `190d2adb:…:1218` `` / `` `:2711` ``; `:1103` `` `190d2adb:…:2710` `` | commit (2): anchor `:663` as "line 2716 at `39cc365a`"; `:1098` / `:1103` are already anchored to `190d2adb` and stay. Complement measured (below). |
| CLAUDE.md:54 and :55 (on main `e44ffce6` line 55 is the **Git** rule; the checker-run rule is this branch's) | 2 | commit (2): :54 states the one exception to "memo 実行は手動 … CI job に入れない" — this family's guard runs in the `tools` job, and through its `setUp` precondition any rc-2 defect of a family memo makes that job red; :55 extends the trigger to every family memo, replaces its rc-2 clause ("2 は memo の table header が checker の SCHEMAS とずれた" contradicts :54's "header drift に限らず"), and adds the guard-update rule with the remedy for a red (i) and the pin-length layers (§1.3). Exact text in the edit spec below |

**Kept-text census** (the row above; lines at `39cc365a`, `<c1>` = line − 1605 for 2315-2397).
Enumerated by concept over the kept ranges and the three carve-out bodies: what "this document" /
"this file" / "here" / "above" / "below" points at, where a "single home" lies, and any figure fixed
by the family's file set. **Criterion: a statement is in scope when it is unqualified present tense
and its referent is fixed by the current file layout; one bound to a revision, a round or a past
event (past tense, `at <rev>`, a round or landing record, a carve-out body its head binds to the
pre-split umbrella) is not, since the split does not change what was true then.**

- **False after the split — replaced in commit (2).** 20: "the document also cites … **WebIDL** and
  **WHATWG HTML**" — once §4/§5/§8 leave, the entry's only WebIDL / WHATWG HTML mentions are this
  paragraph's own (20-22); → "the document and its carve-outs also cite …". 2325: the `edge NN`
  namespace covers "this document", the rounds record and the detail file — 27 of the entry's 29
  references move (`slice-plan.md` 16, `open-decisions.md` 11), outside that list; → adds "the five
  files carved from it at `39cc365a`". 2334: "the four files of this program … **29 / 21 / 2 / 0**";
  2339-2340: the line counts "15 / 17 / 2 / 0", disagreeing on "two of the four files" — measured at
  `<c2>` over the nine memos (`grep -o … | wc -l` / `grep -c`): this document 2 / 2,
  `slice-plan.md` 16 / 5, `open-decisions.md` 11 / 8, the detail file 21 / 17, the rounds record
  2 / 2, the other four 0 / 0; → the nine files in that explicit order, "2 / 5 / 8 / 17 / 2 / 0",
  "three of the nine files". 2362: §5's owner row "is this document's single home for that
  assignment" — the row is in `slice-plan.md`; → "the single home, in this document and its
  carve-outs, for that assignment".
- **True after the split — no edit.** 18 ("not the document's citation chokepoint": true of the entry
  and of the family); 26-27 (`<this file>` command → `29 0 0 0 0 0 1`, measured at `<c1>`: the table
  stays in the entry); 105, 390, 439 ("below" into §5 rows and a §8 slot: the stubs stand where §5 and
  §8 stood, C1/C7); 186 ("match this plan file itself": 185-186 are kept and still match); 588, 2330,
  2368, 2371 (homes named by section — §5's rows, §5's one structured home, §9 dec. 16 — resolved
  through the stubs); 2324 ("this sentence is its single home": it stays); 2327 ("nothing else in
  this program numbers edges"); 2329 ("a section number this document no longer contains"); 3080-3083
  and 3090-3093 ("every reference to them in this document links to that file": **false at
  `39cc365a`**, through the bare `§§16-18`, `§§10-15` and `§§10-18` at 2246, 2250 and 2305, all in the
  touch-time passage, and **true after**, since those lines leave the entry and the kept text has none
  bare). The carve-out bodies: the rounds record and the landing record are records (criterion); the
  detail file's present-tense references are an unqualified `§N`, resolved by the reading rule commit
  (2) gives its head, or "this file", which is still that file.
- **Out of scope by the criterion.** 23-24 ("nothing in this document owned", past tense); 2317,
  3079, 3089 ("by this document's touch-time prereq split", the first split's past event — 2318 names
  its passage); 475 ("at this document's HEAD") and 505 ("this document's own baseline"), both
  revision-bound.
- **Outside this class (pre-existing, not edited).** 2320-2321, "every `§6…` reference in this
  document links to that file", is false at `39cc365a` and after (bare `§6.4` at 2324, 2327, 2328,
  2340, all kept); the split does not change it. Its consequence, "scheduling from §5, §8 and §9
  reads exactly as it did", stays true: every umbrella-`§6` reference in those sections is a link
  to the detail file (19 / 3 / 13 outside code spans; the bare `§6…` there are URL and IndexedDB
  sections), moved verbatim.

Complement for the "other docs" row — every tracked file outside the family and this plan that
mentions the umbrella, then every line-cite form in those files (on this branch; `a45a42d1`):

```sh
for f in $(git grep -lE 'es-language-completeness|#506.s memo|main memo' a45a42d1 -- . \
    ':!docs/plans/2026-07-vm-p4-*' ':!docs/plans/2026-10-vm-p4-memo-seam-split.md' | sed 's/^a45a42d1://'); do
  git show a45a42d1:$f | /usr/bin/grep -nE 'es-language-completeness\.md`?:[0-9]|main memo line|…:[0-9]|memo.{0,12}line [0-9]{3,4}' | sed "s|^|$f:|"
done
```

→ the three lines above, nothing else (the selector matches 9 files: six under `.claude/tools/`,
CLAUDE.md, the checker plan and its rounds record).

**Commit (2)'s edit spec** — the single home of every line commit (2) edits; §3 step 2's check reads
this block and compares byte for byte, so there is no free text and no review-by-eye step. Each hunk is
`@ <path>:<line at <c1>>`, then `-` / `+` pairs: the `-` text occurs exactly once in that `<c1>` line
and is replaced by the `+` text, pairs applied in order. No line is added or removed. Wording changes
after this plan is approved are made here, and the check follows.

```c2-edits
@ docs/plans/2026-07-vm-p4-es-language-completeness.md:20
- the document also cites ECMA-262 sections inline without IDs, and cites
+ the document and its carve-outs also cite ECMA-262 sections inline without IDs, and cite

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:712
- `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md`
+ `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` ([§6](2026-07-vm-p4-slice-1a-1b-call-spread-detail.md))

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:720
- in this document, in
+ in this document, in the five files carved from it at `39cc365a`, in

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:729
- the four files of this program in the order they are listed above — **29 / 21 / 2 / 0**:
+ the nine memo files of this program — this document, `2026-07-vm-p4-slice-plan.md`, `2026-07-vm-p4-open-decisions.md`, the detail file, the review-rounds record, then each of the other four — **2 / 16 / 11 / 21 / 2 / 0**:

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:734
- 15 / 17 / 2 / 0
+ 2 / 5 / 8 / 17 / 2 / 0

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:735
- two of the four files.
+ three of the nine files.

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:757
- which is this document's single home for that assignment
+ which is the single home, in this document and its carve-outs, for that assignment

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:807
- `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md`
+ `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md` ([§§10-15](2026-07-vm-p4-umbrella-review-rounds.md))

@ docs/plans/2026-07-vm-p4-es-language-completeness.md:817
- `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`
+ `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md` ([§§16-18](2026-07-vm-p4-slice-0a-landing-record.md))

@ docs/plans/2026-07-vm-p4-slice-0a-landing-record.md:5
- in the umbrella's §5
+ in the umbrella's §5; the umbrella's second split (at `39cc365a`) moved that passage to `docs/plans/2026-07-vm-p4-touch-time-split-record.md`, and "pre-split" below still means before the first split

@ docs/plans/2026-07-vm-p4-slice-0a-landing-record.md:14
- Any other unqualified `§N` refers to the
+ Any other unqualified `§N` names that section

@ docs/plans/2026-07-vm-p4-slice-0a-landing-record.md:15
- **umbrella**, except: `§6` is in `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` and
+ of the umbrella family, located through the carved-out stubs of the entry

@ docs/plans/2026-07-vm-p4-slice-0a-landing-record.md:16
- `§§10-15` are in `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md`, the
+ (`docs/plans/2026-07-vm-p4-es-language-completeness.md`), which name the file each carved section is

@ docs/plans/2026-07-vm-p4-slice-0a-landing-record.md:17
- umbrella's two other carve-outs.
+ in; the rest of the umbrella's sections are in the entry.

@ docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:5
- in the umbrella's §5
+ in the umbrella's §5; the umbrella's second split (at `39cc365a`) moved that passage to `docs/plans/2026-07-vm-p4-touch-time-split-record.md`, and "pre-split" below still means before the first split

@ docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:9
- **Scheduling stays in the umbrella.**
+ **Scheduling stays in the umbrella family.**

@ docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:14
- in this file refers to the **umbrella**, except: `§§10-15`
+ in this file names that section of the umbrella family,

@ docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:15
- are in `docs/plans/2026-07-vm-p4-umbrella-review-rounds.md` and `§§16-18` are in
+ located through the carved-out stubs of the entry (`docs/plans/2026-07-vm-p4-es-language-completeness.md`),

@ docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md:16
- `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`, the umbrella's two other carve-outs.
+ which name the file each carved section is in; the rest of the umbrella's sections are in the entry.

@ docs/plans/2026-07-vm-p4-umbrella-review-rounds.md:5
- in the umbrella's §5
+ in the umbrella's §5; the umbrella's second split (at `39cc365a`) moved that passage to `docs/plans/2026-07-vm-p4-touch-time-split-record.md`, and "pre-split" below still means before the first split

@ docs/plans/2026-07-vm-p4-umbrella-review-rounds.md:16
- refers to the **umbrella**, except: `§6` is in
+ names that section of the umbrella family, located through the

@ docs/plans/2026-07-vm-p4-umbrella-review-rounds.md:17
- `docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` and `§§16-18` are in
+ carved-out stubs of the entry (`docs/plans/2026-07-vm-p4-es-language-completeness.md`), which name

@ docs/plans/2026-07-vm-p4-umbrella-review-rounds.md:18
- `docs/plans/2026-07-vm-p4-slice-0a-landing-record.md`, the umbrella's two other carve-outs.
+ the file each carved section is in; the rest of the umbrella's sections are in the entry.

@ docs/plans/2026-08-plan-memo-umbrella-checker.md:663
- main memo line 2716
+ main memo line 2716 at `39cc365a`

@ CLAUDE.md:54
- (`scripts/trip-wires.sh` の `REQUIRED_WIRES` には登録しない)。
+ (`scripts/trip-wires.sh` の `REQUIRED_WIRES` には登録しない)。例外は VM-P4 family (`docs/plans/2026-07-vm-p4-*.md`) で、その layout guard `docs/plans/test_vm_p4_family_layout.py` が checker を library として import し、population と schema table の置き場所を pin する test として同じ `tools` job で走る (memo 実行そのものではなく、checker の presence rule を締めるものでもない)。guard の前提 `misses == []` により、family の memo に checker が rc 2 を出す欠陥があれば (header drift に限らず、行幅・未解決 reference 等の内容欠陥も) `tools` job が赤になる。memo 編集で `tools` job が赤になる経路は、上記 `symbol_attribution_control` (全 `docs/plans/*.md`) とこの guard (VM-P4 family のみ) の 2 つ。

@ CLAUDE.md:55
- を編集したら
+ を編集したら (carve-out された family memo `docs/plans/2026-07-vm-p4-*.md` のどれを編集した場合も、entry であるこの memo に対して)
- 2 は memo の table header が checker の SCHEMAS とずれた = 未走査 (clean ではない)。
+ 2 は未走査 (clean ではない。原因は table header と checker の SCHEMAS のずれに限らず、checker が miss として報告する欠陥すべて)。family の構造を変える編集 (memo の追加・削除、schema table の別 memo への移動・追加) は `docs/plans/test_vm_p4_family_layout.py` の pin を同 commit で更新する。family 外の `.md` への相対リンクで guard が赤になったら、原則はそのリンクを外す (path を backtick で書けば辺にならない)。family を広げるなら guard の `FAMILY` と、この行と上の行の glob `docs/plans/2026-07-vm-p4-*.md` を同 commit で更新する。schema の pin が長さ 1 の間は、その schema の header drift で checker 自身が rc 2 を出す。pin を長さ 2 以上にしても drift は guard (ii) が `tools` job で赤にするが、checker の手動実行は rc 0 を返し得る (drift した table の行に kind phrase が無い場合) — 長さ 2 以上で手放すのは checker 単体での検出であって、`tools` job の赤ではない。
```

"The 2 routes" on :54 is measured, not asserted: the tracked `.py` files that read `docs/plans` at run
time are found by `git grep -nE "docs/plans|['\"]plans['\"]" -- '*.py'` at `08a2a68c` — every hit is a
docstring or comment except `plan_memo_selftest_records.py:304` (`_attribution_corpus`, which
`symbol_attribution_control` reads; it globs `*.md`, so the guard's `.py` beside the memos does not
enter it); the guard adds the second. The other set members that touch a `docs` path
(`.claude/tools/_webref/test_*.py`) build it under a temporary root.

**Commit order in the split PR.**

1. **Content-free commit**: the five children (heads per §1.4), the entry's 58 relinks, the four new
   stubs. Checks, all from the repository root:
   - C10, C1 and C4's second half: §2.1's script with `<c1>` →
     `base 39cc365a ok; moved 2296 kept 797 stubs 4 links 5 entry = kept + stubs` (stub text, the five
     links and stub order = original section order are part of its byte compare);
   - C2, the relink census at the real head — every `[§N](<child>.md)` on a **kept** line, as
     (`<c1>^` line, label, target), counted against §3's row:

     ```sh
     U=docs/plans/2026-07-vm-p4-es-language-completeness.md
     git show <c1>^:$U > old.md && git show <c1>:$U > new.md
     python3 relink_census.py old.md new.md
     ```

     where `relink_census.py` is `vm-p4-artifacts/split-sim/r2-relink-census.py` (§2.1's link regex
     and kept/moved partition; it maps each `equal` block of the kept-vs-undone diff back to its
     `<c1>^` line and lists the link tokens there). Expected — measured at the production-form `<c1>`:
     `relinks 58 lines 51`; `§5 -> slice-plan 25`, `§5 -> touch-time-split-record 1` (line `[2318]`),
     `§8 -> slot-ledger 18`, `§9 -> open-decisions 14`; no `§4` line (the exclusion);
   - C6: §2.2's commit-(1) prediction holds (after the baseline check of §2.2).
2. **Pointer commit**: every line edit of §3's edit spec (the three existing stub links, the entry's
   six kept-text replacements, the three existing heads, the checker-plan `:663` anchor, CLAUDE.md:54
   and :55) and the guard
   `docs/plans/test_vm_p4_family_layout.py` (§1.5). Checks:
   - every changed line is a line the edit spec names and equals its `<c1>` line with the spec's
     substitutions applied, byte for byte — CLAUDE.md included, with :54 and :55 asserted as the
     lines; no other line or file changes; line counts are unchanged; the guard is new and equals its
     production text: `python3 c2_edits.py <c1> <c2> docs/plans/2026-10-vm-p4-memo-seam-split.md
     docs/plans/test_vm_p4_family_layout.py` (`vm-p4-artifacts/split-sim/r3/c2_edits.py`; in the
     split PR the last argument is the file as committed, so the guard clause reduces to "new").
     Measured at the production-form heads, verified 2026-10-31: `c2 edits ok: 6 files, 26 lines, 27 substitutions; guard
     new, byte-equal`, and `git diff --numstat <c1> <c2>`: CLAUDE.md `2 2`, entry `9 9`,
     `slice-0a…` `5 5`, `slice-1a-1b…` `5 5`, `…-rounds` `4 4`, checker plan `1 1`, guard `92 0`.
     Negative controls (`r3/c2-controls.sh`, output `r3/r4-c2-controls.txt`): one character changed in
     the :55 replacement, one extra edited line in a carve-out head, a head edit moved to the next
     line — each rc 1;
   - the checker output at `<c2>` byte-identical to `<c1>`'s (§2.2);
   - the guard green (`python3 -m unittest docs/plans/test_vm_p4_family_layout.py`);
   - checker `--self-test`: `all controls behaved as declared.` (the guard is outside the checker's
     directory, so `registry_membership_control` stays green — §1.5);
   - `mise run tools-test` green, with `root docs/plans (1 member(s))` in its output;
   - the `tools` job timeout (`ci.yml`, the job's "Timeout" comment: a PR that adds suite cost
     re-derives `timeout-minutes` in the same PR from its own run, the slower leg bounding the job):
     the split PR reads its own run's slower leg with the comment's `gh run view` command and writes
     one line in its description — the guard's cost (measured locally: `Ran 2 tests in` 1.6-1.9 s, 1.8-2.1 s wall, at load average ≈ 60 — the split PR states its own figure), the leg's duration,
     and whether the 22-minute value keeps the comment's margin (re-derive when a run measures under
     2x). `ci.yml` changes only if that margin is lost.

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

R3 passed the gate (CRIT 0 / IMP 0); this revision applies its MINs, and a re-review covers the changed
parts only:

1. **§2.1** — the proof now admits no free insertion (entry = kept + the four §1.2 stubs, byte-exact,
   links and order included); its controls table, three of which R2's proof passed.
2. **§3** — the edit spec as the single home of commit (2)'s text, and `c2_edits.py` comparing
   against it byte for byte; the existing heads' reading-rule rewrite (the list of what becomes false,
   and the claim that nothing else in the heads does); CLAUDE.md :55's rc-2 clause replaced; the
   relink count corrected to 58 with one named exclusion (7's `§4`, a memory file's section), which
   also changes §1.2/§1.3's edge accounting for `design-invariants.md`.
3. **§1.3 / §1.5 / CLAUDE.md :55** — the drift guarantee stated by layer (checker rc 2 at pin length
   one; `tools` red at any length through (ii)); the sorted compare; the remedy for a red (i).
4. **§1.5** — the rewritten (c) rejection (from the control's reach) and the precedents paragraph
   ("first CI suite member under `docs/`"); the `tools` timeout rule as a split-PR check.
5. §7 — the container-aware definition count.
6. **§3 kept-text census** — the six kept entry statements the split falsifies, joined to commit (2)
   as same-line replacements (edit spec); the criterion separating them from revision-bound text; the
   statements found true; §2.0's C8 rows; `<c2>` re-measured (`r3/r4-commits.txt`).

## §6. Discharge, ledger, and hand-off

- **Discharge.** The split discharges the slot `#11-vm-p4-memo-section-seam-split`. It is closed when
  #506 is squash-merged with commits (1) and (2) in it; not at commit time, not at plan approval.
- **Ledger.** The slot's ledger note (`project_open-defer-slots.md:17`, "SCHEMAS が §5/§8 両 table を
  単一 file から読む") gave #510 Slice 1 as the precondition; the precondition was S0, now landed.
  The ledger belongs to the PM: the driver reports the landing (squash hash, the two commit hashes)
  and the PM records the discharge and the corrected precondition. This memo does not edit the ledger.

## §7. Spec coverage map

**No web spec is implemented.** The split moves Markdown text and changes no behaviour. It depends
on outside rules in three places. Which destination is a sibling decides the checker's population —
the WHATWG URL Standard's parser decides that a destination has no scheme, and the rest is the
checker's local policy (the checker plan's §3 row on `plan_memo_sibling.sibling_path`, `:434`, says
CommonMark and GFM say nothing about siblings on disk). Which text is a link, and which is masked as
code, is CommonMark 0.31.2; whether a schema table still is a table is GFM 0.29. URL Standard numbers
and names are webref lookups (`.claude/tools/webref heading url 4.4 --exact` → §4.4 URL parsing
`#url-parsing`; `heading url 1.3 --exact` → §1.3 Percent-encoded bytes; `dfn url 'scheme start
state'` → §4.4 `#scheme-start-state`; `dfn url percent-decode` → §1.3 `#string-percent-decode`).
Webref has no CommonMark or GFM extract: their section numbers are the ones the checker plan's §3
cites for the same rule (the provenance column of the table below); the vendored
`.claude/tools/commonmark-0.31.2-{block,inline}-examples.json` carry section **names** only, and Link
reference definitions, Code spans, Links and Autolinks all appear there under those names.

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| WHATWG URL §4.4 URL parsing | scheme start state (`#scheme-start-state`), then **local policy** | every new destination begins `2026-…`: step 1 needs an ASCII alpha, so step 2 → *no scheme state* and the destination is relative; the rest is the checker's local policy (checker plan `:434`): relative on every platform, `.md`, joined beside the memo — all new destinations are bare file names in the same directory | `plan_memo_sibling.sibling_path` stages (a), (c)-(e) → `Population.__init__` (checker plan `:434`) | ✓ one destination shape (§3) | no |
| WHATWG URL §1.3 Percent-encoded bytes | percent-decode on a string (`#string-percent-decode`) | no new destination contains `%`; decoding is the identity | `plan_memo_sibling.sibling_path` stage (b) | ✓ | no |
| CommonMark §6.3 Links | inline link, relative `.md` destination | new: the 58 relinks (commit (1)), the five new stub links (commit (1)), the three existing stub links (commit (2)) — each an edge | `Memo.linked_files()` → `Population.__init__` (BFS) (checker plan `:409`, `:411`, `:573`) | ✓ one form: same directory, no fragment, no title | no |
| CommonMark §6.3 Links | inline link, relative `.md` destination | existing: the 84 links to the three carve-outs (P 11, moved 73), text unchanged | resolve from the new files (same directory) | ✓ counted in §3 | no |
| CommonMark §6.3 Links | not a link | a bare `§N` and a bare file name have no destination and add no edge (§0) — so a stub's backticked path alone would not keep its file in the population, and the heads' backticked plan path adds none | §1.2 stub link; §1.4 | ✓ | no |
| CommonMark §4.7 Link reference definitions | definition | the umbrella at `39cc365a` has **0** definitions, so no reference link resolves in it and none can cross a file boundary (a definition is matched within its own document): `git show 39cc365a:docs/plans/2026-07-vm-p4-es-language-completeness.md \| /usr/bin/grep -cE '^ {0,3}\[[^]]+\]:'` → 0, and with block-quote and list-item containers admitted (a definition inside a block quote registers — checker plan `:505`): `/usr/bin/grep -cE '^[ >*+-]*([0-9]+[.)] *)?\[[^]]+\]:'` → 0 on the same blob and on each of the nine family files at the production-form `<c2>` (the `][` sequences — 11 lines, 14 occurrences: `/usr/bin/grep -c '\]\['` → 11, `/usr/bin/grep -o '\]\[' \| wc -l` → 14 — are all inside code spans or fences) | checker plan `:412`, `:505` | ✓ | no |
| CommonMark §6.5 Autolinks | `<…>` URI autolink | an autolink's URI is absolute — a scheme of 2-32 characters, then `:` — so a relative `.md` destination can never be one. Measured in that form: same blob `\| /usr/bin/grep -cE '<[A-Za-z][A-Za-z0-9.+-]{1,31}:[^<> ]*>'` → 0, and the simulated heads and stubs (§2.2) and the relinks add no `<`. Independently, the checker lexes an autolink and masks it whole (checker plan `:575`, row §6.5), so even one would add no edge | — | ✓ | no |
| CommonMark §6.1 Code spans | relink exclusion | a `§N` inside a code span stays verbatim (4, all in PLAN); a backticked path is a code span, not a link | §3 relink rule; §2.1's link-undo regex reverses exactly the relink form (checker plan `:413`, `:571`) | ✓ | no |
| CommonMark §4.2 ATX headings | heading line, then **local policy** | a heading's text is inline content to CommonMark; that a `§N` in a heading line is **not relinked** is §3's own count rule (local policy), not a consequence of §6.1 | §3 (checker plan `:500`) | ✓ | no |
| GFM §4.10 Tables | recognition (header row + delimiter row) | the four schema tables move verbatim, so each header still matches its `SCHEMAS` entry; a drifted header makes a non-schema table (#562 Known remainder (ii)), which is rc 2 only through presence or an unbound kind claim (§1.3) | `plan_memo_tables.SCHEMAS`, `Memo.tables`; guard (ii) (checker plan `:415-417`, `:510`) | ✓ four tables, one memo each (§1.3) | no |
| GFM §4.10 Tables | cell content | the cells that name rows of the other table (§4: 43 §5 `Slot` cells, 24 §8 `Trigger` cells) are not edited; no per-cell link | §4 | ✓ | no |


This memo's own checks are not spec rows and are not counted in K or M:

- **§2.1 byte proof** (commit (1)): each child = its range; the entry, relinks undone = kept lines +
  exactly the four §1.2 stubs, byte-exact with links, in section order; six negative controls.
- **§2.2 checker output** (both heads): rc, the four counts and the footer compared; commit (2)
  byte-identical to commit (1).
- **§1.5 family guard**: precondition `misses == []` in `setUp`; (i) population = the nine memos;
  (ii) one table per schema, in the pinned memo, with the `SCHEMAS` keys pinned.
- **§3 step 1 / step 2 checks**: the relink census (58 on 51 lines, per target) and the commit-(2)
  check against the edit spec (byte-exact, CLAUDE.md included).

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
