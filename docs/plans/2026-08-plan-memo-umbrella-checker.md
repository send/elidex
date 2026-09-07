# Umbrella plan — `plan-memo-umbrella-check` carved out of #506 into a 2-slice prerequisite program

**Status**: plan-review **converged** 2026-08-22 (IMP 16 → 10 → 3 across three rounds; R3's three were mechanism decisions, applied below; remaining MINs applied). Implementation order: Slice 0 → Slice 1 (this PR) → Slice 2. **Implementation record (Slice 0 `718626e9`, Slice 1 `7931798d`)**: premises of this plan the implementation found false are marked ⚠ inline below; measurements in §6 are the re-run values. Branch `vm-p4-plan-memo-checker` (worktree
`elidex-wt-vmp4checker`, base `origin/main`). Files carried verbatim from #506 @ `190d2adb` **at the
carry commit `5e9439b4`** (`git diff --quiet 5e9439b4 190d2adb -- .claude/tools/` = identical there, not
at HEAD): `.claude/tools/plan-memo-umbrella-check.py` 811 lines, `plan_memo_tables.py` 407,
`plan_memo_umbrella_selftest.py` 396 (`wc -l`, 1,614 total). At HEAD of this PR the program is nine
`.py` files: `plan-memo-umbrella-check.py` 508 / `plan_memo_tables.py` 348 / `plan_memo_umbrella_selftest.py`
630 (the three carried names, 1,486) + `plan_memo_lexer.py` 439 / `plan_memo_blocks.py` 666 / `plan_memo_memo.py` 617 /
`plan_memo_roles.py` 398 / `plan_memo_selftest_cases.py` 611 / `plan_memo_selftest_cases_pr510.py` 538 /
`plan_memo_selftest_conformance.py` 172 / `plan_memo_selftest_mutants.py` 936 — **5,863 total, measured on the tree of the
design re-gate 3 fix commit (parent = the `Memo` split commit `5c74cef7`, §7); re-run at landing** (`wc -l
.claude/tools/plan*.py`, re-run before each push; a figure here is stale the moment a file is touched). No `crates/` change.
**Discharges** slot `#11-plan-memo-umbrella-checker-prereq` (registered 2026-08-22 in
`memory/project_open-defer-slots.md`; its "1,449 LoC" describes neither the carry (1,614) nor the program
this PR lands (4,771 on the tree of the commit after `1840251b`) — premise-correct the ledger to the live `wc -l` at landing) — **CLOSE −1 at landing of Slice 2**.

## §0 Why a separate program

CLAUDE.md *Edge-dense work*: a subsystem with no canonical algorithm needs plan-review **before**
implementation, is split into an umbrella + per-PR plans, and is not bundled into a feature PR.
The checker grew inside #506's converge loop without any of that: 21 commits touch `.claude/tools/`
(`git log --oneline origin/main..190d2adb -- .claude/tools/ | wc -l` = 21, re-run 2026-08-22).
Codex then spent five consecutive rounds with IMPORTANT findings in the checker — R90b 2 / R91 1 /
R92 1 / R93 2 / R94 6, each a different class of one hand-rolled Markdown + prose grammar — and in
the last three (R92–R94, `gh api` review batches 18:45Z / 18:56Z / 19:13Z) the checker was the *only*
subject: every thread under `.claude/tools/`, 0 memo threads (R91's batch also carried 2 memo threads). #506's loop cannot terminate while the tools sit in its diff.

**Why two slices, not one**: §2 names six invariants. The lexical half (I-A/B/C) has a canonical
algorithm (CommonMark 0.31.2 + GFM 0.29) and is implemented by construction; the prose half (I-D/E)
has none and is a seed with declared misses. They are reviewed against different references, so
they are different PRs. Slice 1 is what #506 needs to land (the tool, structurally sound); Slice 2
closes the three R94 threads that are predicates (#4/#5/#6).

## §1 Charter (whole program)

One gate over a plan memo and the memos it links: (a) the §5 / §8 row-kind census read from each
row's own declaring field; (b) a **seed** (declared misses, not an inventory) of prose/cell sites
that name a no-owner row (umbrella or kind-undetermined) as an owner, ordering subject, or
acceptance holder. Exit 0 = mechanical assertions clean / 1 = mechanical finding / 2 = schema miss
or unscanned population (never a clean exit for "could not scan"). Seeds never gate.

**Interim connection on `main`** (the checker's subject memo lands with #506, later): Slice 1 adds
`.claude/tools/plan-memo-umbrella-selftest-trip-wire.sh` (runs `python3 … --self-test --mutants`;
memo-independent — fixtures live in `tempfile` dirs, 0.08 s) **and registers it in `REQUIRED_WIRES`
in `scripts/trip-wires.sh` in the same commit** — the driver diffs the glob against that list in both
directions and FAILs an unregistered wire (list `scripts/trip-wires.sh:85-90`, diff + FAIL
`:208-235`; measured: a stub wire → rc 1 "ran but are not registered"). Two documentation claims become false and are updated in the
same commit: `.github/workflows/ci.yml:152` "No toolchain step: the wires are grep-only" (this wire
needs `python3`, present on `ubuntu-latest` but named nowhere in the workflow — state the dependency
there) and the wire inventory comment in `mise.toml:103-106`. A CLAUDE.md *Development Rules*
sentence names the tool + its trip-wire. The memo-specific invocation clause stays in #506. **#506's
concurrent-branch obligations at its `git merge origin/main`** (both branches touch the same paths;
neither is resolved by the merge driver): (a) the three pre-split tool files #506 carries
(`plan-memo-umbrella-check.py` / `plan_memo_tables.py` / `plan_memo_umbrella_selftest.py`, last touched
there at `190d2adb`) conflict modify/modify against this PR's rewrite of the same names — #506 must
**DROP its side** (`git checkout --theirs`, i.e. take `main`'s) rather than resolve hunk by hunk, since
its versions are the pre-Slice-0 monolith this program replaces; (b) `CLAUDE.md:54` is duplicated: #506's
"Plan-memo checker" bullet must **shrink to the memo-specific invocation clause only** (which memo, when to
run it, what its exit means for that memo) — this PR's bullet carries the tool description, exit codes and
the trip-wire — and must **reword** its "だから `trip-wires` には入れない" sentence (true of the memo run,
false of the self-test wire once this lands). The file header's "branch `vm-p4-plan-doc`" / "home is
CLAUDE.md" / "duplicated four ways" lines and the two `§6.6` docstrings (`plan_memo_tables.py:116/153`,
CommonMark 0.31.2 §6.6 = Raw HTML) are rewritten in Slice 1.

## §2 Coupled invariants (edge-dense)

- **I-A Lexical masking** — text inside fenced blocks and inline code spans is never read as a
  link, as prose, **or as a kind marker** (today `umbrella_ids` / `_attributed_to_other` read the
  declaring field raw, so a *quoted* `` `UMBRELLA, not a terminal unit` `` contaminates the census —
  probe: fixture row 7z quoting the phrase joins `umbrella_ids()`); code spans are lexed over the
  **block's inline content** (paragraph / cell), never per line — CommonMark §6.1 spans may contain
  line endings, and per-line pairing flips parity on every line a span crosses (measured at
  `190d2adb` with `awk '{n=gsub(/\`/,"\`"); if(n%2==1)print FILENAME":"NR}'` over the four memos,
  fence lines excluded: **10** prose line-pairs in the main memo + 2 in `umbrella-review-rounds.md`;
  `--worklist` at review-rounds 276–277 lists 7 mentions of which **4** sit inside one span
  (276: 0b/0c/1a, 277: 1b) and are artefacts, while review-rounds:115 `"Slice 4"` is masked today by
  a per-line pairing and becomes a real mention — net delta of the 706 under block lexing was
  predicted **−3**; ⚠ measured **−4**: this plan missed a second multi-line span at review-rounds:114
  (`` `Deps: 1b (I-3\nhelper)` ``) whose `1b` is now masked); a line is a reporting coordinate only. A **paragraph** ends at ONE
  block-boundary predicate (`block_end`, re-gate 2026-08-23, each member checked against commonmark.js
  0.31.2): a blank line (§2.1), a raw-extent opener (`raw_opener`: a fence §4.5, or an HTML-block
  opener §4.6 — types 1–6 anywhere, type 7 only where **no paragraph is open**; that bit is the
  predicate's ONE context argument and is the Phase-1 driver's block state — a run open or not —
  never a look at the previous raw line (R11: the look-back `blank / raw / one-line block before it`
  read `<span>` after a table's rows as a body row, and after `text\n===` as paragraph text; by the
  oracle `# h\n<span>`, `text\n===\n<span>` are a heading and a raw block, `text\n<span>`,
  `[foo]: /url\n<span>`, `- item\n<span>` one paragraph, and GFM §4.10 breaks a table at "the
  beginning of another block-level structure" — a table is not a paragraph)), a paragraph-interrupting
  block start (`starts_block`) —
  ATX heading (§4.2), thematic break (§4.1), list-item line (§5.2; ⚠ local policy,
  stricter than the spec's empty-item / ordered-from-1 interruption rules), `>` line (§5.1: the
  container opens; R13) — a setext underline **where a paragraph is open** (§4.3; the paragraph becomes the
  heading; the same `para_open` bit as type 7: with no paragraph open there is nothing to underline,
  so after a table's rows `===` is a one-cell body row (GFM §4.10 Example 202 — the width miss fires)
  and `---` the thematic break §4.1 already names; R12: the unconditional arm let a malformed memo
  exit 0. The driver adds the paragraph-head rule: a run headed by a list-item line (Example 94) is not a
  paragraph an underline closes — `container_text`) — or a GFM table header
  (§4.10; ⚠ local policy over pure CommonMark, which has no tables). NOT a boundary, by the same
  oracle: an indented line while a paragraph is open (§4.4 cannot interrupt a paragraph) and a
  reference definition (§4.7 cannot interrupt a paragraph). **R13 — the §4.4 / §5.1 root**: an
  indented line where NO paragraph is open is the third RAW opener (`raw_opener` `("indented", None)`,
  the same `para_open` bit as type 7 — so after a table's rows it opens a block, like `<span>`; cmark-gfm
  agrees, MEASURED at design re-gate 3 — `gh api -X POST /markdown -f mode=gfm -f text=$'<table>\n    | a | b | c | d |'`
  renders the table and then `<pre><code>| a | b | c | d |</code></pre>`, a tab-indented row the same — ⚠ an
  earlier draft of this sentence and of `raw_opener`'s docstring said "cmark-gfm's row continuation would
  read a row"; that was never measured and is FALSE. **The four table ends, each cmark-gfm's reading by the
  same command**: `<span>` → an HTML block; an indented or tab-indented line → `<pre><code>`; `===` → a
  one-cell body row; `---` → `<hr>`. The ONLY local policy at a table's end is I-C's width miss on that
  `===` row (exit 2 where cmark-gfm pads), never the boundary), and `raw_extent` runs it over the §4.4
  chunk (consecutive indented lines; a blank line stays inside when an indented line follows,
  Example 111; the trailing blank lines are not part of it), so `    foo\n---` is code + `<hr />`
  (Example 100) with no `container_text` arm for it. A block quote is a CONTAINER (`Memo._quote`):
  every marker line stripped by `quote_content` (§5.1's marker, incl. the §2.2 tab rule in LINE
  columns — `>\t\tfoo` is code holding `  foo`, Example 6; `>  \ta` a paragraph) and the lines
  without a marker gathered as **lazy continuation candidates**, then the SAME `_parse` over the
  content, so a definition inside registers (Example 218), a table inside is a table, a paragraph
  inside is a paragraph at its real line, a nested quote the same again. Laziness goes through the
  ONE predicate: `block_end(lines, i, para_open, lazy)` reads a candidate as a boundary outright
  where no paragraph is open (the quote ends after a raw extent, a table, a heading: `> ```\nlazy`,
  `>     foo\n    bar`, `> # h\nlazy`) and, with one open, by every arm EXCEPT the setext underline
  ("cannot be a lazy continuation line": `> foo\nbar\n===` is one paragraph, Example 93; `> Foo\n---`
  a quote and `<hr />`, Example 92); `raw_extent` and `table_header_at` read the same list (a
  candidate ends a fence in the quote; the DELIMITER row may not be lazy — `> | a |\n|---|` is a
  paragraph — while a lazy HEADER row is the table's header where a paragraph is open: `> a\n| h |\n>
  |---|\n> | 1 |` is quote[p(a), table(h; 1)] — cmark-gfm's reading, measured at design re-gate 3 (⚠
  this sentence once said "neither table row may be lazy", half false); the driver hands that lazy line
  to `admit_table` instead of ending the quote, the one boundary a lazy line can be with a paragraph
  open, since it is the only `block_end` arm that needs the next line. Measured divergence, not modelled:
  after a reference DEFINITION cmark-gfm still hands the lazy line to the table and prints the definition
  as a paragraph — `> [a]: /u\n| h |\n> |---|`; here a definition is a block of its own and the quote
  ends). Each expectation above was checked against commonmark.js 0.31.2,
  and the block-sequence control in the runner (`Memo.sequence`, the pass's own statement of what it
  read) holds the shapes the vendored examples do not reach. Every block start reads its "up to three spaces of indentation"
  through ONE measure, §2.2 tab stops (`indentation` / `unindented` / `is_indented`: a tab advances to
  the next multiple of 4, so `\tfoo` and ` \tfoo` are indented code and `\t| a |` over `\t|---|` is
  not a table — R12: every pattern spelt ` {0,3}` / ` {4,}` itself and read a tab as one column). The
  same predicate bounds a definition's continuation lines (the run
  it is parsed over, `run_end`: paragraph open) and a GFM table's body (`admit_table`: no paragraph
  open), so `[foo]:\n---` is a setext heading, `[foo]:\n#` /
  `>` / `***` a paragraph and a block, `[foo]:\n    code` a definition, `[foo]:\n|a|b|\n|--|--|` a
  paragraph and a table. Phase 1 is ONE forward pass (`Memo._parse`), the Appendix's own shape: each
  line classified once with the open block in hand, re-entered once per block quote over the quote's
  content. The scanners read the block's ONE disposed
  stream too (R12: `_anchored` / `_bare` matched on the raw text and `` `Slice `C `` was anchored
  across the mask boundary; `Block.masked` is gone).
  CommonMark: a span never crosses a block boundary; control = backtick opened in one list item and closed in the next ⇒
  literal). When a `|` inside backticks splits a row, each half holds an unmatched backtick string
  (§6.1 ⇒ literal) and is scanned as prose — control: umbrella id on each side ⇒ 2 mentions. **Disposition exception**: a code span whose content is only row ids (and
  separators) is the document *spelling* an id and IS a mention (existing `code_spans(keep=…)`,
  POSITIVE control). So masking is not id-independent — see order below.
- **I-B Link grammar** — every CommonMark §6.3 link form (inline: plain / `<dest>` / fragment /
  title / balanced parens; reference: full / collapsed / shortcut with §6.3 label matching =
  casefold + strip + collapse internal whitespace; definitions per §4.7 incl. `<dest>`) yields its
  destination; the `.md` file part of each destination, **transitively** over linked memos, is the
  sibling population; duplicate definitions: **the first in the document wins** (§6.3; today a dict
  comprehension makes the last win). An absent target = exit 2. Link **text** is scanned as prose
  (B×D, a stated deviation); ⚠ a reference *definition* line renders nothing and is masked whole,
  label included. ⚠ The `#11-` slot-id pass reads through code spans (a backticked slug is the
  document spelling an id — the same disposition exception as I-A); fences and link tails mask it.
- **I-C Table admission (local policy over GFM §4.10)** — GFM: header and delimiter row must have
  equal cell counts (else no table); body rows "may vary" (short padded, long truncated); leading
  pipe optional. **This checker overrides the "may vary" clause**: a body row whose cell count ≠
  header is a schema miss (exit 2), never a silent skip or a shifted read — because a shifted read
  fabricated a mechanical finding in a probe (long row → spurious `UMBRELLA-MARK`). Admission is
  decided **once** in `admit_table` (named `find_tables` until R11: the driver now finds the header,
  `admit_table` admits the table at it); every per-consumer width guard is deleted — **nine** today
  (`grep -n 'len(cells)' .claude/tools/*.py` at `190d2adb`: `tables.py:334/374/400`,
  `check.py:333/460/496/533/586/617`; `tables.py:54 is_separator` is structural and stays), and the
  acceptance is that grep returning only `is_separator`, not this list (8 are skips, `check.py:333`
  is a ternary degrade to `None` — same class). **Cell shape**: `admit_table` is the single writer
  of a row's cells and emits **body cells** (optional leading/trailing pipes stripped at admission,
  per GFM); `SCHEMAS` column indexes are re-based to body columns, `scan_tables` offsets are computed
  from each body cell's raw start — no reader re-splits the line (today every reader indexes the
  raw split incl. the leading empty cell, so an admitted pipe-less row would shift every read by
  one: the I-C class again). Control: the same table written without leading/trailing pipes yields
  the same ids and the same reported spans. The row lexer applies to **every** GFM row (≈154–158
  non-schema `|` rows exist today; `grep -c '^|'` minus schema rows): non-schema tables enter the
  `Population` with `schema=None` (cells lexed, no `self_id`, no width policy — the exit-2 width rule
  is for schema tables only, whose cells the checker reads).
- **I-D Naming population = complement** — an id mention is reported unless attached to a licensed
  construction; a licensed prefix does not license a later clause of the same sentence; a row's
  mention of itself is classified like any other (the existing NEGATIVE control "a row naming itself
  in its own cell" **flips to POSITIVE** when the role is forbidden; the licensed `mints its
  children` form stays NEGATIVE).
- **I-E Kind attribution (a closed grammar of predication — mechanical, not a seed)** — a kind
  marker in a row's declaring field is *self-declaring* **unless** it is the predicate of a clause
  whose subject is another row's id, and a clause is recognised only by a **predication connective**
  between that subject and the marker: ` — ` (appositive dash), ` is ` (copula), `, which is `
  (relative). Binding: the subject is the **nearest preceding `row-noun + id` with nothing but
  emphasis marks between the id and the connective** (`Slice **9z** (child of Slice **7z**), which
  is …` ⇒ no binding ⇒ self-declaring; control). No connective ⇒ self-declaring: `Unlike Slice 7z,
  **UMBRELLA…**` (existing NEGATIVE control; a proximity rule fails it — measured, and
  `tables.py:351-358` records the same failure once before). The connective set is the **whole**
  accepted grammar — closed, each member a control — because the attribution result gates today
  (`check.py:440-444` appends `UMBRELLA-MARK` ⇒ rc 1) and therefore cannot be a §1 seed: a false
  attribution is a false rc 1, a missed one lets a pointer row into the census where assertion (b)
  can fire. Controls therefore include the title-shaped false-attribution NEGATIVE (`**Slice 9 —
  UMBRELLA…**` where `Slice 9` is a *title*, not a row id … verified against the 48 real declaring
  fields: all are `**<title> — UMBRELLA…**` with no row-noun+id, 0 attributions under both the old
  and the new rule). ` was ` is not a connective (no motivating instance in the population;
  `Slice 9z was **UMBRELLA…** until R3` in 9z's own field must stay self-declaring — NEGATIVE control).
  ⚠ **Slice-1 delta in I-E territory** (`/code-review high` F4, `2ff35065`, kept): `attributed_to_other`
  now reads the **first** marker occurrence in the field and that occurrence alone decides — a field that
  declares itself and then says a sibling "is not it" is self-declaring, and a later occurrence never
  overrides the first ("first marker occurrence decides; self-declaring ⇒ None"). Slice 2's connective
  grammar is written over that rule, not over "any occurrence".
- **I-F One pipeline, one population** — `check(memo)` is the only entry (⚠ implemented as a `Result` NamedTuple whose first three fields are `findings, notes, rc` — the report also needs `mentions` and `population`);
  `main()` and `--self-test` both call it. A single transitive `Population` (tables, row ids, census)
  is built once and is the only input of the mention scan, the code-span keep-set (`all_ids`,
  `check.py:658` today main-only) and the four assertions (`memo.data_rows`, `check.py:459-616` today
  main-only) — probe: a sibling-declared terminal id in an id-only run masks the umbrella next to it,
  and a sibling umbrella with a `Deps` edge is never asserted. Sharing the pipeline gives the
  gating stages reachability, not proof, so §6 binds a control to each of them explicitly. Mutation
  proof is **re-executable**: a `MUTANTS` table (name → exact source substring → replacement →
  control expected red); `--self-test --mutants` loads the module from the patched source text via
  `importlib` (`spec_from_loader` + `exec_module` on a string), a substring that no longer applies is
  a **FAIL** (never "survived"). Mechanics for the three-module layout after Slice 0: a mutant names
  its module; the patched module is exec'd from the patched source and installed in `sys.modules`
  under its import name **before** the checker module is loaded; every mutant gets a fresh module
  set; the substring must occur **exactly once** (`count == 1`, else FAIL — `len(cells)` occurs 10×
  today). The trip-wire runs both `--self-test` and `--mutants`. **Duplicate declarations**: the
  transitive `Population` is a map `id → (kind, declaring memo)`; the same id declared in two memos is
  a schema miss (exit 2 — "could not scan" is never clean); link cycles (A→B→A) are walked with a
  visited set and are not an error. Controls for both.

**Lexing order (settles A×B, A×C, A×D, A×E)**: document → fenced blocks masked (CommonMark §4.5) →
blocks: a GFM row is split on *raw* unescaped `|` (GFM §4.10 prose: escaping works "including
inside other inline spans"; that an *unescaped* `|` inside backticks splits is inferred from
Example 200, `` `\|` `` → `<code>|</code>`; `\|` becomes `|` in cell content) → per block inline
content (paragraph or cell): code spans (CommonMark §6.1, backtick strings of equal length) → links
(CommonMark §6.3 / §4.7) → **row ids read from the raw id cell** (15 of the 48 census ids are
backticked `#11-` slugs; `bare_id` strips them itself, so ids never depend on the mask — no cycle) →
code-span disposition (id-only run ⇒ mention) → kind markers read from the masked declaring field →
scanners read that stream.

Intersections: A×B = links come only from the masked stream (R94 #2); A×C = cell split precedes
masking (GFM order; a masked stream fed to the splitter would break I-C); A×D = id-only spans are
mentions, so disposition needs ids, so tables precede disposition; B×C = a sibling's tables join
the **census** and the scan (today the census is main-only: `no_owner_ids` at `check.py:709`,
probe: umbrella declared in a sibling is invisible) — Slice 1's `Population` is transitive and is
the single input of census, keep-set and assertions (I-F); A×E = the kind-marker reader reads the
masked declaring field (a quoted marker is not a declaration); B×D = link labels are prose; C×D = a row of wrong width has no cells to scan ⇒ exit 2; D×E = a pointer
row's attribution text is a licensed mention of the attributed id; D×F / E×F = each licensing arm
and each attribution spelling has a positive control and a mutant.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (site name @ `190d2adb`; implemented as `plan_memo_lexer.py::link_destination` / `link_title` / `inline_pass` (Phase 2) and `plan_memo_blocks.py::reference_definitions` / `fenced_lines` / `split_row` + `delimiter_width` (Phase 1; split from the lexer at the 2026-08-23 re-gate, touch-time 1000-line rule)) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CommonMark §6.3 Links | inline link | bare destination = nonempty, not starting with `<`, no space / ASCII control, parens balanced or escaped; `<dest>` = no line ending, no *unescaped* `<`/`>`; backslash escapes ASCII punctuation only (§2.4) | `plan_memo_tables.py::_link_destination` | ✗ (`isspace`/`ord>31` ≠ spec classes; `\` skips any char) — Slice 1 | no |
| CommonMark §6.3 Links | inline link | title `"…"` / `'…'` / `(…)` with escapes | `plan_memo_tables.py::_link_title` | ✗ (`(` inside `(…)` title unguarded) — Slice 1 | no |
| CommonMark §6.3 Links | reference link | full / collapsed / shortcut (shortcut = label not followed by `[]` or a link label — ⚠ this plan once said `[a][undefined]` = shortcut `[a]` + literal; §6.3 / Example 571 say the opposite (571 = `[foo][bar][baz]` with `[foo]` defined and `[bar]` not: "`[foo]` is not parsed as a shortcut reference, because it is followed by a link label"; 570 is the full-reference case), `[undefined]` IS a link label so `[a]` is not a shortcut; implemented per spec with a NEGATIVE control); label = 1–999 chars, ≥1 non-blank; match = casefold + strip + collapse; duplicate definitions: first wins | `plan_memo_lexer.py::links` / `_reference_tail` (⚠ pre-split name was `plan_memo_tables.py::links`) | ✗ (collapse missing; collapsed label found by nearest `[`, not matching `[`) — Slice 1 | no |
| CommonMark §4.7 Link reference definitions | definition | label non-blank, no unescaped `[`; optional one line ending before destination; `<dest>`; nothing after destination/title | `plan_memo_blocks.py::reference_definitions` (⚠ pre-split name was `_REF_DEF`) | ✗ (`[ \t]*`, `[^\]]+`, `\S+`) — Slice 1 | no |
| CommonMark §6.1 Code spans | masking | opener/closer = backtick strings of equal length; unmatched strings literal | `plan_memo_tables.py::code_spans` | ✗ (next single backtick closes) — Slice 1 | no |
| CommonMark §4.5 Fenced code blocks (⚠ all lexer touch sites below live in `plan_memo_lexer.py` / `plan_memo_blocks.py`, not `plan_memo_tables.py` — tables.py would have crossed ~800 lines; seam = lexing vs inventory; `dispose` (mask disposition) stays in tables.py because it needs ids) | masking | ≥3 ``` or ~~~, not mixed; ≤3 spaces indent; closer same char, ≥ length, ≤3 spaces indent, only spaces/tabs after; info string of a backtick fence has no backtick; unclosed runs to EOF | (NEW) `plan_memo_tables.py::fenced_spans` | ✗ (absent) — Slice 1 | no |
| GFM §4.10 Tables | recognition | header/delimiter equal width else not a table; delimiter cell = ≥1 hyphen with optional leading/trailing colon; leading/trailing pipe optional; ends at blank line or block start | `plan_memo_tables.py::find_tables` / `is_row` / `is_separator` | ✗ (no width compare; `is_row` requires leading `|`; `[:\- ]*` admits empty / colon-only cells) — Slice 1 | no |
| GFM §4.10 Tables | cell split | unescaped `|` splits (incl. inside backticks, Example 200); `\|` → cell content `|` (backslash consumed); spaces between pipes and content trimmed | `plan_memo_tables.py::split_row` | ✗ (keeps `\|` with the backslash — then `` `9z \| 7z` `` is masked as code while `9z | 7z` is an id-only mention) — Slice 1 | no |
| GFM §4.10 Tables | body row width | spec: pad/truncate; **local policy**: ≠ header ⇒ exit 2 | `find_tables` (admission, one site) | ✗ — Slice 1 | no |
| **PR #510 rows (clauses the R1–R3 rounds added; ✓ = a named control exists)** | | | | | |
| CommonMark Appendix A "A parsing strategy", Phase 2 "look for link or image" | bracket stack | one left-to-right pass over `[` / `![` openers with an *active* flag; on `]` pop the nearest opener, try inline → full → collapsed → shortcut; a LINK deactivates every `[` opener before it ("links may not contain links", §6.3), an image does not; a failed opener is literal text | `plan_memo_lexer.py::links` | ✓ controls "(link) nested inline links: the INNER link is the link…", "(link) a reference link nested in inline brackets…", "(link) a link wrapping a REFERENCE image `[![alt][img]](child.md)`…", "links() is linear: 30 nested brackets are one inline_pass call" | no |
| CommonMark §6.4 Images | image | `![` opens an image (an unescaped `!` before `[`); not a link; destination never a sibling; alt text prose; tail masked as kind `image` | `plan_memo_lexer.py::_is_image` / `Lexed.images`; `plan_memo_tables.py::dispose` (`image`) | ✓ controls "(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)`…", "(image) `![alt][img]` with a definition is consumed whole…" | no |
| CommonMark §4.7 Link reference definitions | next-line title | the title may follow on the line after the destination; an invalid next line leaves the definition ending at the destination (one attempt, same-line and next-line) | `plan_memo_blocks.py::reference_definitions` | ✓ controls "(def) a next-line title is part of the definition…", "(def) a next-line title holding `[x](missing.md)`…", "(def) a next line that is NOT a valid title is prose…" | no |
| CommonMark §4.7 | orphan detection | exactly the spec-grounded class: a line that parses as a VALID definition but cannot take effect because it is not at a block start ("a link reference definition cannot interrupt a paragraph"); a label-and-colon line that is NOT a valid definition is prose (commonmark.js: `[C1]: ECMA-262 §1 says so` is a paragraph; `[sib]: child.md "title\n\nmore"` is a paragraph) and a shortcut naming it is exempt; linear — the runs are joined once, one parse per line | `plan_memo_memo.py::Memo._parse` (`Memo.orphans`; ⚠ pre-split name `plan_memo_tables.py::Memo._blocks`), `plan_memo_blocks.py::definition_block` | ✓ controls "(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan…", "Phase-1 orphan detection is linear: <= 4 link_label calls per line, t(4N)/t(N) < 8" | no |
| CommonMark §2.4 Backslash escapes | row split parity | only an ODD backslash run escapes a `|` (`a\\|b` is two cells); the trailing-pipe check reads the same parity (`_escaped`, one helper) | `plan_memo_blocks.py::split_row` | ✓ controls "(row) `a\\|b` holds an UNESCAPED pipe…", "(row) `a\|b` is one cell…", "(row) a trailing `\\|`…" | no |
| §5 (local policy over the disposition exception) | id-only code spans | an id-only run is tokenised by the declared-id GRAMMAR longest-first (a `#11-` slug is atomic; `` `#11-zz-alpha / 9z` `` spells two ids), with separators between tokens | `plan_memo_tables.py::id_only` (`_ID_RUN_TOKEN`) | ✓ control "(span) a `#11-` slug is ATOMIC in an id-only run…" | no |
| (no spec clause — a tokenisation fact of these documents) | bare `.md` file name | read by PATH SYNTAX: the maximal run of non-whitespace characters ending in `.md` (inline delimiters `[` `]` `<` `>` `` ` `` `\|` excluded so link text and code spans are not swallowed; parentheses only as a balanced pair), bounded by spaces / tabs / line ends or the cell edge; no trailing-punctuation rule is needed because the token ENDS at `.md` (the GFM §6.9 autolink rule is moot) | `plan_memo_lexer.py::_TOKEN` | ✓ controls "(file) `9z+notes.md`…", "(file) `9z@notes.md`…", "(file) `(9z).md`…", "(file) a link's visible text is not swallowed…" | no |
| CommonMark §2.1 / §4.9 / GFM §4.10 | ASCII classes at every boundary | a blank line = spaces or tabs only (§4.9); edge pipes and cell trimming use the same space/tab class; the far side of a `.` after a bare id is the ASCII id class (`9z.次の工程` reports `9z`); `is_empty`'s `isalnum` is deliberately Unicode (a letter in any script fills a cell) | `plan_memo_blocks.py::is_blank` / `split_row`, `plan-memo-umbrella-check.py::_glued`, `plan_memo_tables.py::is_empty` | ✓ controls "(span) an NBSP-only line is NOT blank…", "(table) a row opening with an NBSP…", "(bare) `9z.次の工程`…", "(bare) `9z.é`…" | no |
| CommonMark §6.3 | one label grammar | the text of a collapsed / shortcut reference is a label iff `link_label` reads it from the opener (no second walker); the full form carries `raw` out of `_reference_tail` | `plan_memo_lexer.py::_reference_tail` / `link_label` | ✓ control "(link) bracket text holding unescaped brackets is not a label (§6.3)…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 2 | one inline pass | code spans (§6.1) and brackets are recognised together, left to right; a backtick string opens a span as met and the scan jumps past it; an inline-link tail is parsed by lookahead on the RAW text (a backtick inside a destination is consumed by the link; one before the `]` opens a span that swallows it); no code pre-mask | `plan_memo_lexer.py::inline_pass` (`code_spans` / `links` are views over it) | ✓ controls "(span) a backtick inside a link DESTINATION is consumed by the link…", "(span) a backtick BEFORE the `]` opens a code span that swallows it…", "(link) a link inside a code span is not a link (A x B)" | no |
| CommonMark §6.4 Images | unresolved reference image | `![alt][missing]` is literal image syntax, never an unresolved memo reference (the opener's image flag travels with the unresolved record); the failed reference is recorded ONCE — its label bracket is re-scanned (the scan resumes after the literal `]`; §6.3 Example 571: `[missing][baz]` may be a link, so the tail is NOT consumed) but a shortcut it closes as is the same site, not a second record, so an orphan `[missing]: image.md` later in the paragraph does not turn `[missing]` into a memo miss (R11) | `inline_pass` (`relabel`) → `Memo.unresolved_references` | ✓ controls "(image) an undefined reference image `![diagram][missing-image]`…", "(image) the reviewer's input `![alt][missing]` + an orphan…", "(image) `![alt][missing][Slice 9z]` with `[Slice 9z]` defined…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1 → Phase 2 | block structure before inline structure | Phase 1 over RAW lines (`Memo`): fences, reference definitions as blocks of their own recognised only at a block start and parsed over the REST OF THEIR RUN (the lines up to the next `block_end`, joined once — §6.3 label up to 999 chars and may span lines, `link_title`; a valid definition inside a paragraph is an orphan, `Memo.orphans`), GFM tables ending at the same `block_end` (a reference definition right after a table is a ROW of it — GFM Example 202 — not a definition), paragraphs; Phase 2 (`inline_pass`) over each paragraph's / cell's content only, with `defs` from Phase 1 | `plan_memo_memo.py::Memo._parse` (its `definition_at`); `plan_memo_blocks.py::definition_block` / `run_end` | ✓ controls "(def) a definition is read from RAW lines at a block start…", "(table) a reference definition right after a schema table ENDS the table…", "(def) a definition cannot interrupt a paragraph…", "Phase-1 orphan detection is linear…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1; §4.4; §4.6; §5.1; GFM §4.10 | block start = block state | "is this line at a block start" is the driver's state (a run open or not), the ONE context bit `block_end` / `raw_opener` take; a type-7 HTML opener or an indented line after a table's rows, a one-line block or a setext heading opens a raw extent; after a run line (paragraph text, a definition, a list-item line, a quote's lazy line) it is text — no caller looks back at the previous raw line (R11); inside a block quote the same state, re-entered (R13) | `plan_memo_blocks.py::raw_opener` / `block_end` / `run_end`, `plan_memo_memo.py::Memo._parse` / `_quote`, `plan_memo_tables.py::admit_table` | ✓ controls "(table) a type-7 HTML opener right after a schema table ENDS it…", "(html) `# h\n<span>\n9z owns it`…", "(html) `Heading\n===\n<span>\n9z owns it`…", "(html) `[sib]: slice-9z-sib.md\n<span>\n9z owns it`…", "(html) `- item\n<span>\n9z owns it`…" | no |
| WHATWG URL (scheme before decoding) + **local policy** (CommonMark §6.3 / GFM say nothing about siblings on disk) | ONE destination → sibling resolver | `Memo.sibling_path`, stages in spec order: (a) scheme on the RAW path (`notes%3Achild.md` is the file `notes:child.md`), (b) percent-decode (`slice%20sib.md` = `slice sib.md`), (c) reject absolute `/` / `//` and C0 control / DEL (`child%00.md` would make `resolve()` raise), (d) `.md`, (e) `_resolve` = `resolve()` with `OSError` or (Python 3.9–3.12 symlink loop) `RuntimeError` read as an UNAVAILABLE sibling, reported by the population's one I/O chokepoint (`Memo()` under `OSError | RuntimeError | UnicodeDecodeError`, `read_text(encoding="utf-8")`) as the exit-2 unavailable-memo miss; `linked_files` dedups with a set | `plan_memo_memo.py::Memo.sibling_path` / `_resolve` / `linked_files`, `Population.__init__` | ✓ controls "(link) `notes%3Achild.md` has no scheme…", "(link) a percent-encoded destination…", "(rc) a percent-encoded ABSOLUTE destination…", "a decoded destination with a C0 control character is rejected…", "an OSError from resolve() is the unavailable-sibling schema miss…" (OSError and RuntimeError injected), "an undecodable sibling is the unavailable-linked-memo schema miss…", "linked_files scales linearly…" | no |

### §3.0 Block grammar — the spec's CLOSED list (the bound IS this table)

CommonMark 0.31.2 enumerates its block types in §4 (leaf blocks) and §5 (container blocks); GFM 0.29
adds §4.10 tables. Every type has a disposition here: **LEXED** (a Phase-1 clause with a control — a
leaf, a RAW extent, or the §5.1 container) or **LEXED-FLAT** (§5.2 only: the marker line starts a
paragraph; nesting and laziness are not modelled). A `[LEX-UNSUPPORTED?]` SEED names a RAW line the lexer
never inline-parses — an HTML-block line (§4.6) or an indented-code line (§4.4; a fence excepted, the
author's explicit code marker) — when it holds a `|` or a declared id, under ONE seed rule (design re-gate
3 IMP-2: an indented schema row after a table's rows is raw under cmark-gfm too and left the census
SILENTLY, rc 0 and no seed — the I-C class — while a raw HTML line holding a `|` was seeded), printed with
its READING; indented code opened while a list item may still be open (the §5.2 row's item-open bit) is
reported as the item's content under CommonMark, the one place the flat list reading hides prose: a seed
in the ORDER-PROSE? idiom, never an inventory. Section numbers
are the spec's own (`.claude/tools/webref specs commonmark` = no source: webref carries no CommonMark
extract, so this list is cited from the 0.31.2 text directly, re-verified 2026-08-23).

**The falsifier of every row is the spec's own example list** (R12): `spec.commonmark.org/0.31.2/spec.json`
is official and machine-readable, and its examples for `Tabs` §2.2, §4.1–§4.9 and — since design re-gate 3
(MIN-6) — `Block quotes` §5.1, Examples 228–252 (221 of the 652; fields `example` / `section` / `markdown` /
`html`, 37 KB) are vendored in
`.claude/tools/commonmark-0.31.2-block-examples.json`. The control `plan_memo_selftest_conformance.py`
(every `--self-test` run) puts each through `Memo` and consumes the expected html against Phase 1's
**block sequence** — since R13 the pass's OWN statement of what it read (`Memo.sequence`: `[kind, first
line, last line]`, a quote before its content), nothing re-derived from dropped lines — a block quote
`<blockquote>` … content … `</blockquote>`, a raw HTML extent verbatim, an indented / fenced extent
`<pre><code…</code></pre>`, `<hr />`, `<hN>` at the `#` count or 1/2 for `=`/`-`, `<p>…</p>`, a definition
nothing, the html exhausted at the end — never a rendering (Phase 2 is skipped over). Excluded by
predicate over Phase 1's own output, printed per run — ONLY a paragraph whose FIRST line is a §5.2
list-marker line (design re-gate 3 MIN-4: a marker line LATER in a paragraph means `starts_block` failed to
end the run before it, a Phase-1 defect, so such an example is aligned and FAILS — the broader "any marker
line in a paragraph" predicate had hidden the `starts_block` list-item mutant, which now reds this control
at 4 examples; a `>` line read as prose would be a FAIL, not a re-classification): 13 (4 5 7 9 57 60 61 94
99 108 109 175 235): **208 aligned / 13 excluded / 0 FAIL** (184 / 12 / 0 over the 196 before §5.1 was
vendored). Before R13 the count was 169 / 27 / 0, and the 27 were
exactly where R13 landed: 8 block-quote examples (6 92 93 101 128 174 214 218) and 7 §4.4 chunk shapes
(85 110 111 112 114 115 225) under the old PROSE-AS-WRITTEN dispositions, plus the 12 list examples;
the two dispositions were replaced by the grammar (§4.4 RAW, §5.1 container) and the exclusions went to
0 for both. What remains excluded is §5.2 alone, by its stated reason (LEXED-FLAT), and no vendored
example forces it. A FAIL there is a Phase-1 defect or an unstated disposition, never a control
rewrite; the mutant that drops `div` from the type-6 list (`search`, the reviewer's example, has no spec
example) reds it at 153–161/185, the four-space literal at Tabs 1–2, the §4.4 opener arm at 100 among
others, the marker's tab rule at 6, the lazy setext arm at 93. Two rounds of spec-table transcription
errors in both directions preceded this control.

| § | Block type | Disposition | Phase-1 site | Control |
|---|---|---|---|---|
| §2.2 | Tabs | LEXED — ONE indentation measure: a tab advances to the next multiple of 4 columns; every block start reads "up to three spaces" and §4.4 "four or more" through it (`\tfoo`, ` \tfoo`, ` \t# foo` are indented code; a tab-indented header/delimiter row is not a table) | `indentation` / `unindented` / `is_indented` (`_match`, `fence_opener` / `fence_closes`, `html_block_type`, `table_header_at`, `reference_definitions`) | "(lex-seed) `\tSlice 9z owns it`…", "(lex-seed) ` \tSlice 9z owns it`…", "(table) `\t\| Slot \| …`…"; spec examples Tabs 1–3, 8–11, 82 |
| §4.1 | Thematic break | LEXED — one-line block, ends a paragraph / table; `---` after paragraph text is a §4.3 underline instead (Example 59); after a table's rows it is the thematic break (no paragraph is open) | `one_line_block` (`_THEMATIC`) | "(span) a paragraph ends at an ATX heading" family; "(setext) a `---` after paragraph text…"; "(table) `---` right after a schema table is a thematic break…"; spec examples |
| §4.2 | ATX heading | LEXED — one-line block; its text is inline content | `one_line_block` (`_ATX`) | "(span) a paragraph ends at an ATX heading"; spec examples |
| §4.3 | Setext heading | LEXED — paragraph text + `=`/`-` underline; the underline ends the paragraph and is not content; a boundary ONLY where a paragraph is open (`block_end`'s `para_open` arm: after a table's rows `===` is a body row) and never on a lazy line of a block quote (Example 93, the `lazy` arm); not after a run headed by a list-item line (Example 94) — `container_text` (a `>` line heads no run and neither does an indented line: the container / the raw extent, so Examples 92–93 and 100 hold by the grammar) | `block_end` (`is_setext_underline`), `Memo._parse` (`container_text`) | "(setext) `Heading\n===`…", "(setext) `==` after a list item is NOT an underline…", "(table) `===` right after a schema table is a one-cell body row…", "(setext) a bare `===` IS a paragraph…", "(quote) `> Heading `open\n===\nSlice 9z owns it` here`…"; spec examples |
| §4.4 | Indented code block | LEXED — a RAW extent (R13), exactly like a fence: opened by a line of four or more columns where NO paragraph is open ("cannot interrupt a paragraph": `[foo]:\n    code` is a definition, `text\n    x` a paragraph; after a table's rows, a one-line block, a heading or a blank line it IS a block start — the same `para_open` bit as type 7), running over the §4.4 chunk (consecutive indented lines; a blank line stays inside when an indented line follows it, Example 111; the trailing blank lines are not part of it), never inline-parsed; SEEDED under the one raw-line rule when a line holds a `\|` or a declared id (design re-gate 3 IMP-2; a fence is not — the author's explicit code marker), with the reading `item` while a list item may be open (§5.2 row) — cmark-gfm agrees that an indented row after a table is `<pre><code>` (measured) | `raw_opener` (`("indented", None)`, `is_indented`) / `raw_extent`, `Memo._parse` (`raw`) | "(indented) `    Slice 9z owns it` at a block start is a raw extent…", "(indented) `text\n    Slice 9z owns it`…", "(indented) `\tSlice 9z owns it`…" (×3), "(table) an indented line right after a schema table opens an indented code block…", "(lex-seed) `para\n\n    Slice 9z owns it`: … seeded by the one raw-line rule", "(lex-seed) a 4-space-indented slot row after a table's rows … is SEEDED", "(lex-seed) a tab-indented slot row …", "(lex-seed) an indented-code line with neither a `\|` nor a declared id is no seed"; block-sequence control (chunk shapes, raw content asserted); spec examples 85 107–118 225 aligned, 100 with `---` |
| §4.5 | Fenced code block | LEXED — a RAW extent: never inline-parsed, always a run / paragraph / table end; one opener rule (`raw_opener`) and one extent rule (`raw_extent`) shared with indented code and HTML blocks, consumed in place by the driver; an unclosed fence runs to the end of its containing block (the document, or the quote's first lazy line) | `plan_memo_blocks.py::fence_opener` / `raw_opener` / `raw_extent` | "(fence) …" family (10 controls); block-sequence control (`> ```\nlazy`) |
| §4.6 | HTML block | RAW (+ SEED) — a leaf block "treated as raw HTML", exactly like a fence: its lines from the opener to the §4.6 end condition (types 1–5 by content, possibly the opener itself; 6–7 at the next blank line) are a RAW extent under the same opener / extent rules as fences (`raw_opener` / `raw_extent`), never inline-parsed, always a run / paragraph / table end; a type-7 opener only where no paragraph is open — the driver's block state, so after a table, a one-line block or a setext heading it IS a block start (R11) — type 1–6 openers interrupt; **case per condition** (R13): conditions 1 and 6 name their tags "(case-insensitive)" (a scoped `(?i:…)`, ASCII-only), 7's names are `[A-Za-z]` classes by grammar, 4 is `<!` + an ASCII letter of either case, 2 `<!--` / 3 `<?` / 5 `<![CDATA[` are exact — a global IGNORECASE read `<![cdata[` as CDATA where commonmark.js reads a paragraph; every raw HTML line holding a `\|` or a declared id is a `[LEX-UNSUPPORTED?]` seed (commonmark.js: `<pre>\n`\n</pre>` is raw, `text\n<span>` is a paragraph, `# h\n<span>` a heading and a raw block, `<div>\nx\n</div>\ny` runs to the blank, `text\n<PRE>` / `text\n<DIV>` a paragraph and a raw block — ⚠ the case probes must interrupt a paragraph, since at a block start `<PRE>` is a type-7 opener too) | `plan_memo_blocks.py::raw_opener` / `raw_extent` / `html_block_type` / `html_block_ends`, `Memo._parse` (seed) | "(lex-seed) an HTML-block opener holding a declared id is a seed", "(html) …" family incl. the four R13 case controls, "(table) a type-7 HTML opener right after a schema table ENDS it…"; spec examples 152 (`<DIV CLASS="foo">`), 182 (`<![CDATA[`) — ⚠ this cell once cited 151 / 190, which are `</div>\n*foo*` and `<table>` (design re-gate 3 MIN-7, verified against `spec.json`) |
| §4.7 | Link reference definition | LEXED — a block of its own at a block start, parsed over the rest of its run; elsewhere an orphan; inside a block quote it registers (Example 218) | `Memo._parse` / `definition_block` (`run_end`) | "(def) …" family, "(quote) `> [sib]: slice-9z-sib.md`…" |
| §4.8 | Paragraph | LEXED — the inline unit Phase 2 scans; inside a block quote, at its real line | `Memo._parse` / `Paragraph` | every prose control |
| §2.1 / §4.9 | Blank line | LEXED — "A line containing no characters, or a line containing only spaces (U+0020) or tabs (U+0009), is called a blank line" (§2.1); ends every block (inside a §4.4 chunk it stays when an indented line follows) | `is_blank` | "(span) an NBSP-only line is NOT blank…" |
| §5.1 | Block quote | LEXED — a CONTAINER (R13): the marker (`quote_content`: ≤3 columns, `>`, one optional space of indentation — a tab gives one column to the marker and the rest to the content, in LINE columns, Example 6) stripped from every marker line, the lines without a marker gathered as lazy continuation candidates, and the SAME Phase 1 run over the content (`Memo._quote` → `_parse`), so a definition / table / raw extent / paragraph / nested quote inside is that block at its real line; laziness through the ONE predicate's `lazy` arm (a candidate is a boundary where no paragraph is open, and with one open by every arm but the setext underline, Examples 92–93; a lazy line that is a GFM table header with a paragraph open is that table's header — cmark-gfm, measured, design re-gate 3 MIN-9); a `>` line interrupts a paragraph (§5.1 itself); no seed (the content is parsed) | `quote_content`, `block_end` / `raw_extent` / `table_header_at` (`lazy`), `Memo._quote` | "(quote) …" family (8 controls, the lazy-header / lazy-delimiter slot tables included), "(span) a paragraph ends at a `>` line", "(block) `[Slice 9z owns it]:\n>`…"; block-sequence control (13 quote shapes, Example 6's raw content asserted); **falsifier = the section's own list, `Block quotes` Examples 228–252: 24 aligned, 235 excluded (a list item), 0 FAIL — 237 `> ```\nfoo\n```` is the driver's stop at a lazy candidate where no paragraph is open** (⚠ `Memo._quote`'s docstring once cited Examples 128 / 174 for that stop; both end their quotes at a BLANK line, the gather's stop); 6 92 93 101 128 174 214 218 from the other sections aligned; "block quotes are linear…" |
| §5.2 / §5.3 | List item / list | LEXED-FLAT (+ SEED) — a list-item line starts a paragraph and its content is inline content; nesting, laziness and the §5.2 interruption rules (empty item, ordered-from-1) are not modelled (local policy, stricter: any marker line interrupts); ASCII digits only. The one place the flat reading HIDES prose is seeded: an indented code block opened while a list item MAY STILL BE OPEN is the item's next paragraph under CommonMark (Example 108 `- foo\n\n    bar`) and a raw extent here — every such line holding a `\|` or a declared id is a `[LEX-UNSUPPORTED?]` seed with the reading `item` (`Memo.raw`). **The item-open bit** (design re-gate 3 IMP-1, the flat reading's whole model of §5.2 nesting): SET when a paragraph headed by a marker line is flushed; CLEARED by a block start (any line classified outside a run, a blank line excepted) below 2 columns of indentation — the smallest content indent a marker gives; the real content indent is not tracked, so the seed over-approximates (`-   item\n\n        x` is code under CommonMark and still seeded; a seed, never an inventory). Verified against commonmark.js 0.31.2: `- item\n\n  para\n\n    x`, `- item\n\n  > q\n\n    x`, `1. item\n\n   para\n\n     x` are each `<p>x</p>` in the item; `- item\n\n para\n\n    x` and `- item\n\n  para\n\n# h\n\n    x` a code block outside it. ⚠ Before re-gate 3 the bit was a ONE-BLOCK memory ("the last block is the item's paragraph"), lost at the item's second paragraph, at a quote inside it and at an ordered item's 3-column paragraph — the prose after was raw and unseeded (rc 0, no seed) | `list_item_line`, `Memo._parse` (`item_open`, `close`) | "(span) a paragraph ends at a list item…", "(ascii) `١.` is not a list marker", "(lex-seed) `- item\n\n    Slice 9z owns it`…", "(lex-seed) `- item\n\n  para\n\n    Slice 9z owns it`: the item's SECOND paragraph keeps it open…", "… `> q` inside the item keeps it open…", "… `1. item` … 3-column paragraph keeps it open…", "… a 1-column paragraph CLOSES the item…", "… a 0-column heading CLOSES the item…", "(lex-seed) `para\n\n    Slice 9z owns it`: its reading is §4.4 raw code, not the item's content"; the 13 excluded spec examples |
| GFM §4.10 | Table | LEXED — see the §3 rows above; the id cell of a schema row is scanned too, with the row's own id suppressed (`**7z** — Slice 9z lands first` names `9z`); a NON-schema row's excess cells are ignored before lexing (GFM §4.10, verbatim: "If there are a number of cells fewer than the number of cells in the header row, empty cells are inserted. If there are greater, the excess is ignored" — ⚠ misquoted as "If greater, the excess is ignored" before design re-gate 3 MIN-8; R13 — an ignored cell's `[x](absent.md)` is no link), a schema row's width miss unchanged; inside a block quote a table is a table; its delimiter row may not be lazy, while a lazy header row IS the header where a paragraph is open (cmark-gfm, measured; MIN-9) | `Memo._parse` (`table_header_at`) / `admit_table` / `split_row`, checker `blocks()` (`Row.line`, the content line) | "(row)" / "(table)" families; "(id) the id cell's trailing prose is scanned…"; "(table) an excess body cell of a non-schema table is ignored…" (×3); "(quote) a slot table inside a block quote is a table…" |

Inline constructs outside the lexed rows (§6.5 autolinks, §2.5 entity references, §6.2 emphasis
beyond the `**` / `` ` `` decoration the id grammar reads) are read as written; the umbrella memo's
`[LEX-UNSUPPORTED?]` count at `07ecf7d8`+ was **5** (one `>` line, four indented-code lines at a block
start — all `sed`/shell examples holding a `|` or a digit-shaped id) and is **4** since R13: the `>`
line (1013) is parsed content, the three shell examples (1047 / 2336 / 2342) are raw code unseeded
like a fence, and the chunk at 2955–2960 — indented prose after list item `10.`'s paragraph, the
Example 108 shape — is seeded on its four id-holding lines under the §5.2 row; reported, not gating.
**10 since design re-gate 3** (IMP-2, one seed rule for raw lines): the same four `item` lines
(2955 / 2958 / 2959 / 2960 — the item-open bit changes nothing on this memo, 2955 follows item `10.`'s
paragraph directly) plus SIX indented-code lines holding a `|`, printed with the §4.4 reading: the shell
examples at 1047–1048 (`sed -n … | grep -v '^    ' | grep -c`), 2336 (`grep -o -E … | wc -l`) and
2342–2344 (a `grep | grep -o | sort | uniq | tr` pipeline) — ⚠ the re-gate brief counted three (1047 /
2336 / 2342): each example's continuation lines hold a `|` as well. Reported, not gating; census 48, 717
sites and 36 ORDER-PROSE? rows unchanged.

**Breadth**: K=2 (CommonMark 0.31.2, GFM 0.29), M=9
**Split decision**: by the edge-dense rule (not K/M): umbrella + **2 slices** (§7). Each slice is a
terminal unit under this umbrella. The memo split `#11-vm-p4-memo-section-seam-split` is a separate
follow-up, not needed here; its ledger note (paraphrase: the checker must first include sibling
tables in the census) is **satisfied by Slice 1 (B×C)** — premise-correct the ledger at landing.

### §3.1 User-input touch audit

No opcode / native / ECS surface. Input = repository Markdown. Data-flow model in lieu of an ECS map:
SoT = memo text; derived, in order = fences → rows → cells → code spans → links → ids → mask
disposition → mentions → census → findings.

## §4 Open defects (Codex R94 on #506, threads left open there as inventory) → slice

| # | Site @ `190d2adb` | Defect | Invariant | Slice | Closed by |
|---|---|---|---|---|---|
| 1 | `plan_memo_tables.py:163-188` | label match lacks **internal-whitespace collapse** (case already lowered both sides — `lower()`, not casefold) | I-B | 1 | grammar |
| 2 | `plan_memo_tables.py:166` | `links()` reads raw text; a link in a code span / fence is taken as real ⇒ exit 2 on a valid memo | I-A×B | 1 | lexing order |
| 3 | `plan_memo_tables.py:87` | body-row width not validated at admission; 8 consumer guards skip silently + 1 degrades (`check.py:333`); long row fabricates a finding | I-C | 1 | single admission site |
| 4 | `plan-memo-umbrella-check.py:180` | licensed-kind prefix licenses a forbidden later clause | I-D | 2 | predicate |
| 5 | `plan-memo-umbrella-check.py:269` | `self_id` exclusion hides a forbidden self-attached role | I-D | 2 | predicate (+ control flip) |
| 6 | `plan_memo_tables.py:364` | attribution recognises only the dash-appositive | I-E | 2 | structural subject rule |

Also Slice 1 (found by plan-review R1, no Codex thread): census is main-memo-only (B×C);
sibling discovery is one level deep (I-B transitive); harness ≠ `main()` on three gating stages
(I-F); `_REF_DEF` / destination / title character classes (§3).

## §5 Design decision — subset lexer (Option A), with its fidelity bound stated

Option B (a Markdown library) would give full CommonMark fidelity, but `.claude/tools/` has no
Python dependency mechanism (stdlib only; no manifest in the repo — `git ls-files | grep -E
'requirements|pyproject|Pipfile'` = 0), the stdlib has no Markdown parser, and the checker needs two
deliberate deviations a parser's AST would have to be post-processed for anyway (id-only code spans
are mentions; link labels are scanned). Introducing a dependency mechanism for one tool is a
separate infra decision — **deferred trigger-only, no slot**: re-evaluate if a second Python tool
in `.claude/tools/` needs a parser, or (a behaviour that CAN fire) if a LEXED row's control cannot be
made to pass without modelling §5.2 list nesting (§5.1 block quotes with lazy continuation and §4.6
HTML blocks ARE modelled by hand, R10–R13, in ~150 lines under the one predicate — the trigger did not
fire for them) or if the `[LEX-UNSUPPORTED?]` seed on the umbrella memo reports a line holding a `|`
inside an HTML block or in indented code after a list item (a table the lexer would read that
CommonMark would not). Therefore
**A**: the constructs in §3 are lexed by construction from the spec clauses listed there, each clause
with a control; constructs outside §3 are read as prose and are listed there as not detected. The
`feedback_helper-prefer-upstream-machine-readable` memory concerns spec *data* sources and is not a
ground for either option; it is not cited.

## §6 Acceptance (per slice)

- **Slice 1**: `--self-test` and `--self-test --mutants` green; a positive control + named mutant
  for: every §3 ✗ row, §4 #1–#3, **each exit-gating stage** (absent sibling → rc 2, missing schema →
  rc 2, KIND-SPELLING → rc 1, sibling-declared umbrella in the census, sibling row in the keep-set,
  sibling `Deps` edge asserted), multi-line code span, quoted kind marker (A×E), `\|` unescape
  (⚠ the link-cycle control has no mutant — removing the visited set hangs; control only);
  `check()` is the only pipeline and `grep -n 'len(cells)'` returns only `is_separator`; the #506
  memo at its head re-run reports census `48 = 33 + 15`, rc 0, and the site count is **re-measured
  and reported with its delta** (706 at `190d2adb` → **702**, −4, re-run 2026-08-22 on `7931798d`;
  ⚠ **703** after the `/simplify` pass made the scanners block-level like the lexer -- the one new
  site is main-memo 2604 `2`, whose row noun `rows` ends line 2603, i.e. a line is a reporting
  coordinate only; **703 again, the same 703 sites** (`--worklist` file/line/id/source set
  identical) after the `/code-review high` pass, re-run 2026-08-23: **140 controls, 67 mutants / 0
  survived / 0 crashed** -- a crashing control is now a FAIL, not a kill -- census `48 = 33 + 15`,
  rc 0, 36 ORDER-PROSE? rows unchanged, memo run 0.39 s → 0.28 s. ⚠ That pass changed one
  `Population.ids` key without moving the census: the §8 `Intl` row (main memo line 2716, id cell
  `` `Intl` → owned externally by … ``) was keyed by its WHOLE cell, because `bare_id` fell back
  to the cell text when no grammar matched; the id cell now reads the one decorated-id grammar at
  the cell's start and that row is keyed `Intl` (a 4-character id, terminal). A non-empty id cell
  that does not start with an id declares nothing and was reported as an `[ID-CELL]` note (⚠ since the
  Stage-6 pass below: a schema miss, rc 2 — an unkeyed row's cells go unasserted, the I-C class); the
  main memo has 0 such rows (`**—**` is an empty cell). `mise run trip-wires` rc 0);
  trip-wire added, registered, green in `mise run trip-wires`; header + docstrings rewritten.
  ⚠ **`/elidex-review` Stage-6 pass (re-run 2026-08-23)**: **162 controls, 80 mutants / 0 survived / 0
  crashed**, census `48 = 33 + 15`, rc 0, **36 ORDER-PROSE? rows unchanged (same rows)**, sites
  **703 → 717** (+14, nothing lost): both sides of the bare-id boundary are now the complement of the
  id-continuation class (`[0-9A-Za-z]`, a `.` inside a dotted number; a decorated side bounded by its
  decoration; a hyphen BOUNDS a short id on both sides — only the `#11-` slug keeps internal hyphens,
  and `slice-9z-sib.md` is safe as a lexer `file` token). Gained: ten ids bounded by an ASCII `"` the
  old punctuation list did not name (main:1944/2007×2/3004/3009/3049, detail:201×2/215/267 — all
  quotations of withdrawn ordering / owner text, reported by default); `` `9d` `` at main:1081, the far
  end of `` `9a`-`9d` `` whose decoration now bounds it; and three ids the old asymmetric rule (rhs `-`
  admitted, lhs `-` not) hid behind a leading hyphen — main:1978 `9c` in `9a-9c`, main:1983 `1b` in the
  second `"1b-5"`, main:3053 `1a` in `0cb-before-1a`. Unresolved references and unkeyed
  schema rows became schema misses (rc 2) — the memo has 0 of either, so rc stays 0. Every seed and
  the licensing rule read the block's one disposed stream (`stream()`), so a `gates` or `MERGED`
  inside a code span is code; measured: no ORDER-PROSE? row moved. `is_empty` is decided by shape
  (no alphanumeric) with `n/a` / `none` as the only lexical exceptions — for `Deps`; the ID cell's
  blanks are LITERAL (`""` / `—` / `-` / `–` after decoration strip, `is_blank_id_cell`), so a `?` / `…`
  id cell is unkeyed → rc 2, not a silent non-row. A citation-grammar label (`[C19]`) is exempt from
  the unresolved-reference miss in every form (shortcut / full `[C19][C20]` / collapsed `[C19][]`).
  Stage 4.5 re-run: **168 controls, 83 mutants / 0 / 0**, 717 sites unchanged. Slice-1 delta recorded under
  I-E above. `mise run trip-wires` rc 0.
  ⚠ **PR #510 Codex R12 (2026-09-07)**: **266 controls, 145 mutants / 0 / 0**, census `48 = 33 + 15`, rc 0,
  36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist` file/line/id/source set
  identical to `29dc1eb7`'s), `[LEX-UNSUPPORTED?]` 5 unchanged (the memo holds no tab). The conformance
  control (§3.0) is the falsifier for the LEXED rows from here on: **169 aligned / 27 excluded / 0 FAIL**.
  `--self-test --mutants` **49 s → 4.2 s**: the Phase-1 linearity witness counted the lexer's
  `link_label` binding (Phase 2's one call per `[l..]`, never `reference_definitions`' calls), so the
  quadratic mutant RG-1 was killed by 48 s of wall clock alone; it now counts `plan_memo_blocks`'
  binding and requires EXACTLY one call per line (a mis-bound counter sees fewer: red), with a runner
  mutant re-binding it. One mutant deleted as EQUIVALENT (the licensing rule's stream read: since the
  scanners read the stream, no fixture reaches it with raw ≠ stream at a mention boundary — reason in
  `plan_memo_selftest_mutants.py`). `.github/workflows/ci.yml` `trip-wires` `timeout-minutes` 2 → 5
  (211 s on Codex's runner had cancelled a green job).
  ⚠ **PR #510 Codex R13 (2026-09-07) — the block-grammar family closed**: `plan_memo_blocks.py` had
  drawn IMP findings three rounds running, each on a §3.0 PROSE-AS-WRITTEN disposition; the root was
  that the 27 conformance exclusions were the unmodelled grammar. §4.4 indented code is now a RAW
  extent (the third `raw_opener` arm, the §4.4 chunk in `raw_extent`, no seed — like a fence) and §5.1
  block quotes are CONTAINERS (`quote_content` + `Memo._quote`: the same `_parse` over the content,
  laziness as `block_end`'s `lazy` arm); `unsupported_block`, `_QUOTE` and `container_text`'s §4.4 arm
  are deleted, `Memo._phase1` is `Memo._parse(lines, linenos, lazy)`, and Phase 1 states its result
  as `Memo.sequence`, which the conformance control now consumes directly (its ownership
  re-derivation from dropped lines is gone). Conformance **169 / 27 / 0 → 184 / 12 / 0**; the 12 are
  §5.2 list items only. Also R13: a NON-schema row's excess cells are cut at admission (GFM §4.10) and
  §4.6 reads case per condition (`<![cdata[` is a paragraph). **285 controls, 163 mutants / 0 / 0**,
  `--self-test --mutants` 4.3 s; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717
  sites, the same 717** (`--worklist` file/line/id/source set identical to `5b2ecdd3`'s tools on the
  same memo — ⚠ the brief expected the four indented lines' sites to leave the 717; they were never
  in it: their ids `5` `F` `2` `4` `8` are the two declared-miss shapes), one LICENSED mention gone
  (main:2959 `10a`, inside the 2955–2960 chunk), `[LEX-UNSUPPORTED?]` 5 → 4 (above). Two R13 probes
  survived their mutants on the first run and were rewritten: `<PRE>` / `<DIV>` at a block start are
  type-7 openers too, so the case fold was never their subject — the probes now interrupt a paragraph;
  and the quote-bound mutant is a cost mutant, killed by the new linearity witness (≤ 4N
  `quote_content` calls), while laziness itself has its own mutant. The harness's exclusion predicate
  was narrowed from "any block-start line in a paragraph" to list-marker lines, so a quote read as
  prose is a FAIL rather than a silent LEXED-FLAT re-classification.
  ⚠ **Design re-gate 3 (2026-09-07, 3 IMP + 7 MIN, applied on top of the `Memo` split, §7)**: IMP-1 the
  §5.2 item-open bit replaces the one-block memory (`Memo._parse` `item_open` / `close`); IMP-2 ONE seed rule
  for raw lines (`Memo.raw` with a reading; an indented schema row after a table is seeded, never a silent
  skip); IMP-3 the false "cmark-gfm's row continuation" attribution rewritten to the measured table ends
  (§2 I-A); MIN-4 the conformance exclusion is "marker line FIRST in the paragraph"; MIN-5 the block-sequence
  control asserts the raw extents' CONTENT (Example 6 exactly); MIN-6 `Block quotes` §5.1 Examples 228–252
  vendored, Example 237 cited for the driver's lazy stop; MIN-7 §4.6 examples 152 / 182; MIN-8 the GFM
  §4.10 and CommonMark §5.1 quotes verbatim; MIN-9 a lazy HEADER row is the table's header (cmark-gfm,
  measured), the delimiter arm kept; MIN-10 `setext_underline` returns the heading kind, the second spelling
  deleted. **297 controls, 170 mutants / 0 / 0**, conformance **208 / 13 / 0 over 221**, census `48 = 33 +
  15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist` file/line/id/source
  set identical to `8d08b333`'s), `[LEX-UNSUPPORTED?]` **4 → 10** (§3.0), `mise run trip-wires` rc 0.
- **Slice 2**: §4 #4–#6 each with positive + mutant controls, I-E's connective set each a control
  plus the `Unlike Slice 7z` negative; the flipped self-reference control documented; R94 threads
  #4/#5/#6 resolved on #506; slot CLOSE −1.

## §7 Slices

- **Slice 0 — touch-time split (standalone prereq commit, stacked first)**: `plan-memo-umbrella-check.py`
  (811) has seams at the licensing-rule regex block (L139–209) / mentions+scan (L210–361) / roles
  (L362–395) / assertions (L396–641) / pipeline `collect_mentions` (L642–687) / `main` (L688–). Slice 1
  touches the assertions (guard deletion) and the pipeline, Slice 2 touches roles and `_anchored`;
  both slices touch both `.py` files. Per `feedback_split-on-touch-prereq-workflow` the split precedes
  the first touching edit: roles + assertions → `plan_memo_roles.py`, behaviour-preserving, self-test
  green. ⚠ The licensing regex block (L139–209) moved with them — the assertions import it and
  leaving it behind is an import cycle; ⚠ the fixture builder lives with the cases, not the runner
  (a top-level `from selftest import build` in the cases module is a cycle); the runner imports it. The self-test (396) receives the most new text in Slice 1 (≥20 controls + the MUTANTS
  table), so it is split in Slice 0 too: `plan_memo_umbrella_selftest.py` (runner + fixture builder)
  / `plan_memo_selftest_cases.py` (the one `CASES` list; ⚠ the plan's "ASSERT_CASES" never existed —
  assertion controls are `Case` records with a `("finding", CODE)` measure in the same list) /
  `plan_memo_selftest_mutants.py` (MUTANTS, Slice 1 creates it). ⚠ Touch-time split during the #510
  converge (the cases module reached 954 lines at `1840251b`): the registry is split at the
  review-round seam — `plan_memo_selftest_cases.py` keeps the builder, the record shape and every
  pre-converge control; `plan_memo_selftest_cases_pr510.py` holds the PR #510 review-round controls
  (Codex R1–R11, the design re-gate) and appends to the same `CASES`; the runner is the one import
  site of both. ⚠ Touch-time split before design re-gate 3 (`plan_memo_tables.py` had reached 879
  lines): `Memo` (the Phase-1 driver `_parse` / `_quote`, Phase-2 resolution, file I/O, the sibling
  resolver) AND `Population` move to `plan_memo_memo.py` — `Population` cannot stay behind, since
  `Population → Memo → admit_table` would then be an import cycle (`admit_table` stays with the
  schemas in `plan_memo_tables.py`); seam = row grammar / schemas / disposition vs the document driver
  and the memo set. One import site per consumer (`plan-memo-umbrella-check.py` imports `Population`
  from `plan_memo_memo`, the row grammar from `plan_memo_tables`); the runner's loader gains the module
  in dependency order and the MUTANTS rows whose substring moved (45 of the 59 `TABLES` rows) target
  `MEMO`; two re-injections (`is_empty`, `is_blank`) are qualified with `__import__` because the name
  is no longer imported where the row patches (a crash is a FAIL, not a kill). Behaviour-preserving:
  285 controls / 163 mutants 0 / 0, census and the 717-site worklist identical to `8d08b333`'s.
- **Slice 1 — lexical substrate + one pipeline + one population** (I-A/B/C/F; §3 all rows; §4
  #1–#3; interim connection; header/docstring rewrite). Touch set: `plan_memo_tables.py` (lexer,
  `split_row`, `find_tables`, `links`, `code_spans`, `Memo`), `plan-memo-umbrella-check.py` (`check()`,
  keep-set, guard deletion), `plan_memo_roles.py` (guard deletion only), selftest, the trip-wire,
  `scripts/trip-wires.sh`, `ci.yml` comment, `mise.toml` comment, CLAUDE.md sentence. Plan-review on a
  per-PR plan before implementation; lands first.
- **Slice 2 — prose predicates** (I-D/E; §4 #4–#6). Touch set: `plan_memo_roles.py` (licensing,
  connective grammar), `plan-memo-umbrella-check.py::_anchored` (self_id), `plan_memo_tables.py::
  _attributed_to_other` (moves to roles), selftest. Plan-review; lands second.

## §8 Defer (owned here, not only in file headers)

- Acceptance half of assertion (b) — "not implementable here"; slot
  `#11-plan-memo-acceptance-falsifiability-check` is minted in #506's memo (§5 mention `190d2adb:…:1218`, §8 row `:2711`)
  and is **not yet in the slot SoT ledger** (`memory/project_open-defer-slots.md` = 0 hits); its
  ledger registration is owed at #506's landing, not here. No date — trigger = #506 landing.
- The four assertions' owner, `#11-plan-memo-spec-field-single-home-check` (cited by the tool headers
  as "the single-home slot #506's memo §8 mints") — the same pre-agreed commitment: minted in #506's
  memo §8 (`190d2adb:…:2710`), **not in the slot SoT ledger** (0 hits), registration owed at #506's
  landing, not here; the headers cite that origin rather than presenting the slot as registered.
- Two KNOWN-MISS bare-id shapes (numeric / single letter) — declared in the self-test; trigger = a
  memo minting such an id; no slot (seed boundary, not a platform gap); no date — trigger-only.
- GFM row splitter duplicated four ways across three branch families — trigger = two of
  them on `main`; Slice 1's `split_row` is the candidate canonical copy; no slot; no date — trigger-only.
- Markdown library dependency (§5) — trigger-only (see §5); no slot; no date.
