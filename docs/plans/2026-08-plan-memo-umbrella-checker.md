# Umbrella plan — `plan-memo-umbrella-check` carved out of #506 into a 2-slice prerequisite program

**Status**: plan-review **converged** 2026-08-22 (IMP 16 → 10 → 3 across three rounds; R3's three were mechanism decisions, applied below; remaining MINs applied). Implementation order: Slice 0 → Slice 1 (this PR) → Slice 2. **Implementation record (Slice 0 `718626e9`, Slice 1 `7931798d`)**: premises of this plan the implementation found false are marked ⚠ inline below; measurements in §6 are the re-run values. Branch `vm-p4-plan-memo-checker` (worktree
`elidex-wt-vmp4checker`, base `origin/main`). Files carried verbatim from #506 @ `190d2adb` **at the
carry commit `5e9439b4`** (`git diff --quiet 5e9439b4 190d2adb -- .claude/tools/` = identical there, not
at HEAD): `.claude/tools/plan-memo-umbrella-check.py` 811 lines, `plan_memo_tables.py` 407,
`plan_memo_umbrella_selftest.py` 396 (`wc -l`, 1,614 total). At HEAD of this PR the program is 20
`.py` files: `plan-memo-umbrella-check.py` 583 / `plan_memo_tables.py` 814 / `plan_memo_umbrella_selftest.py`
97 (the three carried names, 1,494) + `plan_memo_ids.py` 224 / `plan_memo_emphasis.py` 216 / `plan_memo_lexer.py` 887 /
`plan_memo_blocks.py` 746 / `plan_memo_memo.py` 917 / `plan_memo_population.py` 239 / `plan_memo_roles.py` 463 /
`plan_memo_selftest_cases.py` 614 /
`plan_memo_selftest_cases_pr510.py` 701 / `plan_memo_selftest_cases_inline.py` 922 /
`plan_memo_selftest_conformance.py` 377 / `plan_memo_selftest_controls.py` 939 / `plan_memo_selftest_harness.py` 268 /
`plan_memo_selftest_work.py` 350 /
`plan_memo_selftest_mutants.py` 494 / `plan_memo_selftest_mutants_pr510.py` 709 / `plan_memo_selftest_mutants_inline.py` 867
— **11,427 total, measured on the tree of the R24 collapse round (`d420b632`); re-run at landing** (`wc -l
.claude/tools/plan*.py`, re-run before each push; a figure here is stale the moment a file is touched).  Every file
is under the 1000-line bound, the largest being `plan_memo_selftest_controls.py` at 939 — ⚠ 61 lines of headroom, and
`plan_memo_selftest_cases_inline.py` 78: the next round that adds controls splits one of them first.  No `crates/` change.
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

- **I-A Lexical masking, and the RENDERED text** (design re-gate 4) — the ONE text every predicate
  over a block reads is the block **as the document renders it** (`plan_memo_tables.stream`): a
  construct that contributes NO character (a §6.2 / GFM delimiter pair, a raw HTML tag or comment, a
  link's `[` and its tail) is dropped, so the text on both sides of it is one run; a §2.4 escape and a
  §2.5 character reference substitute their character; and a construct that renders text this checker
  refuses to read (a code span, an autolink, an image's tail, a citation id, a file name) is BLANKED
  in place — still unreadable, but still a boundary, because a reader does not read across it either.
  The disposition table is `RENDERS_TEXT`, mirrored in §3.0b's `Renders` column, and the stream
  carries the map back to source offsets (`Stream.at`) so every reported line and column is the raw
  one. Two EXCEPTIONS, each the same rule twice: a code span whose content is only declared ids, and
  a `**` pair whose content is only declared ids, are the document SPELLING or DECORATING an id and
  stand in the stream, delimiters and all (`id_only`, one predicate, both constructs — so
  `` `9a`-`9d` `` is two ids and `**9z**7z` is `9z` then `7z`); and a substitution that would spell a
  decoration character stands as written, since the stream carries exactly one kind of markup and
  `\*\*C\*\*` is not a bold `C`. **The residue is reported, never decided**: where a unit the
  reader reads STRADDLES a blanked span the two readings disagree, and `split_units` says so — as the
  `[LEX-SPLIT?]` seed, and, for the kind marker in a row's DECLARING FIELD, as a schema miss (rc 2),
  because §1 forbids a clean exit for a could-not-scan over the census this program exists to take.
  ⚠ Until that re-gate the stream was the source text with the masks blanked: `Slice 9**z**`, which
  renders `Slice 9z`, was read as the row `9` (a fabricated site on one row, a lost site on another),
  and `**UMBRELLA, not a *terminal* unit.**` — or `&#44;` for the comma, or `~~terminal~~`, or
  `<b>terminal</b>`, or an escaped `\,` — declared nothing at all, so the row left the umbrella
  census silently at rc 0. Text inside fenced blocks and inline code spans is never read as a
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
  ATX heading (§4.2), thematic break (§4.1), list-item line (§5.2: the container opens — under the
  spec's INTERRUPTION RULE where a paragraph is open, the same `para_open` bit: only a non-empty item,
  ordered only from 1, interrupts, so `foo\n2. b` and `foo\n-` are one paragraph; on a container's
  lazy candidate any marker line is a boundary, `- a\n2. b` / `> a\n2. b` a container then a list —
  R15; ⚠ until R15 a stricter local policy let any marker line interrupt), `>` line (§5.1: the
  container opens; R13) — a setext underline **where a paragraph is open** (§4.3; the paragraph becomes the
  heading; the same `para_open` bit as type 7: with no paragraph open there is nothing to underline,
  so after a table's rows `===` is a one-cell body row (GFM §4.10 Example 202 — the width miss fires)
  and `---` the thematic break §4.1 already names; R12: the unconditional arm let a malformed memo
  exit 0; the shape is read before a list marker is, commonmark.js's block-start order, so `foo\n-`
  is a heading and not an empty item; Example 94 `- Foo\n---` is the item's lazy candidate, a thematic
  break by the lazy arm — ⚠ the `container_text` paragraph-head rule that once stood for it is gone
  with the flat reading, R15) — or a GFM table header
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
  (Example 100). **R15 — the §5.2 root, the last container**: a list item is a CONTAINER exactly like a
  block quote (`Memo._list` / `_item`): `item_marker` reads the marker (after ≤3 columns; a bullet or 1–9
  ASCII digits + `.`/`)`, followed by a space, a tab or the line's end; a thematic break takes precedence,
  §4.1 Example 61) and the item's CONTENT INDENTATION in line columns — W + N, N the spaces after the
  marker as written when 1–4, 1 when five or more (the item starts with indented code, Example 270) or
  none (a blank first line, Example 278; "at most one blank line", Example 280) — `strip_columns` undoes
  it on every later line (§2.2: a tab past it leaves its remaining columns; `-\t\tfoo` is code holding
  `  foo`, Example 7), and the lines short of it are the item's LAZY CONTINUATION CANDIDATES through the
  SAME gather and the same `block_end` lazy arm as the quote's (§5.2 rule 5 restates §5.1: one
  mechanism; an enclosing container's candidate stays lazy and whole inside the item — `> - a\n    ---`
  is the item paragraph's text, measured). Sibling items of one type (§5.3: the same bullet character, or
  the same ordered delimiter — `- a\n2. b`, `1. a\n1) b` are two lists, Examples 301–302) form a list whose
  entry states its first marker and its TIGHTNESS (§5.3 verbatim: "A list is loose if any of its
  constituent list items are separated by blank lines, or if any of its constituent list items directly
  contain two block-level elements with a blank line between them" — `_parse` returns whether it ended on
  a blank line and whether a block opened across one, recursing into a nested list's last item as
  commonmark.js's `endsWithBlankLine` does; a quote's gaps are its own; the blank first line of an item
  opens no gap; every shape measured). ⚠ Until R15 the item was LEXED-FLAT — the marker line headed a
  paragraph and nothing tracked the content indentation — so the item's next paragraph at that
  indentation (Example 108) was consumed as INDENTED CODE — a list item, a blank line, then an indented
  link to `child.md` — so it never found
  its link and never walked `child.md`, rc 0 (the reviewer's input); an item-open bit only seeded it. ⚠ That
  shape is described in words rather than spelled, because spelled inside this long paragraph its backticks
  paired elsewhere and the example became a LIVE link to a memo that does not exist: it was this document's
  fifth FATAL, and the checker could not run cleanly on its own plan. Code spans are lexed over the
  PARAGRAPH, not the line. The
  bit, the `item` seed reading and `container_text` are deleted. A block quote is a CONTAINER (`Memo._quote`):
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
  to `admit_table` instead of ending the quote, the one boundary a lazy line can be with a RUN
  open, since it is the only `block_end` arm that needs the next line — and a reference DEFINITION keeps
  the run open exactly as paragraph text does (R17: `> [a]: /u\n| h |\n> |---|` is quote[def, table], `- [a]:
  /u\n| h |\n  |---|` item[def, table]; cmark-gfm parses §4.7 out of a paragraph only at its end, so the
  header is read out of the definition's paragraph — the ONE rule in `Memo._parse` is `cur or i == def_end`,
  MEASURED on fourteen variants listed in its docstring: after a blank quote line, a fence, a heading, indented
  code, an HTML block, a thematic break or a table the quote ENDS before the header; ⚠ until R17 the consumed
  definition left `cur` empty and the quote ended, so a linked memo's schema table there was dropped, its
  ids never declared, rc 0 — schema presence is required of the main memo only. ⚠ One divergence stays,
  stated: cmark-gfm also PRINTS that definition as a paragraph and registers nothing — `[a]: /u\n| h
  |\n|---|\n\n[a]` renders `[a]` literally, at the top level and in the quote alike; its table extension
  re-creates the residual paragraph without the §4.7 parse — which is not followed: the definition registers,
  the polarity that loses no memo; the rule models WHERE the header lands). Each expectation above was
  checked against commonmark.js 0.31.2,
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
  line classified once with the open block in hand, re-entered once per container — a block quote, a
  list item — over its content. The scanners read the block's ONE disposed
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
| CommonMark §6.4 Images | image | `![` opens an image (an unescaped `!` before `[`); not a link; destination never a sibling; alt text prose; tail masked as kind `image`. **R19**: a RESOLVED image's description is plain text (§6.4: the description is rendered as the `alt` attribute's plain string content), so in ONE rule at the point the image closes (`inline_pass`, offset containment: the entries recorded past the image's `[` — the trailing ones, since brackets nest and each list is appended in closing order) every bracket construct inside the description is the description's: a link is DEMOTED to a masked tail in `Lexed.images` (never a memo link — its destination never joins the population — and never prose: the alt text is the link's TEXT), a nested image stays masked, a failed reference names no lost memo (resolved, it would have been demoted). An UNRESOLVED image is literal `![…]` text and the link inside it IS a link. commonmark.js 0.31.2, measured: `![alt [docs](absent.md)](image.png)` → `<img alt="alt docs">`; `![alt [docs](child.md)][missing]` → `![alt <a href="child.md">docs</a>][missing]`; `![a ![b [c](x.md)](i.png)](j.png)` → `alt="a b c"`; `![a ![b [c](x.md)](i.png)][missing]` → `![a <img alt="b c">][missing]`; `[a ![b [c](x.md)](i.png)](y.md)` → `[a <img alt="b c">](y.md)` (the inner link deactivated the outer `[` before the image demoted it — the §6.3 rule of the row above, no second rule). ⚠ Until R19 the inner link joined the population AS IT CLOSED, so the reviewer's `absent.md` inside a resolved image's description was a false unavailable-memo miss, rc 2 | `plan_memo_lexer.py::_is_image` / `inline_pass` (the `is_img` arm) / `Lexed.images`; `plan_memo_tables.py::dispose` (`image`) | ✓ controls "(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)`…", "(image) `![alt][img]` with a definition is consumed whole…", the R19 "(image) …" family (the reviewer's input rc 0; `child.md` not walked and the image's `.md` destination not probed; the unresolved image KEEPS its link, `child.md` walked; a nested image; the outer failing while the inner resolves; a link outside the inner description; `[a ![b [c](child.md)](i.png)](parent.md)` links nothing; a failed reference inside the description rc 0; the demoted tail masked — `9z` in its destination is no site); mutants R19 #1 (the immediate add re-injected; a plain drop instead of the masked demotion; the failed-reference rule dropped; demotion re-injected on the FAILED image too) | no |
| CommonMark §4.7 Link reference definitions | next-line title | the title may follow on the line after the destination; an invalid next line leaves the definition ending at the destination (one attempt, same-line and next-line) | `plan_memo_blocks.py::reference_definitions` | ✓ controls "(def) a next-line title is part of the definition…", "(def) a next-line title holding `[x](missing.md)`…", "(def) a next line that is NOT a valid title is prose…" | no |
| CommonMark §4.7 | orphan detection | exactly the spec-grounded class: a line that parses as a VALID definition but cannot take effect because it is not at a block start ("a link reference definition cannot interrupt a paragraph"); a label-and-colon line that is NOT a valid definition is prose (commonmark.js: `[C1]: ECMA-262 §1 says so` is a paragraph; `[sib]: child.md "title\n\nmore"` is a paragraph) and a shortcut naming it is exempt; linear — the runs are joined once, one parse per line | `plan_memo_memo.py::Memo._parse` (`Memo.orphans`; ⚠ pre-split name `plan_memo_tables.py::Memo._blocks`), `plan_memo_blocks.py::definition_block` | ✓ controls "(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan…", "Phase-1 orphan detection is linear: <= 4 link_label calls per line" | no |
| CommonMark §2.4 Backslash escapes | row split parity | only an ODD backslash run escapes a `|` (`a\\|b` is two cells); the trailing-pipe check reads the same parity (`_escaped`, one helper) | `plan_memo_blocks.py::split_row` | ✓ controls "(row) `a\\|b` holds an UNESCAPED pipe…", "(row) `a\|b` is one cell…", "(row) a trailing `\\|`…" | no |
| §5 (local policy over the disposition exception) | id-only code spans | an id-only run is tokenised by the declared-id GRAMMAR longest-first (a `#11-` slug is atomic; `` `#11-zz-alpha / 9z` `` spells two ids), with separators between tokens | `plan_memo_tables.py::id_only` (`_ID_RUN_TOKEN`) | ✓ control "(span) a `#11-` slug is ATOMIC in an id-only run…" | no |
| (no spec clause — a tokenisation fact of these documents) | bare `.md` file name | read by PATH SYNTAX: the maximal run — **possibly empty (R20)** — of non-whitespace characters ending in the lexer's `FILE_SUFFIX` (inline delimiters `[` `]` `<` `>` `` ` `` `\|` excluded so link text and code spans are not swallowed; parentheses only as a balanced pair), bounded by spaces / tabs / line ends or the cell edge; no trailing-punctuation rule is needed because the token ENDS at the suffix (the GFM §6.9 autolink rule is moot). ONE notion of "is a file name", defined by the lexer (`FILE_SUFFIX`) and consumed by `sibling_path` stage (d): the stem is unconstrained on both sides, so `.md` alone is a file name in prose and a sibling in a destination; ⚠ until R20 the token arm required a one-character stem while `sibling_path` did not, and beside a declared id `md` the prose `Read .md for details` reported `md` | `plan_memo_lexer.py::_TOKEN` / `FILE_SUFFIX`, `plan_memo_memo.py::Memo.sibling_path` (d) | ✓ controls "(file) `9z+notes.md`…", "(file) `9z@notes.md`…", "(file) `(9z).md`…", "(file) a link's visible text is not swallowed…", "(file) `.md` alone is a file name…", "(file) `notes.md` beside a declared no-owner id `md`…", "(file) bare `md` (no suffix)… IS a site", "(link) `[x](.md)` links the sibling file named `.md`…" | no |
| CommonMark §2.1 / §4.9 / GFM §4.10 | ASCII classes at every boundary | a blank line = spaces or tabs only (§4.9); edge pipes and cell trimming use the same space/tab class; the far side of a `.` after a bare id is the ASCII id class (`9z.次の工程` reports `9z`); `is_empty`'s `isalnum` is deliberately Unicode (a letter in any script fills a cell) | `plan_memo_blocks.py::is_blank` / `split_row`, `plan_memo_ids.py::_glued` (⚠ pre-R14 name `plan-memo-umbrella-check.py::_glued`), `plan_memo_tables.py::is_empty` | ✓ controls "(span) an NBSP-only line is NOT blank…", "(table) a row opening with an NBSP…", "(bare) `9z.次の工程`…", "(bare) `9z.é`…" | no |
| §5 (local policy: the id grammar of these documents) — **R14** | ONE id-token grammar for every reader | the three kinds (short / `#11-` slug / `[C19]` citation, either case), the decoration, and the ONE two-sided boundary — a decorated side is bounded by its decoration; an undecorated side by the complement of the kind's continuation class (short: ASCII alnum + the dotted-number rule, a hyphen BOUNDS; slug: alnum + `_` + `-` on BOTH sides; citation: its brackets) — are spelled once in `plan_memo_ids.py` and CONSUMED by the bare and anchored naming scans, the raw-line seed, the id cell, the kept-slug exception inside a code span, the lexer's citation mask and the reference walk's citation exemption (R8/R9/R14: the boundary had been spelled six ways, disagreeing on a hyphen on a raw line, a slug's right side, and a citation's case); the orphan-definition exemption is by the orphan's exact bracket `(line, column)`, never by line | `plan_memo_ids.py::tokens` / `is_cite_label` / `kind_of`; readers `plan-memo-umbrella-check.py::_bare` / `_anchored` / `lex_unsupported_seed`, `plan_memo_tables.py::bare_id` / `code_mask`, `plan_memo_lexer.py::_TOKEN`, `plan_memo_memo.py::Memo.unresolved_references` (`Memo.orphans`) | ✓ controls "(lex-seed) a raw HTML line `9z-owner`…" / "…`owner-9z`…", "(slug) `#11-zz-alphaZZ`…", "(slug) `` `tool #11-zz-alpha_extra` ``…", "(cite) `[c1]` beside a declared no-owner id `c1`…", "(cite) adjacent lowercase citations `[c1][c2]`…", "an orphan definition exempts its OWN bracket only…", and the PROPERTY control "the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)" — a sweep over the other modules' string constants for the grammar's spellings, which by construction cannot see a class spelled in another order (`[A-Za-z0-9-]` is the HTML tag-name grammar, `[a-zA-Z0-9+.-]` the URL scheme grammar; neither is an id class), a class built by concatenation, a hand-written character test, `\b` under `re.ASCII`, or a comment — **nor KIND coverage (R20)**: a composer built on `SHORT_ID` alone spells nothing twice; `ROW_KINDS` / `ROW_ID` (slug \| short — a citation keys a citation-table row but declares no kind and is no row in this sense) is the grammar's ONE "a row id here" alternation, composed by `ROW_NOUN_ID` → `_APPOSITIVE`, `OWNS_TWO` and the anchored reading (`_anchored`, `t.kind in ROW_KINDS`), and the PROPERTY control "every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)" probes each composer with a sample of every kind in the grammar's tuple (a kind without a sample is red); ⚠ until R20 `_APPOSITIVE` composed `decorated_id(SHORT_ID)`, so `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributed nothing: the pointer row was an umbrella, no `UMBRELLA-MARK` finding, exit 0 | no |
| CommonMark §6.3 | one label grammar | the text of a collapsed / shortcut reference is a label iff `link_label` reads it from the opener (no second walker); the full form carries `raw` out of `_reference_tail` | `plan_memo_lexer.py::_reference_tail` / `link_label` | ✓ control "(link) bracket text holding unescaped brackets is not a label (§6.3)…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 2 | one inline pass | code spans (§6.1) and brackets are recognised together, left to right; a backtick string opens a span as met and the scan jumps past it; an inline-link tail is parsed by lookahead on the RAW text (a backtick inside a destination is consumed by the link; one before the `]` opens a span that swallows it); no code pre-mask | `plan_memo_lexer.py::inline_pass` (`code_spans` / `links` are views over it) | ✓ controls "(span) a backtick inside a link DESTINATION is consumed by the link…", "(span) a backtick BEFORE the `]` opens a code span that swallows it…", "(link) a link inside a code span is not a link (A x B)" | no |
| CommonMark §6.4 Images | unresolved reference image | `![alt][missing]` is literal image syntax, never an unresolved memo reference (the opener's image flag travels with the unresolved record); the failed reference is recorded ONCE — its label bracket is re-scanned (the scan resumes after the literal `]`; §6.3 Example 571: `[missing][baz]` may be a link, so the tail is NOT consumed) but a shortcut it closes as is the same site, not a second record, so an orphan `[missing]: image.md` later in the paragraph does not turn `[missing]` into a memo miss (R11) | `inline_pass` (`relabel`) → `Memo.unresolved_references` | ✓ controls "(image) an undefined reference image `![diagram][missing-image]`…", "(image) the reviewer's input `![alt][missing]` + an orphan…", "(image) `![alt][missing][Slice 9z]` with `[Slice 9z]` defined…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1 → Phase 2 | block structure before inline structure | Phase 1 over RAW lines (`Memo`): fences, reference definitions as blocks of their own recognised only at a block start and parsed over the REST OF THEIR RUN (the lines up to the next `block_end`, joined once — §6.3 label up to 999 chars and may span lines, `link_title`; a valid definition inside a paragraph is an orphan, `Memo.orphans`), GFM tables ending at the same `block_end` (a reference definition right after a table is a ROW of it — GFM Example 202 — not a definition), paragraphs; Phase 2 (`inline_pass`) over each paragraph's / cell's content only, with `defs` from Phase 1 | `plan_memo_memo.py::Memo._parse` (its `definition_at`); `plan_memo_blocks.py::definition_block` / `run_end` | ✓ controls "(def) a definition is read from RAW lines at a block start…", "(table) a reference definition right after a schema table is a ROW of it (GFM Example 202…) — one cell under a 4-cell header, width miss rc 2" (⚠ until R20 this cell cited the control as proving the definition "ENDS the table", the opposite of the mechanism column and of the control's own expectation), "(def) a definition cannot interrupt a paragraph…", "Phase-1 orphan detection is linear…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1; §4.4; §4.6; §5.1; GFM §4.10 | block start = block state | "is this line at a block start" is the driver's state (a run open or not), the ONE context bit `block_end` / `raw_opener` take; a type-7 HTML opener or an indented line after a table's rows, a one-line block or a setext heading opens a raw extent; after a run line (paragraph text, a definition, a list-item line, a quote's lazy line) it is text — no caller looks back at the previous raw line (R11); inside a block quote the same state, re-entered (R13) | `plan_memo_blocks.py::raw_opener` / `block_end` / `run_end`, `plan_memo_memo.py::Memo._parse` / `_quote`, `plan_memo_tables.py::admit_table` | ✓ controls "(table) a type-7 HTML opener right after a schema table ENDS it…", "(html) `# h\n<span>\n9z owns it`…", "(html) `Heading\n===\n<span>\n9z owns it`…", "(html) `[sib]: slice-9z-sib.md\n<span>\n9z owns it`…", "(html) `- item\n<span>\n9z owns it`…" | no |
| WHATWG URL §4.4 "URL parsing" — the basic URL parser's *scheme start state* (`#scheme-start-state`) and *scheme state* (`#scheme-state`), then §1.3 "Percent-encoded bytes" *percent-decode* on a string (`#string-percent-decode`) — + **local policy** (CommonMark §6.3 / GFM say nothing about siblings on disk) | ONE destination → sibling resolver | `Memo.sibling_path`, stages in spec order: (a) scheme on the RAW path — `#scheme-start-state` step 1 ("If c is an ASCII alpha, append c, lowercased, to buffer, and set state to scheme state") and `#scheme-state` step 1 ("If c is an ASCII alphanumeric, U+002B (+), U+002D (-), or U+002E (.), append c, lowercased, to buffer") / step 2 ("Otherwise, if c is U+003A (:)" → the scheme) read the input's code points AS WRITTEN; `%` is in neither class, so at `%` the parser leaves for the *no scheme state* and `notes%3Achild.md` has no scheme — it is the file `notes:child.md`; percent-decoding is no step of the parser (the *path state*, `#path-state`, percent-ENCODES and keeps `%xx`), (b) percent-decode — `#string-percent-decode` ("Let bytes be the UTF-8 encoding of input. Return the percent-decoding of bytes"; `urllib.parse.unquote`), a consumer's operation on the parsed path (`slice%20sib.md` = `slice sib.md`) — ⚠ until R17 this row and the docstring cited "WHATWG URL" bare, (c) the DECODED name must be RELATIVE on every platform, and hold no C0 control / DEL (`child%00.md` would make `resolve()` raise) — **R19**: ONE platform-independent reading, Windows path syntax (`pathlib.PureWindowsPath`, the superset: `/` and `\` both separate, a drive letter / UNC prefix / root ANCHORS), which is the URL standard's own reading of a special-scheme path (`file` is a special scheme): `#path-state` step 1 ends a segment at `/` or, "url is special and c is U+005C (\)", at a backslash (invalid-reverse-solidus validation error), and step 1.4.1's drive-letter rule is, in the spec's words, "a (platform-independent) Windows drive letter quirk" — so `PureWindowsPath(name).anchor` must be empty: `/x`, `//host/x` (a site URL joined to the memo's directory would probe the host's filesystem root), `\x`, `C:\temp\x`, `\\server\share\x` and the drive-relative `C:x` (raw `C:x.md` is already scheme `c` at (a); percent-encoded `C%3Ax.md` decodes to a drive anchor here, as does the one-letter `n%3Ax.md`, where the multi-letter `notes%3Ax.md` of (a) is a file name) are rejected; ⚠ until R19 (c) rejected a leading `/` only, so `C%3A%5Ctemp%5Cchild.md` and `%5Cchild.md` passed and on Windows `parent / name` discarded the memo's directory, (d) `.md`, (e) the name's PARTS joined beside the memo under the same syntax — `\` is a separator everywhere, never a POSIX name character: `sub%5Cchild.md` is the sibling `sub/child.md` on every platform (R19) — then `_resolve` = `resolve()` with `OSError` or (Python 3.9–3.12 symlink loop) `RuntimeError` read as an UNAVAILABLE sibling, reported by the population's one I/O chokepoint (`Memo()` under `OSError | UnicodeDecodeError` — I/O ONLY, `read_text(encoding="utf-8")`; ⚠ until R16 `RuntimeError` too, which read a parser `RecursionError` — a `RuntimeError` — as an unavailable memo, rc 2, no census; a parser exception is a crash, crash = FAIL) as the exit-2 unavailable-memo miss; `linked_files` dedups with a set | `plan_memo_memo.py::Memo.sibling_path` / `_resolve` / `linked_files`, `Population.__init__` | ✓ controls "(link) `notes%3Achild.md` has no scheme…", "(link) a percent-encoded destination…", "(rc) a percent-encoded ABSOLUTE destination…", "a decoded destination with a C0 control character is rejected…", "an OSError from resolve() is the unavailable-sibling schema miss…" (OSError and RuntimeError injected), "an undecodable sibling is the unavailable-linked-memo schema miss…", "linked_files is linear: <= N Path.__eq__ calls over N distinct siblings…", "a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss" (R16; mutant re-injects the broad except), the R19 "(rc) …" family (`C%3A%5Ctemp%5Cchild.md`, `%5Cchild.md`, the percent-encoded UNC, the raw `\\server\share\x.md` — §2.4 decodes it to the `\`-rooted `\server\share\x.md`, commonmark.js's href `%5Cserver%5Cshare%5Cx.md` — drive-relative `C:child.md` raw and encoded, the one-letter `n%3Achild.md`) and "(link) `sub%5Cchild.md`…" (walks `sub/child.md`); mutants R19 #3 (the `/`-only test re-injected; the POSIX reading `parent / name` re-injected) | no |
| CommonMark §2.5 Entity and numeric character references + §6.3 (destination) — **R16** | ONE destination normalisation | `normalize_destination`: ONE left-to-right pass over the destination's raw text at the ONE site both forms and both grammars return through (`link_destination`, bare and `<…>`; the inline link and the §4.7 definition both call it) — a §2.4 escape yields its character; a §2.5 reference — `&` + an HTML5 entity name + `;` (`html.entities.html5`, looked up WITH the `;`, so HTML's legacy semicolon-less `&copy` is literal, Example 29, and `&MadeUpEntity;` is literal, Example 30), `&#` + 1–7 digits + `;`, `&#x` / `&#X` + 1–6 hex digits + `;` — yields its character, with U+0000, code points above U+10FFFF and surrogates → U+FFFD; the two grammars meet at a character exactly once (`\&#46;` is a literal `&#46;`; a decoded `&`, `&#x26;#46;`, is never re-read as a reference). NOT decoded: a label (§6.3 matching is on the raw label — `[foo&auml;]` ≠ `[fooä]`, commonmark.js measured), a title (`link_title` reads shape only, never text), a code span (`inline_pass` jumps past it). ⚠ Until R16 backslash-only: `[child](child&#46;md)` / `[sib]: child&#46;md` reached `sibling_path` as the literal, no `.md` suffix, the sibling silently outside the population, rc 0 | `plan_memo_lexer.py::normalize_destination` / `_CHAR_REF` / `_reference` / `_codepoint`, `link_destination` (both returns) | ✓ controls "(link) `[child](child&#46;md)`…", "(link) `[child](<child&#46;md>)`…", "(link) `[child](child&period;md)`…", "(link) `[child](child&#x2E;md)`…", "(def) `[sib]: child&#46;md`…", "(link) `[x](slice&#37;20sib.md)`…" (§2.5 then `sibling_path` stage b, spec order), "(link) `[x](child&#0;.md)`…", "(link) `[x](child&#x110000;.md)`…", "(link) `[x](child&copy.md)`…", "(link) `[x](child&#46md)`…", "(link) `[x](child\&#46;md)`…", "(link) `[x](child&#x26;#46;md)`…", "(link) `[x](child&MadeUpEntity;md)`…", "(span) `` `[c](child&#46;md)` ``…", "(label) `[foo&auml;]: child.md` then `[fooä]`…"; mutants R16 #2 (backslash-only re-injected; `html.unescape` re-injected; the U+0000 rule dropped; `;` optional; decoding re-injected in `normalize_label`) | no |
| CommonMark §6.6 Raw HTML — **R17** | ONE tag grammar, a span of the one inline pass | `_HTML_TAG` in `inline_pass`, tried at every `<` the scan meets (left to right with backtick strings and brackets, commonmark.js's order: `<a href="`">b` c` is a tag then a literal backtick, `` `x <span title="`">b `` a code span then text): an **open tag** (`<` + a tag name — an ASCII letter then ASCII letters / digits / `-` — + attributes + optional spaces / tabs / one line ending + optional `/` + `>`; an **attribute** = at least one space / tab / line ending (≤1 line ending), an attribute name — ASCII letter / `_` / `:` then letters / digits / `_` / `.` / `:` / `-` — and an optional value spec `=` with optional whitespace around it and a value: **unquoted** = a nonempty string without spaces, tabs, line endings, `"`, `'`, `=`, `<`, `>`, `` ` `` — so `<span title=[x](y.md)>` IS a tag (⚠ the R17 brief presumed a negative), **single-quoted**, **double-quoted**), a **closing tag** (`</` + name + optional whitespace + `>`), an **HTML comment** (0.31's `<!-->`, `<!--->`, or `<!--` + a string not containing `-->` + `-->`: `<!-- a -- b -->` is one, 0.30 forbade it), a **processing instruction** (`<?` … `?>`), a **declaration** (`<!` + an ASCII letter + no `>` + `>`, either case) or a **CDATA section** (`<![CDATA[` … `]]>`, exact case). The span is RAW + SEEDED, the §3.0 disposition of a §4.6 block line applied to the same kind of text: never inline-parsed (a bracket inside an attribute value or a comment is no link delimiter — `<span title="[child](absent.md)">` once made a false unavailable-memo miss, rc 2), never a naming site (an id inside an attribute is masked, kind `html`), recorded in `Memo.raw` with the `inline` reading and seeded under the one raw-line rule when it holds a `\|` or a declared id; a `<` the grammar refuses is text and the brackets after it are read (`<3 [x](y.md)`, `<a href="x" [x](y.md)>`, `< span>`, `</ span>`, `<a b="c"d>` Example 622, `<! …>`, `<![cdata[`, `\<span …>`). §4.6 start condition 7 ("a complete open tag … or a complete closing tag") reads the same `OPEN_TAG` / `CLOSING_TAG` — ⚠ until R17 the tag grammar was spelled a second time in `plan_memo_blocks.py` | `plan_memo_lexer.py::_HTML_TAG` / `OPEN_TAG` / `CLOSING_TAG` / `inline_pass` (`html`), `Lexed.html`, `plan_memo_tables.py::dispose` (`html`), `plan_memo_memo.py::Memo._inline_raw` (seed), `plan_memo_blocks.py::_HTML_BLOCK` (t7) | ✓ controls "(html) the R17 reviewer's shape `<span title=\"[child](absent.md)\">text</span>`…" and the "(html) …" R17 family (each arm positive, each negative rc 2 or a reported site, the two left-to-right probes, the cell, `[<span>x</span>](…)`), "(lex-seed) `<span title=\"Slice 9z owns it\">`…" (+ the `inline` reading); **falsifier = the section's own list, `Raw HTML` Examples 613–632, vendored in `commonmark-0.31.2-inline-html-examples.json`** — the runner's inline conformance control: the spans Phase 2 masks are exactly the text the html emits verbatim (each span verbatim in order in its `<p>` body, and the body's unescaped `<` count — minus two per code span — equals the spans'); mutants R17 #2 (the `<` arm dropped; quoted values dropped; the comment arm dropped; 0.30's comment exclusion re-injected; the PI / CDATA arms dropped; `<!` + anything; optional whitespace before an attribute; the seed record dropped; the `html` disposition dropped) | no |
| CommonMark §6.2 Emphasis and strong emphasis (+ GFM 0.29 Strikethrough) — **design re-gate 4** | which delimiter runs PAIR | a delimiter run is a maximal run of `*` / `_` (CommonMark) or one-or-two `~` (GFM: "a matching pair of one or two tildes", so three or more is literal — `a~~~b~~~c` renders verbatim, measured on GitHub's pipeline); LEFT-FLANKING = not followed by Unicode whitespace and either not followed by Unicode punctuation or preceded by whitespace or punctuation (right-flanking mirrors it; the classes are Unicode by the spec's own words — P* or S* for punctuation, 0.31's definition — never ASCII, since the surrounding text of these memos is Japanese as often as not); `*` opens iff left-flanking and closes iff right-flanking, `_` adds §6.2 rules 5–6 (`snake_case` stays intact), `~` reads the `*` conditions and pairs EQUAL lengths only; then the Appendix's `process_emphasis` — each closer matched to the nearest live opener at or above the bracket's bottom, the RULE OF THREE on the ORIGINAL run lengths, `openers_bottom` so a failed search is never repeated, the delimiters between a matched pair removed — run where the Appendix runs it: when a link or an image closes (over the delimiters inside it) and once at the block's end, so emphasis never crosses a link's text boundary. A pair inside a RESOLVED image's description is DEMOTED (§6.4: plain string content, no `<em>` — the R19 rule, one more construct). What the caller wants is not a tree but the CHARACTER SPANS the delimiters occupy, since those are what the stream drops | `plan_memo_emphasis.py::run_at` / `_matches` / `process`, pushed by `plan_memo_lexer.py::inline_pass`, disposed by `plan_memo_tables.py::dispose` | ✓ | no |
| (no spec clause — this checker's disposition, over CommonMark's rendering) — **design re-gate 4** | ONE stream: the block as the document renders it | each construct contributes text, nothing, or its character (`RENDERS_TEXT` + the §2.4 / §2.5 substitution, §3.0b's `Renders` column); a DROP beats a BLANK where they overlap (the fact of the spec beats this checker's policy — a link tail holding a `.md` file token settles it); the two id-decoration exceptions (`id_only` over a code span and over a `**` pair); the offset map back to raw coordinates; and the residue where a unit straddles a blanked span, reported as `[LEX-SPLIT?]` and, in a declaring field, as a schema miss | `plan_memo_tables.py::stream` / `Stream` / `dispose` / `split_units`, `plan_memo_memo.py::Population._kind_residue`, `plan-memo-umbrella-check.py::lex_split_seed` / `Block.at_raw` | ✓ | no |

### §3.0 Block grammar — the spec's CLOSED list (the bound IS this table)

CommonMark 0.31.2 enumerates its block types in §4 (leaf blocks) and §5 (container blocks); GFM 0.29
adds §4.10 tables. Every type has a disposition here, and since R15 every one is **LEXED** (a Phase-1
clause with a control — a leaf, a RAW extent, or a container: the §5.1 block quote, the §5.2 list item
grouped into §5.3 lists). ⚠ Until R15 §5.2 was **LEXED-FLAT** (the marker line started a paragraph;
nesting and laziness not modelled) with an item-open bit seeding the one place that hid prose. A
`[LEX-UNSUPPORTED?]` SEED names a RAW line the lexer
never inline-parses — an HTML-block line (§4.6), an indented-code line (§4.4; a fence excepted, the
author's explicit code marker) or, since R17, an INLINE raw-HTML span (§6.6, keyed on the line it starts on;
the §3 row) — when it holds a `|` or a declared id, under ONE seed rule (design re-gate
3 IMP-2: an indented schema row after a table's rows is raw under cmark-gfm too and left the census
SILENTLY, rc 0 and no seed — the I-C class — while a raw HTML line holding a `|` was seeded), printed with
its READING (`html` / `indented` / `inline`; the `item` reading is gone with the flat reading): a seed
in the ORDER-PROSE? idiom, never an inventory. Section numbers
are the spec's own (`.claude/tools/webref specs commonmark` = no source: webref carries no CommonMark
extract, so this list is cited from the 0.31.2 text directly, re-verified 2026-08-23).

**The falsifier of every row is the spec's own example list** (R12): `spec.commonmark.org/0.31.2/spec.json`
is official and machine-readable, and its examples for `Tabs` §2.2, §4.1–§4.9, — since design re-gate 3
(MIN-6) — `Block quotes` §5.1, Examples 228–252, and — since R15 — `List items` §5.2, Examples 253–300, and
`Lists` §5.3, Examples 301–326 (295 of the 652; fields `example` / `section` / `markdown` / `html`) are
vendored in
`.claude/tools/commonmark-0.31.2-block-examples.json`. The control `plan_memo_selftest_conformance.py`
(every `--self-test` run) puts each through `Memo` and consumes the expected html against Phase 1's
**block sequence** — since R13 the pass's OWN statement of what it read (`Memo.sequence`: `[kind, first
line, last line]`, a container before its content; a list entry carries its first marker and its
tightness), nothing re-derived from dropped lines — a block quote
`<blockquote>` … content … `</blockquote>`, a list `<ul>` / `<ol start="N">` … items … `</ul>`, an item
`<li>` … content … `</li>`, a raw HTML extent verbatim, an indented / fenced extent
`<pre><code…</code></pre>`, `<hr />`, `<hN>` at the `#` count or 1/2 for `=`/`-`, `<p>…</p>` — or, as a
direct child of a TIGHT list's item, bare inline text up to `</li>` or the next block tag, a `<p>` there
refusing the tight claim (⚠ the first R15 aligner searched to `</li>` and read past a `<p>`, so every
"loose" mutant survived: the claim could not go red) — a definition
nothing, the html exhausted at the end — never a rendering (Phase 2 is skipped over). Excluded by
predicate over Phase 1's own output, printed per run: a GFM table Phase 1 admitted (local policy over pure
CommonMark; no vendored example holds a `|` where a table could open — empty by construction), and NOTHING
ELSE: **295 aligned / 0 excluded / 0 FAIL**. Before R15 the count was 208 / 13 / 0 over 221, the 13 (4 5 7 9
57 60 61 94 99 108 109 175 235) being every example a §5.2 list-marker line headed a paragraph in — the
LEXED-FLAT disposition (design re-gate 3 MIN-4 had narrowed the predicate to "marker line FIRST in the
paragraph" so a `starts_block` failure would FAIL rather than hide) — and 184 / 12 / 0 over the 196 before
§5.1 was vendored; before R13 169 / 27 / 0, the 27 being exactly where R13 landed: 8 block-quote examples
(6 92 93 101 128 174 214 218) and 7 §4.4 chunk shapes (85 110 111 112 114 115 225) under the old
PROSE-AS-WRITTEN dispositions, plus the 12 list examples. Each disposition was replaced by the grammar
(§4.4 RAW, §5.1 container, §5.2 container) and its exclusions went to 0; what a kind sequence cannot see
(inline content, the spec's html for it) is skipped over, not excluded, and §5.3 tightness IS seen, since the
list entry states it. A FAIL there is a Phase-1 defect or an unstated disposition, never a control
rewrite; the mutant that drops `div` from the type-6 list (`search`, the reviewer's example, has no spec
example) reds it at 153–161/185, the four-space literal at Tabs 1–2, the §4.4 opener arm at 100 among
others, the marker's tab rule at 6, the lazy setext arm at 93, the thematic-break precedence at 11 / 51 / 53
/ 54 / 60 / 61 / 105, the flat list reading at 78 examples. Two rounds of spec-table transcription
errors in both directions preceded this control.

| § | Block type | Disposition | Phase-1 site | Control |
|---|---|---|---|---|
| §2.2 | Tabs | LEXED — ONE indentation measure: a tab advances to the next multiple of 4 columns; every block start reads "up to three spaces" and §4.4 "four or more" through it (`\tfoo`, ` \tfoo`, ` \t# foo` are indented code; a tab-indented header/delimiter row is not a table) | `indentation` / `unindented` / `is_indented` (`_match`, `fence_opener` / `fence_closes`, `html_block_type`, `table_header_at`, `reference_definitions`) | "(lex-seed) `\tSlice 9z owns it`…", "(lex-seed) ` \tSlice 9z owns it`…", "(table) `\t\| Slot \| …`…"; spec examples Tabs 1–3, 8–11, 82 |
| §4.1 | Thematic break | LEXED — one-line block, ends a paragraph / table; `---` after paragraph text is a §4.3 underline instead (Example 59); after a table's rows it is the thematic break (no paragraph is open) | `one_line_block` (`_THEMATIC`) | "(span) a paragraph ends at an ATX heading" family; "(setext) a `---` after paragraph text…"; "(table) `---` right after a schema table is a thematic break…"; spec examples |
| §4.2 | ATX heading | LEXED — one-line block; its text is inline content | `one_line_block` (`_ATX`) | "(span) a paragraph ends at an ATX heading"; spec examples |
| §4.3 | Setext heading | LEXED — paragraph text + `=`/`-` underline; the underline ends the paragraph and is not content; a boundary ONLY where a paragraph is open (`block_end`'s `para_open` arm: after a table's rows `===` is a body row) and never on a lazy line of a container (Example 93 in a quote, Example 94 `- Foo\n---` in an item — the `lazy` arm; R15: `container_text`, which stood for Example 94 under the flat reading, is gone); read before a list marker, in commonmark.js's block-start order (`foo\n-` is a heading, `- a\n  -` an item holding one) | `block_end` (`setext_underline`), `Memo._parse` | "(setext) `Heading\n===`…", "(setext) `==` after a list item is NOT an underline…", "(table) `===` right after a schema table is a one-cell body row…", "(setext) a bare `===` IS a paragraph…", "(quote) `> Heading `open\n===\nSlice 9z owns it` here`…"; spec examples |
| §4.4 | Indented code block | LEXED — a RAW extent (R13), exactly like a fence: opened by a line of four or more columns where NO paragraph is open ("cannot interrupt a paragraph": `[foo]:\n    code` is a definition, `text\n    x` a paragraph; after a table's rows, a one-line block, a heading or a blank line it IS a block start — the same `para_open` bit as type 7), running over the §4.4 chunk (consecutive indented lines; a blank line stays inside when an indented line follows it, Example 111; the trailing blank lines are not part of it), never inline-parsed; SEEDED under the one raw-line rule when a line holds a `\|` or a declared id (design re-gate 3 IMP-2; a fence is not — the author's explicit code marker), inside a container too (`- item\n\n      x` is code IN the item, R15) — cmark-gfm agrees that an indented row after a table is `<pre><code>` (measured) | `raw_opener` (`("indented", None)`, `is_indented`) / `raw_extent`, `Memo._parse` (`raw`) | "(indented) `    Slice 9z owns it` at a block start is a raw extent…", "(indented) `text\n    Slice 9z owns it`…", "(indented) `\tSlice 9z owns it`…" (×3), "(table) an indented line right after a schema table opens an indented code block…", "(lex-seed) `para\n\n    Slice 9z owns it`: … seeded by the one raw-line rule", "(lex-seed) a 4-space-indented slot row after a table's rows … is SEEDED", "(lex-seed) a tab-indented slot row …", "(lex-seed) an indented-code line with neither a `\|` nor a declared id is no seed"; block-sequence control (chunk shapes, raw content asserted); spec examples 85 107–118 225 aligned, 100 with `---` |
| §4.5 | Fenced code block | LEXED — a RAW extent: never inline-parsed, always a run / paragraph / table end; one opener rule (`raw_opener`) and one extent rule (`raw_extent`) shared with indented code and HTML blocks, consumed in place by the driver; an unclosed fence runs to the end of its containing block (the document, or the quote's first lazy line) | `plan_memo_blocks.py::fence_opener` / `raw_opener` / `raw_extent` | "(fence) …" family (10 controls); block-sequence control (`> ```\nlazy`) |
| §4.6 | HTML block | RAW (+ SEED) — a leaf block "treated as raw HTML", exactly like a fence: its lines from the opener to the §4.6 end condition (types 1–5 by content, possibly the opener itself; 6–7 at the next blank line) are a RAW extent under the same opener / extent rules as fences (`raw_opener` / `raw_extent`), never inline-parsed, always a run / paragraph / table end; a type-7 opener only where no paragraph is open — the driver's block state, so after a table, a one-line block or a setext heading it IS a block start (R11) — type 1–6 openers interrupt; **case per condition** (R13): conditions 1 and 6 name their tags "(case-insensitive)" (a scoped `(?i:…)`, ASCII-only), 7 is "a complete open tag … or a complete closing tag" — §6.6's, the lexer's ONE tag grammar `OPEN_TAG` / `CLOSING_TAG` (R17; ⚠ spelled a second time here until then), 4 is `<!` + an ASCII letter of either case, 2 `<!--` / 3 `<?` / 5 `<![CDATA[` are exact — a global IGNORECASE read `<![cdata[` as CDATA where commonmark.js reads a paragraph; every raw HTML line holding a `\|` or a declared id is a `[LEX-UNSUPPORTED?]` seed (commonmark.js: `<pre>\n`\n</pre>` is raw, `text\n<span>` is a paragraph, `# h\n<span>` a heading and a raw block, `<div>\nx\n</div>\ny` runs to the blank, `text\n<PRE>` / `text\n<DIV>` a paragraph and a raw block — ⚠ the case probes must interrupt a paragraph, since at a block start `<PRE>` is a type-7 opener too) | `plan_memo_blocks.py::raw_opener` / `raw_extent` / `html_block_type` / `html_block_ends`, `Memo._parse` (seed) | "(lex-seed) an HTML-block opener holding a declared id is a seed", "(html) …" family incl. the four R13 case controls, "(table) a type-7 HTML opener right after a schema table ENDS it…"; spec examples 152 (`<DIV CLASS="foo">`), 182 (`<![CDATA[`) — ⚠ this cell once cited 151 / 190, which are `</div>\n*foo*` and `<table>` (design re-gate 3 MIN-7, verified against `spec.json`) |
| §4.7 | Link reference definition | LEXED — a block of its own at a block start, parsed over the rest of its run; elsewhere an orphan; inside a block quote it registers (Example 218) | `Memo._parse` / `definition_block` (`run_end`) | "(def) …" family, "(quote) `> [sib]: slice-9z-sib.md`…" |
| §4.8 | Paragraph | LEXED — the inline unit Phase 2 scans; inside a block quote, at its real line | `Memo._parse` / `Paragraph` | every prose control |
| §2.1 / §4.9 | Blank line | LEXED — "A line containing no characters, or a line containing only spaces (U+0020) or tabs (U+0009), is called a blank line" (§2.1); ends every block (inside a §4.4 chunk it stays when an indented line follows) | `is_blank` | "(span) an NBSP-only line is NOT blank…" |
| §5.1 | Block quote | LEXED — a CONTAINER (R13): the marker (`quote_content`: ≤3 columns, `>`, one optional space of indentation — a tab gives one column to the marker and the rest to the content, in LINE columns, Example 6) stripped from every marker line, the lines without a marker gathered as lazy continuation candidates, and the SAME Phase 1 run over the content (`Memo._quote` → `_parse` — each pass a generator frame on `Memo._run`'s EXPLICIT stack, never the interpreter's: R16, ~500 nested `>` raised `RecursionError` inside `_parse`; 1,000 nested quotes / items parse, as commonmark.js 0.31.2 nests them, measured; control "container nesting is off the call stack…", mutant re-injects a depth cap of 200), so a definition / table / raw extent / paragraph / nested quote inside is that block at its real line; laziness through the ONE predicate's `lazy` arm (a candidate is a boundary where no paragraph is open, and with one open by every arm but the setext underline, Examples 92–93; a lazy line that is a GFM table header with a RUN open — paragraph text, or a reference definition just consumed — is that table's header — cmark-gfm, measured, design re-gate 3 MIN-9 and R17); a `>` line interrupts a paragraph (§5.1 itself); no seed (the content is parsed) | `quote_content`, `block_end` / `raw_extent` / `table_header_at` (`lazy`), `Memo._quote` | "(quote) …" family (8 controls, the lazy-header / lazy-delimiter slot tables included), "(span) a paragraph ends at a `>` line", "(block) `[Slice 9z owns it]:\n>`…"; block-sequence control (the quote shapes — the R17 lazy-header family included — with Example 6's raw content asserted); **falsifier = the section's own list, `Block quotes` Examples 228–252: 25 aligned, 0 excluded, 0 FAIL (235, a list item, excluded until R15) — 237 `> ```\nfoo\n```` is the driver's stop at a lazy candidate where no paragraph is open** (⚠ `Memo._quote`'s docstring once cited Examples 128 / 174 for that stop; both end their quotes at a BLANK line, the gather's stop); 6 92 93 101 128 174 214 218 from the other sections aligned; "block quotes are linear…" |
| §5.2 / §5.3 | List item / list | LEXED — a CONTAINER (R15), exactly like the block quote: `item_marker` reads the marker (≤3 columns; `-` / `+` / `*`, or 1–9 ASCII digits then `.` / `)`, followed by a space, a tab or the line's end — `-foo` is text, `١.` is text, and a thematic break takes precedence, §4.1 Example 61) and the item's content indentation W + N in LINE columns (N as written when 1–4; 1 when ≥5 — the item starts with indented code, Example 270 — or when the first line is blank, Example 278), re-spelling the first line's content from its true column (`-\t\tfoo` is code holding `  foo`, Example 7); every later line at that indentation is content with it stripped (`strip_columns`, the §2.2 tab rule), a blank line stays, and a line short of it is a LAZY CONTINUATION CANDIDATE through the SAME gather and `block_end` lazy arm as the quote's (§5.2 rule 5 restates §5.1's "paragraph continuation text": ONE mechanism; an enclosing container's candidate stays lazy and whole inside the item); the SAME `_parse` over the content, which stops at a candidate where no paragraph is open (`- a\n\nfoo`: the item is `a` + its blank line); "at most one blank line" to begin (Example 280: `-\n\n  foo` is an empty item and a paragraph). **Interruption** (§5.2, `starts_block`'s `para_open` bit): with a paragraph open only a non-empty item, ordered only from 1, interrupts (`foo\n2. b`, `foo\n-` one paragraph; `foo\n1. b`, `foo\n- b` a paragraph and a list); on a lazy candidate any marker line is a boundary (the unmatched container closes: `- a\n2. b`, `> a\n2. b`, `> a\n*` a container then a list). **Lists** (§5.3, `_list`): sibling items of one type — the same bullet character or the same ordered delimiter (`- a\n2. b` and `1. a\n1) b` are two lists, Examples 301–302) — with the blank lines between them; the entry states the first marker (`<ul>` / `<ol start="N">`) and TIGHTNESS by §5.3's rule (loose when a non-final item ends with a blank line — recursing into a nested list's last item as commonmark.js's `endsWithBlankLine` does — when blank lines stand between two items at the list's level, or when an item opens a block across a gap; a quote's gaps are its own, an item's blank first line opens none). ⚠ Until R15: LEXED-FLAT + an item-open bit — the marker line headed a paragraph, the content indentation was tracked by nothing, and the item's next paragraph at it (Example 108) was consumed as indented code, seeded (`item` reading) but never scanned: `- item\n\n    [child](child.md)` never walked `child.md`, rc 0. The bit, the seed reading and `container_text` are deleted; nothing here is seeded (the content IS parsed) | `item_marker` / `_same_list` / `strip_columns`, `starts_block` (`para_open`), `block_end` (`lazy`), `Memo._list` / `_item` / `_parse` (`gap` / `loose`) | "(item) …" family (20 controls: the reviewer's input → `child.md` walked; `1. item` at 3 columns; the third paragraph; a quote inside; the 1-column line and the 0-column heading ending the item; code at six columns, seeded; laziness; the three interruption probes; a definition in an item in a quote / in a quote in an item; a slot table in an item declares its id), "(span) a paragraph ends at a list item…", "(setext) `==` after a list item…", "(ascii) `١.` is not a list marker", "(table) a list item right after a schema table ends it…"; block-sequence control (17 list shapes: siblings of two types, `foo\n-`, `- a\n  -`, `* a\n* * *\n* b`, Example 280, tightness five ways, the tab rule, an item in a quote and a quote in an item); **falsifier = the sections' own lists, `List items` 253–300 and `Lists` 301–326: 74 aligned, 0 excluded, 0 FAIL**, and the 13 list examples of the other sections aligned |
| GFM §4.10 | Table | LEXED — see the §3 rows above; the id cell of a schema row is scanned too, with the row's own id suppressed (`**7z** — Slice 9z lands first` names `9z`); a NON-schema row's excess cells are ignored before lexing (GFM §4.10, verbatim: "If there are a number of cells fewer than the number of cells in the header row, empty cells are inserted. If there are greater, the excess is ignored" — ⚠ misquoted as "If greater, the excess is ignored" before design re-gate 3 MIN-8; R13 — an ignored cell's `[x](absent.md)` is no link), a schema row's width miss unchanged; inside a container — a block quote, a list item (cmark-gfm, measured, R15) — a table is a table; its delimiter row may not be lazy, while a lazy header row IS the header where a run is open — paragraph text (MIN-9) or a reference definition just consumed (R17: `> [a]: /u\n| h |\n> |---|` is quote[def, table]; the definition registers, where cmark-gfm prints it as a paragraph — the stated divergence) (cmark-gfm, measured) | `Memo._parse` (`table_header_at`) / `admit_table` / `split_row`, checker `blocks()` (`Row.line`, the content line) | "(row)" / "(table)" families; "(id) the id cell's trailing prose is scanned…"; "(table) an excess body cell of a non-schema table is ignored…" (×3); "(quote) a slot table inside a block quote is a table…"; "(item) a slot table inside a list item is a table…" |

Inline constructs outside the lexed rows (§6.5 autolinks, §2.5 character references in PROSE — in a link
DESTINATION they are decoded, the §2.5 row of the §3 table, R16 — §6.2 emphasis beyond the `**` / `` ` ``
decoration the id grammar reads) are read as written; **§6.6 raw HTML is LEXED since R17** (the §3 row) with
the disposition of a §4.6 block line applied to the inline form — RAW + SEEDED: the span is never
inline-parsed and never a naming site (an id inside an attribute value or a comment is masked, kind `html`),
and it is a `[LEX-UNSUPPORTED?]` seed with the `inline` reading when it holds a `|` or a declared id (the one
raw-line rule; `a<br>b` and `<sub>2</sub>` seed nothing); the spec's `Raw HTML` Examples 613–632 are the
falsifier (**20 aligned / 0 excluded / 0 FAIL**). The umbrella memo's
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
sites and 36 ORDER-PROSE? rows unchanged. **6 since R15**: the four `item` lines (2955 / 2958 / 2959 / 2960)
are the item's own paragraphs now — parsed, not seeded — and only the six `|`-holding shell lines remain,
each with the `indented` reading; census 48, 717 sites, 36 ORDER-PROSE? rows unchanged.

### §3.0b Inline grammar — the spec's CLOSED list (the bound IS this table)

CommonMark 0.31.2 enumerates its inline constructs in §6 (plus §2.4 backslash escapes and §2.5
character references, which inline parsing reads); this table gives every one a disposition, exactly
as §3.0 does for block types. It exists because the inline pass was **grown one construct at a time
as the reviewer found them** — R17 §6.6 raw HTML, R19 §6.4 image descriptions, R20 the file token,
R21 §6.5 autolinks — the same shape the block phase had before R13's closed list ended it. Four
consecutive rounds of `plan_memo_lexer.py` findings is the PAUSE this table answers: after it, a
missing construct is a row that says so, not a round. ⚠ **Design re-gate 4 measured how far that
goes**: no construct was missing from the list, and the defect was in a row's own words — §6.2 read
PROSE-AS-WRITTEN with "Cost: none measured", an assertion where the column promised a measurement,
and the cost was in fact a fabricated site on one row, a lost site on another and, through the same
split, a lost kind. A closed list bounds what can be FORGOTTEN; only a measured cost bounds what can
be WRONG about what is on it, which is why every PROSE row now carries a control that measures its
cost.

**Disposition vocabulary** — LEXED: a Phase-2 clause with a control. MASKED: lexed and then skipped
whole, so nothing inside it is a link, a naming site or a sibling. PROSE-AS-WRITTEN: not lexed; its
text is read by the scanners exactly as written, and the row states what that costs.

**Renders** is the column design re-gate 4 added, and it is what the scanners actually consume: the
stream is the block AS THE DOCUMENT RENDERS IT (`plan_memo_tables.stream`), so each construct
contributes to it either **text** (its own characters, or — for a MASKED one — the same count of
blanks, since the checker refuses to read them but a reader does not read across them either),
**nothing** (the span is dropped and the text on both sides of it is ONE run), or **its character**
(a substitution). The column is not commentary: `RENDERS_TEXT` in `plan_memo_tables.py` is the same
table, read by the one stream builder, and a mutant per row turns it red. Before that re-gate the
stream was the source text with the masks blanked, which read `Slice 9**z**` — a document that
renders `Slice 9z` — as the row `9`, and `**UMBRELLA, not a *terminal* unit.**` as no declaration
at all: a fabricated site on one row, a lost site on another, and a row silently out of the umbrella
census at rc 0.

| § | Inline construct | Disposition | Renders | Phase-2 site | Control / corpus |
|---|---|---|---|---|---|
| §2.4 | Backslash escapes | LEXED — one parity helper; an ODD run escapes, and the same helper answers the row splitter (`\|`) and the inline pass | **its character** — the escape and the §2.5 reference are the two spellings of one substitution, at the one site the stream builds; the backslash renders nothing. EXCEPT a `*` or a `` ` ``, which stands as written: the stream carries exactly one kind of markup (a kept id decoration), and substituting `\*\*C\*\*` would spell a bold `**C**` the document does not have (measured — the umbrella memo quotes a `grep` pattern in that shape) | `plan_memo_lexer.py::_is_escape` / `inline_pass`, `plan_memo_tables.py::stream` | spec examples (13); "(row) `a\\|b` holds an UNESCAPED pipe"; "(kind) …§2.4 an escaped comma DECLARES the umbrella…"; "(render) `\*\*C\*\*`…" |
| §2.5 | Entity and numeric character references | LEXED **everywhere** (design re-gate 4) — in a link DESTINATION by `normalize_destination`, because that text must equal a file name (R16), and in PROSE by the same grammar in the one inline pass | **its character**, under the §2.4 row's one rule and its one exception | `plan_memo_lexer.py::normalize_destination` / `_CHAR_REF` in `inline_pass` | spec examples (17); "(§2.5) a character reference in PROSE renders its character…" (`Slice &#57;z` IS a site: 1, where the row above measured 0 — **the re-measured cost of the old reading was never `0 sites`, it was a lost site**); "(kind) …§2.5 a character reference for the comma…"; "(render) `&#42;&#42;C&#42;&#42;`…" |
| §6.1 | Code spans | LEXED → MASKED (backtick strings of equal length). Disposition exception: an id-only span is the document SPELLING an id, not code (`plan_memo_tables.py`) | **text** (blanked) — the one construct that renders characters this checker refuses to read (I-A), so it bounds what it sits between and a unit read ACROSS it is the `[LEX-SPLIT?]` residue, not a decision | `inline_pass` / `_code_closer` | spec examples (22); the code-span family; "(render) `Slice W`z` owns it`…" + its seed |
| §6.2 | Emphasis and strong emphasis (+ GFM 0.29 strikethrough, the same delimiter machinery) | LEXED (design re-gate 4) — the spec's delimiter runs, flanking rules, rule of three and `process_emphasis`; a `~` run of one or two is the GFM extension's, three or more is literal. Disposition exception, the code span's own: a `**` pair whose content is only declared ids is the document DECORATING an id, so its delimiters STAND and bound it (`**9z**7z` is `9z` then `7z`, never the token `9z7z`) | **nothing** for a matched pair; an UNMATCHED run is literal **text** and bounds what it sits between | `plan_memo_emphasis.py` (`run_at` / `process`), pushed by `inline_pass`, disposed in `plan_memo_tables.py::dispose` | spec examples (132, `Emphasis and strong emphasis` 350-481); the "(render)" family (the LOST-SITE and FABRICATED-SITE probes over `W**z**` / `W*z*` / `W~~z~~`, the literal `W*z`, the intraword `W_z_`, `W~~~z~~~`, `**9z**7z`, `*9z*7z`); 8 mutants. **The old row's "Cost: none measured" was false**: the cost was a fabricated site on one row and a lost site on another, and, through the same split, a lost kind |
| §6.3 | Links | LEXED — the Appendix bracket stack; inline / full / collapsed / shortcut; a link deactivates every earlier `[` | **nothing** for the `[` and the tail (`](dest)` prints no character); the link TEXT is the document's text there and stays prose (B×D) | `inline_pass` | spec examples (90); the link family; "(render) §6.3 a link's brackets `W[z](…)`…" |
| §6.4 | Images | LEXED — not a link; destination never a sibling; a RESOLVED description is plain text, so a link inside it — and, since design re-gate 4, an emphasis pair inside it — is demoted (R19: no tag of its own) | **text** (blanked) for the tail: an image puts a picture in the flow, not the letters of its alt text, so its two sides are not one word — while the description is scanned as prose, the stated deviation | `inline_pass` (the `is_img` arm) | spec examples (22); the R19 image family; the demotion is what makes Examples 573 / 576 / 577 / 585 / 589 align (`alt="foo bar"` emits no `<em>`) |
| §6.5 | Autolinks | LEXED → MASKED whole (R21) — ONE token tried at a `<` **before** the tag grammar (the spec's order); its contents are not inline syntax, so a bracket inside it opens nothing and an id inside it is no site | **text** (blanked) — its text IS its URL, which a reader reads | `inline_pass` / `_AUTOLINK` | spec examples (19); the R21 autolink family (12 controls, 5 mutants) |
| §6.6 | Raw HTML | LEXED → **DROPPED** whole (R17 masked it; design re-gate 4 made the mask a drop) — one tag grammar (open / closing tag, comment, PI, declaration, CDATA) whose tag bodies are also §4.6 condition 7's. Seeded (`[LEX-UNSUPPORTED?]`) when it holds a `\|` or a declared id | **nothing** — a tag or a comment is markup, not text: `W<b>z</b>` and `W<!-- c -->z` render `Wz`, and a `Deps` cell holding only a comment is EMPTY | `inline_pass` / `_HTML_TAG` | spec examples (20); the R17 raw-HTML family; "(render) §6.6 …`W<b>z</b>` / `W<!-- c -->z`…"; "(cell) a `Deps` cell holding only an HTML comment is EMPTY…" |
| §6.7 | Hard line breaks | PROSE-AS-WRITTEN | **text** — the break's own markup stands in the stream (a `\` before a line ending escapes nothing: §2.4 is ASCII punctuation and a line ending is none; two trailing spaces are two spaces). **Cost, measured**: none, and for a stated reason rather than an unexamined one — the break renders a LINE ENDING, and a line ending bounds every unit the scanners read, so no reading of the markup can join what the break separates | — | "(§6.7) a hard line break's own markup stands in the stream … and costs nothing" (`Slice W\` + `z` names no `Wz`, on the probe table where `Wz` IS the umbrella); "(§6.7) a HARD line break inside a link's text does not break the link" (1 site) |
| §6.8 | Soft line breaks | PROSE-AS-WRITTEN — same reading as §6.7 (the joined block text) | **text** — the line ending itself, the same bound. **Cost, measured**: none, by the same argument and its own control | — | "(§6.8) a SOFT line break is the same measurement: `Slice W` then `z` on the next line renders two words and names no `Wz`"; and the §6.7 control for the link case |
| §6.9 | Textual content | PROSE-AS-WRITTEN — the base case: every character not claimed above is text the scanners read (this is where ids and row nouns are found at all) | **text**, itself | — | every naming control in the suite |

**Inline conformance corpus** (`commonmark-0.31.2-inline-examples.json`, the §3.0 corpus's sibling):
the spec's own examples for every LEXED row — Backslash escapes 13, Entity and numeric character
references 17, Code spans 22, **Emphasis and strong emphasis 132** (Examples 350–481, vendored at
design re-gate 4), Links 90, Images 22, Autolinks 19, Raw HTML 20 = **335 examples, 335 aligned, 0
excluded, 0 FAIL**. The §6.2 claim the aligner checks is one `<em>` per matched pair of one delimiter
character a side and one `<strong>` per pair of two, `_` and `*` alike, and a `<del>` for none of
them — pure CommonMark has no strikethrough, so the GFM half of that row is **empty by construction
on this corpus**, the same shape §3.0's GFM-table exclusion has (measured: the alignment would fail on
any example where a `~` run paired, and none does). The three PROSE-AS-WRITTEN rows are not vendored
because they make no structural claim to falsify — their cost is the named control instead, which is
why each such row carries one. "Cost: none measured" is written in this table only where the §6.2
row NAMES what it replaced: a cost is a measurement or it is a guess.

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
made to pass by hand under the one predicate (§5.1 block quotes with lazy continuation and §4.6
HTML blocks were, R10–R13, in ~150 lines; §5.2 list items with §5.3 lists, the last container, in ~130
more at R15 — the trigger did not fire for any of them, and the spec's closed block list is now
exhausted; §6.2 emphasis with GFM strikethrough, the LAST inline construct, in 216 more at design
re-gate 4, where the falsifier is the spec's own 132 Emphasis examples — the trigger did not fire
there either, and the closed INLINE list is now exhausted too) or if the `[LEX-UNSUPPORTED?]` seed on the umbrella memo reports a line holding a `|`
inside an HTML block (a table the lexer would read that CommonMark would not). Therefore
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
  ⚠ **PR #510 Codex R15 (2026-09-08) — the last Phase-1 container family closed**: §5.2 list items are
  CONTAINERS exactly like block quotes (`item_marker` / `strip_columns` / `_same_list`, `Memo._list` /
  `_item`, the same `_parse` and the same lazy gather; §5.3 lists with tightness stated in the sequence
  entry; the §5.2 interruption rule on `starts_block`'s `para_open` bit; the §4.1 thematic-break
  precedence in `item_marker`), replacing the LEXED-FLAT reading, the item-open bit, the `item` seed reading
  and `container_text` — the reviewer's `- item\n\n    [child](child.md)` (Example 108's shape) was
  consumed as indented code, so the link was never found and `child.md` never walked, rc 0. #2: an orphan
  definition keeps its DESTINATION and the population miss is raised only where `sibling_path` (the one
  resolver) maps it to a sibling on disk — `[x]: #section` / `[x]: https://…` orphans no longer promote a
  later `[x]` to the exit-2 miss. #3: `ROW_NOUN` is ASCII case-insensitive in one place (`(?ai:…)`), so
  `SLICE C owns it` / `ROW 9` / `UMBRELLA C` name a row. `List items` 253–300 and `Lists` 301–326 vendored
  (221 → 295 examples); conformance **208 / 13 / 0 → 295 / 0 / 0** — the 13 §5.2 exclusions are gone and
  NOTHING remains excluded (the GFM-table arm is empty by construction; §5.3 tightness IS checked, since the
  list entry states it and the aligner refuses a `<p>` under a tight claim — ⚠ its first draft read past one
  and three "loose" mutants survived, caught by the mutant run). **324 controls, 184 mutants / 0 / 0**,
  `--self-test --mutants` 7.0 s; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites,
  the same 717** (`--worklist` file/line/id/source set identical to `956c640b`'s tools on the same memo);
  what moved: the ROLE vocabulary of 4 of them (1202 `2a` — → landing, 1757 `2ab` +landing, 2790 `0b` — →
  acceptance, 3053 `1a` +owner) and one LICENSED mention back (2959 `10a`, inside the 2955–2960 chunk,
  which is item `10.`'s paragraph again), both by the one §5.2 rule: an item paragraph's content no longer
  carries the marker nor each continuation line's W + N columns, so the licensing window (80 characters of
  the block's stream) reaches further back into the prose at those sites. `[LEX-UNSUPPORTED?]` **10 → 6**
  (the four `item` lines are parsed; the six `|` shell lines remain, `indented`). `plan_memo_selftest_mutants.py`
  split at the review-round seam first, as a standalone prereq commit (§7). `scripts/trip-wires.sh` rc 0.
  ⚠ **PR #510 Codex R16 (2026-09-08)**: #2 (IMP) §2.5 character references in a link DESTINATION are decoded by
  the ONE destination normalisation (`normalize_destination`, §3 row; every destination site returns through
  `link_destination`) — `[child](child&#46;md)` and `[sib]: child&#46;md` were backslash-unescaped only, so the
  sibling was silently outside the population, rc 0; labels / titles / code spans are NOT decoded, each with a
  control. #3 (IMP) container nesting is off the call stack: the Phase-1 passes are generator frames driven by
  `Memo._run`'s explicit stack (~500 nested `>` raised `RecursionError` inside `_parse`, and the population
  chokepoint — then `OSError | RuntimeError | UnicodeDecodeError` — reported the memo as "linked memo
  unavailable (RecursionError)", rc 2, no census), and the chokepoint is I/O ONLY (`OSError |
  UnicodeDecodeError`; `RuntimeError` is caught at `_resolve` alone, the py≤3.12 symlink loop; a parser
  exception is a crash out of `check()`, crash = FAIL — control + mutant each). #1 (MIN) §7's Slice 0 / 1 / 2
  bullets state the per-PR arrangement (this document IS Slice 0 + 1's per-PR plan; Slice 2 gets its own).
  **341 controls, 191 mutants / 0 / 0**, `--self-test --mutants` ~9 s (8.9 / 9.6 s measured; 7.0 s at R15 — the
  1,000-deep control runs `Memo` twice and `check()` once, and again under the mutant that names it);
  conformance **295 / 0 / 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the
  same 717** (`--worklist` identical to `1ad7e523`'s tools on the same memo — the memo holds no character
  reference anywhere, `grep -c '&[a-zA-Z#][a-zA-Z0-9]*;'` = 0, so no site could move), `[LEX-UNSUPPORTED?]` 6.
  `scripts/trip-wires.sh` rc 0.
  ⚠ **PR #510 Codex R17 (2026-09-08)**: #1 (IMP) a reference DEFINITION keeps the run open, so a lazy GFM table
  header right after it inside a block quote or a list item is that container's table (`Memo._parse`: ONE rule,
  `cur or i == def_end`, the lazy line handed to `admit_table`; cmark-gfm MEASURED on fourteen variants, the table
  in the docstring) — the reviewer's `> [a]: /u\n| # | Slice | … |\n> |---|…` in a LINKED memo dropped the schema
  table, its `#11-…` slot never entered the census and, schema presence being required of the main memo only,
  rc 0. The "not modelled" note is gone; the ONE divergence that stays (cmark-gfm prints that definition as a
  paragraph and registers nothing; here it registers) is stated in §2, the docstring and the §3.0 table row.
  Control + mutant: the runner's "a lazy schema header after a definition in a linked memo's quote is a table…"
  (id declared, kind umbrella, census +1) and the "(quote) the R17 reviewer's shape…" family, the block-sequence
  shapes; the mutant re-injects `cur` alone. #2 (IMP) CommonMark §6.6 raw HTML is a span of the ONE inline pass
  (`_HTML_TAG`: open / closing tag with the attribute grammar — unquoted / single- / double-quoted values —
  the 0.31 comment rule, processing instruction, declaration, CDATA; §4.6 condition 7 now reads the same
  `OPEN_TAG` / `CLOSING_TAG`, one spelling), masked and seeded exactly as a raw HTML-block line is (the §3 row,
  §3.0): `<span title="[child](absent.md)">text</span>` was read as a link — a false unavailable-memo miss, rc 2
  — and ids inside attributes leaked into the scans. `Raw HTML` Examples 613–632 vendored; inline conformance
  control **20 / 0 / 0** (⚠ its first draft EXCLUDED a non-paragraph read by predicate, and the
  optional-whitespace mutant survived through that arm — Example 622 became a type-7 HTML block; the arm is
  deleted, such a read FAILS). ⚠ The R17 brief's negative `<span title=[x](y.md)>` is a TAG (an unquoted
  value admits brackets and parentheses; commonmark.js agrees) and is a NEGATIVE control saying so. Ten
  R17 #2 mutants (each arm dropped or widened, the seed record and the `html` disposition dropped). #3 (MIN)
  the resolver's scheme-before-decoding claim cites WHATWG URL §4.4 `#scheme-start-state` / `#scheme-state`
  and §1.3 `#string-percent-decode` (webref `dfn url` / `body url`), in the docstring, the §3 row and the R8
  control's name. **381 controls, 202 mutants / 0 / 0**, `--self-test --mutants` 10.7 s; conformance **295 / 0
  / 0** + **20 / 0 / 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same
  717** (`--worklist` identical to `dbbeeb8e`'s tools on the same memo at `190d2adb`, save the seed note's
  wording; the population holds ONE inline raw HTML span, `<this file>`, holding no id — `[LEX-UNSUPPORTED?]`
  6, unchanged). `scripts/trip-wires.sh` rc 0.
  ⚠ **PR #510 Codex R19 (2026-09-08)**: #1 (IMP) CommonMark §6.4 — a link that closed while an IMAGE opener was
  still on the bracket stack joined the population as it closed; if the image then resolved, its description is
  plain text (`![alt [docs](absent.md)](image.png)` → `<img alt="alt docs">`, commonmark.js), so the inner link is
  no memo link and the rc-2 miss for `absent.md` was false. ONE rule at the point the image closes (`inline_pass`,
  offset containment — the entries recorded past the image's `[` are the trailing ones): a link inside a RESOLVED
  image's description is DEMOTED to a masked tail (never a memo link, never prose), a nested image stays masked, a
  failed reference there names no lost memo; an UNRESOLVED image is literal `![` and keeps its inner link
  (`![alt [docs](child.md)][missing]` walks `child.md`). The link-in-link mirror needs no rule (the inner link
  deactivates the outer `[` as it closes; `[a [b](x.md)](y.md)` is the R1-3 control). Nine R19 "(image)" /
  "(link)" controls, four mutants (the immediate add re-injected; a plain drop for the demotion; the
  failed-reference rule dropped; demotion re-injected on the FAILED image). #2 (MIN) diagnostics printed
  `path.name`, so `a/child.md:1` and `b/child.md:1` were one `child.md:1` in `--worklist`, the findings and the
  population summary: `Population.display(path)` is the ONE helper every printer reads (the worklist's `Block.file`,
  the findings in `plan_memo_roles.py` / `plan_memo_memo.py` / the checker, the summary) — the path RELATIVE to
  the root memo's directory, the resolved absolute path when the memo lies outside it. Runner control "diagnostics
  name a memo relative to the root memo's directory…" (two same-named siblings told apart in the sites' and the
  seeds' file columns; a memo one directory up named absolutely); mutant re-injects `.name`. ⚠ The R19 brief
  presumed §6 figures might quote a `child.md:N` shape and that controls' expected text might carry a basename:
  neither exists (0 controls adjusted). #3 (IMP) `sibling_path` stage (c) rejected a leading `/` only, so the
  decoded `C%3A%5Ctemp%5Cchild.md` / `%5Cchild.md` passed and on Windows `parent / name` discarded the memo's
  directory: (c) is now ONE platform-independent rule — the decoded name is read under Windows path syntax
  (`PureWindowsPath`, the superset; WHATWG URL `#path-state` step 1 reads a special-scheme path the same way and
  calls the drive-letter quirk "platform-independent") and must have no anchor (drive, UNC, `\` or `/` root,
  drive-relative `C:`); (e) joins the name's PARTS, so **`\` is a separator on every platform, never a POSIX
  name character** (`sub%5Cchild.md` walks `sub/child.md` everywhere). Seven "(rc)" / "(link)" controls
  (drive-absolute, `\`-rooted, percent-encoded UNC, raw `\\server\share\x.md` → `\`-rooted by §2.4,
  drive-relative raw = stage (a) and encoded = stage (c), the one-letter `n%3Achild.md`, the positive
  `sub%5Cchild.md`), two mutants (the `/`-only test re-injected; the POSIX join re-injected); the R3-2 / R5-2 and
  R7-2 mutants re-targeted at the new (c) line, the R3-1 image-tail mutant at the new `is_img` arm.
  **398 controls, 209 mutants / 0 / 0**, `--self-test --mutants` 10.0 s; conformance **295 / 0 / 0** + **20 / 0
  / 0**; census `48 = 33 + 15`, rc 0, 36 ORDER-PROSE? rows unchanged, **717 sites, the same 717** (`--worklist`
  byte-identical to `4c15b7a7`'s tools on the same memo, path column included: the population's four memos all
  sit in the root memo's directory, so the relative names ARE the former basenames); `[LEX-UNSUPPORTED?]` 6,
  unchanged. `scripts/trip-wires.sh` rc 0.
  ⚠ **De-flake (commits `b0a7def4` + this note, after R19)**: the wall-clock ratio gates (`t(4N)/t(N) < 8`) were host-dependent in BOTH directions — once, under host load, a mutant killed only by such a gate survived (209 mutants, 1 survived; 6 clean runs followed); reproduced with 30 bursty CPU-load processes, the same gates went RED on the unmodified production code in 4 of 5 probes (`split_row` 8.4, orphan detection 9.3–10.6, `linked_files` 7.4–7.6 against the bound 8). The three controls whose mutants (`R6-2 locate`, `R8-5 sibling`, `R9 #3 row`) were killed only by a ratio now carry deterministic counting witnesses — `unresolved_references`: reads of the line-offset table ≤ N·(⌈log₂(N+1)⌉+2) (a bisect per site; 11,977/55,905 measured against 12,000/56,000) and ≥ N; `linked_files`: `PurePath.__eq__` ≤ N and `__hash__` ≥ N (a set hashes, a list compares: 0/0 eq, 2,001/8,001 hash); `split_row`: source lines executed in the module ≤ 64·(len + cells) (`sys.settrace`, 4 functions = 58 lines, 6.1/unit measured) — and the orphan-detection control keeps its exact call count and drops its ratio gate; counts are identical on CPython 3.9.6 and 3.14.6; timing is informative only. Measured: 10 consecutive `--self-test --mutants` runs under the 30-process bursty load → `209 mutant(s), 0 survived, 0 crashed` each (15.8–20.8 s wall); unloaded 3.4 s.
  ⚠ **PR #510 Codex R20 (2026-09-08)**: #2 (IMP) — the R14 family from the KIND side: `_APPOSITIVE` was built on
  `ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(SHORT_ID)`, so a declaring field attributing the marker to a SLUG row
  (`Slice `#11-zz-alpha` — **UMBRELLA, not a terminal unit**`) matched nothing: the row was read as its OWN
  umbrella declaration, no `UMBRELLA-MARK` attribution finding, the pointer row inside the census, exit 0 — and the
  R14 spelling sweep is blind to it (a `SHORT_ID`-only composer spells nothing twice). Fix in the grammar:
  `ROW_KINDS = ("slug", "short")` / `ROW_ID` (`plan_memo_ids.py`), the ONE "a row id here" alternation — a citation
  id keys a citation-table row but declares no kind, is never `Slice [C1]`, never owns, never carries the marker, so
  it is not a row in this sense — composed by every row-position reader: `ROW_NOUN_ID` → `_APPOSITIVE`
  (`plan_memo_tables.py`), `OWNS_TWO` (`plan_memo_roles.py`, whose local `(?:slug|short)` `_OWNER_CORE` was a second
  spelling of it and is gone), and the anchored reading `_anchored` (`t.kind in ROW_KINDS`; ⚠ it read
  `t.kind != "short"`, so a slug after a row noun was reported by the bare pass but never `anchored`, and the (c)
  seed's prose-vs-`Deps` comparison never saw a slug named in a Slice cell). Sweep of every `SHORT_ID` /
  `decorated_id(SHORT_ID)` use: routed — `ROW_NOUN_ID`, `OWNS_TWO` (2 groups), `_anchored`; kept — `_ID_RUN_TOKEN`
  (all THREE kinds: `` `[C1]` `` in an id-only span is the document spelling a declared citation id), `KINDS` /
  `_TOKEN` (the grammar itself), the checker's `tid.isdigit()` comment (the short kind's declared miss);
  `_TRAILING_NOUN` composes NO id (it ends where the mention's token starts, whatever its kind) — the R20 brief
  listed it as a composer to route, which is a false premise. Controls: "(a) the R20 reviewer's declaring field
  …attributes the marker to a SLUG row…" (UMBRELLA-MARK 1), the bold-slug and short forms, "(d) `owned by
  `#11-zz-alpha` and **Qx**`…", "(c-seed) `Lands after Slice `#11-zz-alpha``…" (+ its negative), the runner's
  `attribution_control` now over both kinds (count unchanged, kind `pointer`), and the PROPERTY "every row-id
  composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)" — 12 probes over
  the grammar's tuple (a kind without a sample is red). Mutants: `ROW_ID = SHORT_ID` (kills all five at once),
  `decorated_id(SHORT_ID)` re-injected at `ROW_NOUN_ID`, the short-only test re-injected in `_anchored`; the F6 (c)
  mutant's re-injected `re.findall(SHORT_ID, …)` now names the grammar module explicitly (the roles import it relied on
  is gone). #1 (MIN) the lexer's file-token arm required a stem of ≥1 character while `sibling_path` (d) accepted the
  bare suffix, so beside a declared id `md` the prose `Read .md for details` reported `md`: ONE notion of "is a file
  name" — `FILE_SUFFIX` defined in the lexer, consumed by `sibling_path` stage (d), stem unconstrained on both sides
  (the token arm's run is `*`). Controls "(file) `.md` alone is a file name…" (0 sites), "(file) `notes.md` beside a
  declared no-owner id `md`…" (0), "(file) bare `md` (no suffix)… IS a site" (1, the live subject), "(link) `[x](.md)`
  links the sibling file named `.md`…"; mutants: the `+` re-injected in the lexer; a stem requirement re-injected on
  the memo side (the two sides must agree). #3 (MIN, plan) the §3 Phase 1 → Phase 2 row's coverage cell cited its
  control as proving a definition after a table "ENDS the table" while the mechanism column, the control's name and
  its expectation (a one-cell ROW, width miss rc 2 — GFM Example 202) all say ROW: the cell now cites the control as
  named. Every other "ends it" claim in the plan (a list item after a table, a type-7 opener after a table) matches
  its control's rc-0 expectation. **409 controls, 214 mutants / 0 / 0**, `--self-test --mutants` 5.0 s (×2);
  conformance **295 / 0 / 0** + **20 / 0 / 0**; census `48 = 33 + 15`, rc 0, **717 sites, the same 717**
  (`--worklist` vs `ecd2bdcc`'s tools on the same memo: the file / line / id / source / role columns are identical;
  two context columns shift left because an anchored mention's span now starts at the row noun — display only);
  `[LEX-UNSUPPORTED?]` 6, unchanged; **`ORDER-PROSE?` 36 → 37**: the one moved row is §5's `Function`/`eval` pointer
  row (line 1985, empty id cell, printed as `row None`), whose Slice cell writes `the umbrella
  **`#11-vm-function-constructor-global`** (§8 — this row's own Slot cell) charters …` — the slug after the row noun
  `umbrella` is now the anchored reading, so it enters the seed's prose set, and the `Deps` cell (a prose
  `HostEnsureCanCompileStrings` prereq) does not carry it. The named id is the row's OWN slot (its Slot cell), not a
  dependency: a seed hit of the (c) seed's declared natural-language class, invisible before only through the kind
  hole this round closes. `scripts/trip-wires.sh` rc 0. ⚠ Two R20 brief premises were false: `_TRAILING_NOUN` is not
  an id composer (above), and "the umbrella memo has 15 slot rows; if any declaring field attributes to a slug row
  the census WILL change" — no declaring field in that memo does (48 unchanged); what moved was the (c) seed, via
  `_anchored`, which the brief did not name. **#3 (MIN, follow-up commit)** — that finding printed `row None`: every
  printer composed `row %r` of `Row.self_id` itself (eight sites in `plan_memo_roles.py`, one in `plan_memo_memo.py`),
  so a row whose id cell is the literal blank `**—**` was named by nothing a reader could find. One naming site now:
  `Row.name()` — the id's repr when the cell declares one (identical output for every keyed row), else the
  declaring LOCATOR `<no id> at :LINE (TOKEN)` (the line and the declaring field's first token, decoration
  stripped; line 1985 prints `row <no id> at :1985 (Function/eval)`); `Population.attributed` carries the name, not
  the id. ⚠ The brief's "the ONE place rows are named in findings" did not exist — there were nine inline
  `row %r` sites, and the fix is what creates the one place. Control (function-shaped, the message text is the
  subject): an `extra` §5 table with a `**—**` row whose Slice cell opens `` `Function`/`eval` `` and orders after
  a slug its Deps cell lacks — ORDER-PROSE? x1 on that line carrying the locator, no finding of the run spelling
  `row None`; mutant `R20 #3` re-injects the unconditional `repr(self.self_id)`. 410 controls / 215 mutants 0 / 0;
  census 48 / 717 / 37 / 6 unchanged.
  ⚠ **PR #510 Codex R21 (2026-09-08)** — the round that closed the INLINE family, and a **Step-4 PAUSE**:
  `plan_memo_lexer.py` had drawn an IMP in four consecutive rounds (R17 §6.6, R19 §6.4, R20 the file token,
  R21 §6.5), the shape the block phase had before R13. Option A: **§3.0b**, the spec's CLOSED list of inline
  constructs (§2.4, §2.5, §6.1–§6.9) with a disposition each — LEXED / MASKED / PROSE-AS-WRITTEN — and a
  control MEASURING each PROSE row's cost. #1 (IMP) §6.5 autolinks are ONE masked token tried at a `<`
  before the tag grammar, so `<https://example.com/[child](absent.md)>` walks nothing (the false rc 2 is
  gone); 14 `(autolink)` controls, 5 mutants. #2 (IMP) the marker short-circuit is the SEED's alone, so a
  row marked in its declaring field AND again elsewhere is reported. #3 (IMP) KIND-SPELLING gates on
  `pop.spellings`, never on a non-empty undetermined set. #4 (IMP) each `Schema` declares the id KINDS its
  id column may key, applied in the one `bare_id`. Inline corpus 20 → **203 examples, 203 aligned, 0
  excluded, 0 FAIL** (block 295 / 295 / 0). **439 controls / 226 mutants 0 / 0**; census 48 / 717 / 37 / 6
  unchanged, finding-kind counts identical to R20's head.
  ⚠ Five R21 mutants first SURVIVED, all one class: each named a control whose SUBJECT sat outside the span
  the mutation changes (an id glued to a domain label; a link placed after the would-be autolink instead of
  inside it; a space after `<`, which fails at the SCHEME rather than in the tail class the mutation
  widens). Each subject was moved inside the mutated span and each control now records why it is shaped so
  — the `feedback_surviving-mutation-means-the-probe-has-another-subject` class, third sighting on this PR.
  ⚠ **PR #510 design re-gate 4 (2026-09-08) — the stream IS the rendered text**: the re-gate over
  R14–R21 measured two live defects of ONE class, an id or a phrase **split by an inline construct**
  where the RENDERED document reads one token: (i) with rows `9z` and `9` both declared, every one of
  `Slice 9**z**` / `Slice 9<b>z</b>` / `Slice 9<!-- c -->z` / `Slice 9~~z~~` renders `Slice 9z` and was
  reported on row **`9`** — a fabricated site on one row and a lost site on another, rc 0; (ii)
  `**UMBRELLA, not a *terminal* unit.**` in a declaring field (or `&#44;` for the comma, or
  `~~terminal~~`, or `<b>terminal</b>`, or an escaped `\,`) made the literal `MARKER` test — and the
  near-miss backstop `DECLARES`, which carries the same literal — miss, so the row left the umbrella
  census as an ACTIVE TERMINAL, silently, at rc 0: **the "clean exit for a could-not-scan" §1
  forbids**, and the more serious half. Both are the same defect and take ONE fix, which is reading
  (a) of the three the re-gate put: **the scanners consume the text a reader sees**. §2 I-A now states
  it, §3.0b carries the per-construct `Renders` column that decides it, and the id scanners, the
  marker, `DECLARES`, the undetermined and pointer spellings, `EMPTY_WORDS` / `is_empty`, the
  acceptance and ordering vocabularies, the ownership connectives and the licensing rule all read that
  ONE stream — the sweep is re-runnable and its result is an enumeration, not a sample: `grep -n` for
  `MARKER` / `DECLARES` / `UNDETERMINED` / `POINTER` / `EMPTY_WORDS` / `ACCEPT_WORDS` / `ORDER_WORDS` /
  `RETIRED` / `OWNS_TWO` / `ROLE_PATTERNS` / `LICENSE_BEFORE` / `LICENSE_AFTER` / `NOUN_ANCHOR` /
  `_APPOSITIVE` over `plan_memo_roles.py plan_memo_tables.py plan_memo_memo.py
  plan-memo-umbrella-check.py`, kept to the lines that TEST one (`search` / `match` / `finditer` /
  `in` / `.sub(` — `grep -E '\.search\(|\.match\(|\.finditer\(| in |\.sub\('`), returns **18** sites, and every one of them reads `b.stream`, `m.text`,
  `m.window()`, `row.field`, `stream(c.lexed)` or `_stream(row, …)`. `is_blank_id_cell` / `bare_id`
  are the stated exception, since the id cell's decoration IS part of the id grammar and the two readings of that
  cell agree by being the same reading. Readings (b) and (c) were rejected on §1, not on cost: (b)
  "join across the invisible spans" needs the same §6.2 machinery (a `*` is invisible only when it
  PAIRS — `9*z` renders verbatim), and still cannot read `&#44;` or `\,`, which are not invisible but
  SUBSTITUTED; (c) "seed the class" leaves the wrong row named in the report and leaves the census
  wrong at rc 0, which is the very exit §1 forbids. Where the answer IS mixed, the boundary is stated
  and is not arbitrary: reading (a) covers every construct that renders no character; where a
  construct renders text the checker refuses to read (§6.1 code spans, §6.5 autolinks, an image tail,
  a citation id, a file name) the stream is deliberately NOT the rendered text (I-A), the two readings
  disagree, and neither is this program's to pick — so that residue is (c), reported by `split_units`
  as `[LEX-SPLIT?]`, and where it decides the census (the marker in a DECLARING field) it is a schema
  miss, rc 2. New: `plan_memo_emphasis.py` (§6.2 + GFM strikethrough, 216 lines), `Stream` with its
  offset map, `RENDERS_TEXT`, `split_units` / `_straddles`, `Population._kind_residue`,
  `lex_split_seed`; `Emphasis and strong emphasis` Examples 350–481 vendored (inline corpus 203 →
  **335 / 335 / 0**, block 295 / 295 / 0), the aligner claiming one `<em>` per 1-character pair and
  one `<strong>` per 2-character pair, which is what caught the missing §6.4 demotion of emphasis
  inside a resolved image's description (5 examples). Two second spellings of decoration are gone with
  it: `is_empty`'s own `.strip("*`")` (it reads the cell's stream now) and `LICENSE_AFTER`'s
  `(?:\*\*)?` prefix (measured: removing it moves no control and no site). **492 controls, 250
  mutants / 0 survived / 0 crashed** (+53 controls, +24 mutants; 8 of the 24 SURVIVED their first run,
  every one the R21 class — a named control whose subject sat outside the mutated span: a comment
  blanked in a `Deps` cell is still an empty cell, a link tail that FOLLOWS both halves of a token
  splits nothing, an unmatched `*` is unmatched under both flanking readings, and a probe of runs that
  can only OPEN never enters the search `openers_bottom` guards. Each subject was moved inside the
  mutated span and each mutant re-run); conformance **295 / 0 / 0** + **335 / 0 / 0**; census `48 = 33
  + 15`, rc 0, 37 ORDER-PROSE? rows unchanged, `[LEX-UNSUPPORTED?]` 6 unchanged, `[LEX-SPLIT?]` **0**
  (the memo holds no straddle; the seed's controls are the fixtures), and — the one finding kind that
  DID move, so this list is a delta and not a sample — `[UMBRELLA-MARK?]` **6 → 8**: main:2671 `an
  **umbrella**` and main:2672 `**"Terminal under §5's criterion…` each state the kind vocabulary in a
  declaring field with the word INSIDE emphasis, which is precisely the text this reading now sees;
  both are gains, and rc is 0 either way because the class is a seed. Memo run **0.80 s → 1.41 s**
  (the residue detector renders each block a second way, which is what the seed costs; measured twice
  each, `/usr/bin/time -p`, `c6be7995`'s tools against this commit's on the same memo),
  `--self-test --mutants` **3.5 s**. **Sites 717 → 714, and the three that moved are the fix working**
  (`--worklist` against `c6be7995`'s tools on the same memo: file / line / id / source identical but
  for these; nothing gained, 1,205 mentions unchanged, 488 → 491 licensed): main:1944 `4a` in `each of
  *4a's* children`, detail:199 `1b` in `1b's **charter**` and main:2551 `A` in `row **A** is an
  *umbrella* row` are each REPORTED → LICENSED, because the licensing phrase the emphasis used to
  break — `'s children`, `'s charter`, `is an umbrella` — is now read as the document renders it. The
  context and role columns shift on other lines (a role window of 110 characters of the stream reaches
  further when the delimiters are gone): a RANKING over the reported set, never a filter on it —
  measured, 8 sites gain a role vocabulary they always rendered (`(no role vocabulary)` 246 → 238).
  `scripts/trip-wires.sh` rc 0.
  ⚠ **PR #510 Codex R22 (2026-09-08)** — three P2s, each a reading that disagreed with itself.
  #1 (P2) the raw-line seed read ids with the id grammar ALONE, so a declared id inside a `.md` file
  name (`slice-9z-sib.md` on an HTML-block or indented-code line) seeded `9z` where the same name in
  prose seeds nothing — a seed whose contract is "what a naming scan would have read had this been
  prose" reporting what no naming scan would read. The root is that "a bare `.md` file name or a
  citation id, over RAW text" had no name: `Lexed.__init__` spelled it and the seed compensated with a
  hand-written `t.kind != "cite"` covering only the citation half. `plan_memo_lexer.file_and_cite_spans`
  is now that ONE reading, consumed by both, and the hand filter is REMOVED rather than kept beside it.
  Subsumption measured, with its denominator: over the vm-p4 memo's **7** raw lines the hand filter and
  the `cite` spans agree on **7 / 7**, and the `file` spans then remove ids on **1** further line —
  which is main:2343, the defect. Seven lines is a sample, not a sweep, so the argument that carries the
  removal is the CONSTRUCTION rather than the count: the hand filter dropped a token whose KIND was
  `cite`, the spans drop a token that OVERLAPS a `cite` or `file` span, and the second is the weaker
  test on the citation half except where a citation escapes its own `cite` span — `([C1]).md`, where
  the balanced-paren arm swallows it, and the `file` span covers it there.
  #2 (P2) `sibling_path` stage (c) took an empty `PureWindowsPath.anchor` as sufficient, so the
  relative `NUL.md` / `dir/NUL.md` passed; on Windows that OPENS and yields an empty stream, so the
  population counted a linked memo it never scanned and could exit 0 having dropped the sibling — §1's
  forbidden clean exit again, and the same class design re-gate 4 closed. `_is_reserved_component`
  applies CPython `ntpath._isreservedname`'s DEVICE reading (cited to Microsoft "Naming Files, Paths,
  and Namespaces") per PART, in pure string logic so it decides identically on POSIX and is testable
  there; neither `PureWindowsPath.is_reserved()` (deprecated 3.13, removed 3.15) nor
  `os.path.isreserved` (3.13+, `ntpath`-only) is callable by a checker that must decide this the same
  way everywhere. The CHARACTER half is deliberately NOT repeated, and the boundary is drawn on a
  discriminator rather than on taste: a device opens successfully and returns empty (silent, rc 0),
  whereas a reserved character raises `OSError` on Windows and is ALREADY an unavailable linked memo at
  rc 2 — and rejecting `*?"<>:|` here would contradict R8's decided reading of `notes%3Achild.md` as
  the local file `notes:child.md`. Cost named, not hidden: a POSIX memo genuinely called `NUL.md` is no
  sibling now either, dropped without a report exactly as `/abs/x.md` and (since R19) the POSIX file
  named `sub\child.md` are — this stage's standing polarity, one platform-independent reading.
  #3 (P2) the undetermined-kind marker matched inside a longer word (`MANKIND UNDETERMINED`,
  `KIND UNDETERMINEDNESS`, even `unkind undetermined`), which with a non-empty `Deps` cell emits
  `UMBRELLA-CELL` and moves rc to 1. Fixed as a SWEEP, not as the reported spelling: there was no
  spelling of "a phrase boundary" to compose, so `plan_memo_ids` now owns `BEFORE` / `AFTER` /
  `bounded(phrase)` (the module that already owns `ALNUM` and is the only one the spelling sweep lets
  spell a character class; `NOUN_ANCHOR` composes `BEFORE` instead of re-spelling the lookbehind), and
  every phrase matcher the census reads was audited. Six are now bounded — `MARKER_RE` (a bare `in`
  test at 6 call sites; `SUBUMBRELLA, not a terminal unit` matched), `UNDETERMINED`, `POINTER`, and
  **three the review did not name and the sweep found**: `DECLARES` (unbounded both sides),
  `LICENSE_BEFORE` (unbounded left — `grandchild of 9z` licensed) and `LICENSE_AFTER` (unbounded right
  — `9z's memorandum` licensed). `LICENSE_*` is the dangerous polarity, since a match SUPPRESSES a
  report, so tightening is the safe direction. The matchers left alone (`ROLE_PATTERNS`, `ORDER_WORDS`,
  `ACCEPT_WORDS`, `RETIRED`, `OWNS_TWO`, `EMPTY_WORDS`, `ID_CELL_BLANKS`) each carry the reason now —
  `\b` under `re.ASCII` IS a boundary, differing from `ALNUM` only on `_`. Residual named rather than
  claimed away: `classify` hands the backward look a 40-character slice, so a 40-character licensing
  phrase begins at index 0 and the lookbehind succeeds vacuously (measured reachable).
  **517 controls, 264 mutants / 0 survived / 0 crashed**; conformance 295 / 0 / 0 + 335 / 0 / 0;
  `scripts/trip-wires.sh` rc 0; census rc 0 with `51 seed(s)`, 1,205 mentions, 491 licensed, 714
  REPORTED and 8 `[UMBRELLA-MARK?]` all unchanged, and the `--worklist` differing on **exactly one**
  line: main:2343 `        docs/plans/2026-07-vm-p4-slice-1a-1b-call-spread-detail.md |` loses `'1a'`
  and `'1b'` — both came out of the FILE NAME, which is #1 working — while the line still seeds,
  because it holds a `|`.
  ⚠ One R22 mutant SURVIVED its first run — the R21 class, **fourth sighting on this PR**: reading
  `p.parts[-1:]` instead of every part left the `dir/NUL.md` control green, because there the device IS
  the final component and the probe's subject sat outside the mutated span. The control's subject moved
  into the span (`NUL/child.md`, the device as a DIRECTORY) rather than the mutant being weakened, and
  the control records why it is shaped so.
  ⚠ Every device control's destination names a file that EXISTS in the fixture directory (the shared
  `DEVICE` set writes all six), so a green NEGATIVE means "not read", never "not found" — the
  distinction this whole class turns on, and the one a fixture-free control could not make.
  ⚠ **PR #510 Codex R23 (2026-09-08)** — five findings, four real, and **one already fixed**: the
  reviewer asked for a right boundary on the licensed possessive nouns (`9z's memoized` and
  `9z's chartered` were licensed), which is exactly what R22 #3's SWEEP had closed a commit earlier.
  Measured at that head: both are REPORTED (1 site each) while `9z's memo` / `9z's charter` stay
  licensed. That is external confirmation of the sweep's premise — the matcher the review did NOT
  name was the one the review would have named next.
  #1 (P2) a §2.5 reference whose character is `*` or a backtick was excluded from substitution (so a
  decoded `*` could not become live markup) and then left STANDING AS WRITTEN, which is a FOURTH
  disposition beside drop / substitute / blank, and the only one that puts characters into the stream
  the document does not render: with a no-owner row `ast` declared, `The &ast; marks it.` reported a
  site `ast` that no reader can see (`The * marks it.` reports none). The re-gate-4 defect in the
  fabricating direction. Disposition chosen: **BLANK** — the span renders TEXT the checker refuses to
  read, which is what `RENDERS_TEXT` means; not a DROP, because `9&ast;z` renders `9*z` and dropping
  would join the sides into a `9z` no reader sees. `[LEX-SPLIT?]` interaction, settled by class and
  not by luck: an id can never straddle such a blank, since the reader-side fill is a `*` or a
  backtick and neither is in any id token's character class — a PHRASE can, and should, so
  `KIND&ast;UNDETERMINED` is rc 2.
  #2 (P2) the residue gate covered the marker and nothing else, so `` KIND UNDETER`MINED` `` — which a
  reader reads as `KIND UNDETERMINED` — reclassified the row as terminal, dropped its no-owner
  mentions and exited **0**. ⚠ **This was a defect in re-gate 4's own fix**: I gated the phrase I was
  looking at and left every other phrase that decides a row's kind authoritative by default, which is
  the enumerated-exemption class this program keeps meeting. The fix is the construction, not the
  addition: `KIND_PHRASES` is now the ONE enumeration, `_kind` builds its hits from it and matches
  nothing directly, the gate and the seed iterate the same tuple, and a PROPERTY control reads
  `_kind.__code__.co_names` (nested code objects included) and fails on any name that resolves to a
  `re.Pattern` outside the tuple — so the next phrase is covered by default AND cannot slip past.
  ⚠ The same class in the OPPOSITE direction was found while fixing it and is not in any review:
  `` KIND `x` UNDETERMINED `` classified the row *undetermined* at rc 1, because a blank stands as
  spaces and `UNDETERMINED`'s `\s*` reads across it a kind the reader (`KIND x UNDETERMINED`) does
  not. Gating only the reader side would have left that authoritative, so the gate asks per phrase and
  keeps two conjuncts: the two readings must DISAGREE (a phrase spelled cleanly somewhere is not in
  doubt), and the disagreement must come from a STRADDLE (a phrase quoted WHOLE is I-A's deliberate
  disposition and stays no miss).
  #3 (P2) an initial BOM was not stripped, so a linked memo whose first block is a schema table lost
  that table, its rows never entered `Population.ids`, and the run exited 0 with no miss. One U+FEFF
  is stripped at the single place the text becomes lines. ⚠ Honest about the citation: `webref` has no
  CommonMark source and no spec prose is vendored here (only the two example corpora), so §2.1 is
  cited BY SECTION NUMBER WITHOUT a machine-readable source, and the code says so; the corpora cannot
  cover it either (0 of 630 examples holds a U+FEFF). What IS verified in-tree is that `utf-8` decodes
  the BOM to a character — `utf-8-sig` is the codec that consumes it — which is why it landed inside
  line 1. The control that carries the rule is the TWO-BOM one: one is stripped, two are not, which is
  what pins "exactly one" and kills an `lstrip` mutant.
  #4 (P2) nested-image demotion was quadratic against a docstring that claims linear. ⚠ The review
  reported one loop; the second was measured while fixing it and had the identical shape (`pairs`, the
  emphasis half: 0.004 / 0.015 / 0.057 / 0.223 s at n=250 / 500 / 1000 / 2000, also 4× per doubling).
  Root: the range a close must demote was SEARCHED for by walking back over entries an inner close had
  already tagged, so each descendant was tagged once per ancestor — but the range is an O(1) fact of
  the stack, recorded at the `[` exactly as `delim_bottom` already was, and applied once as a union
  via a difference array. Re-measured after: images 0.0013 / 0.0024 / 0.0049 s at n=1000 / 2000 / 4000
  and pairs 0.0017 / 0.0033 / 0.0070 s at n=500 / 1000 / 2000 — linear in both. The work claim is a
  deterministic `_count_lines` witness with a 4×-input / ≤4×-work bound, never wall-clock; the
  correctness half needs no new control, since the 335 inline conformance examples were green on both
  sides of the change.
  **534 controls, 278 mutants / 0 survived / 0 crashed**, 0 `(unknown control)`; conformance 295 / 0 /
  0 + 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; census rc 0 with 51 seeds, 1,205 mentions, 491
  licensed, 714 REPORTED and 8 `[UMBRELLA-MARK?]` unchanged, the `--worklist` differing on exactly one
  line and that line WORDING ONLY (the `[LEX-SPLIT?]` seed's own sentence, since the residue is
  symmetric now and covers every kind phrase, so "the reader reads" and "the marker" were both false
  as written). Cost: the census run 1.46 s → 1.85 s, because `split_units` now builds both renderings
  of every block instead of one.
  ⚠ A PRE-EXISTING mutant (`R21 #1`) survived its first run here, for the R21 reason once more: the
  new `_demote` range covered the demoted-link entries and overwrote the mutation, so the control's
  subject had left the mutated span. The `dem_img.append` moved back before the `out`-pop loop so the
  link conversion stays the ONE site saying a demoted link is not an image. Five other mutants whose
  anchors this round moved failed LOUDLY (`substring occurs 0 times`), never silently — which is the
  difference between an anchor that moved and the `(unknown control)` class R22 collapsed.
  ⚠ **Not yet a control**: the AST check that no constant is declared by both derived mutant
  registries (R22) is still an ad-hoc script. Nothing in `--self-test` enforces it.
  ⚠ **Step-4 PAUSE assessment (2026-09-08, after R23)** — `plan_memo_memo.py` and `plan_memo_tables.py`
  had each drawn a finding in THREE consecutive rounds (R22, R23, R24), which is the signal to stop
  patching and ask whether the mechanism is the problem. Sorting R22–R24's findings by ORIGIN rather
  than by symptom gives **three families**, each still generating: (1) *which text does this reader
  read* — R23 #1, R23 #2 and R24 #4 are all incomplete application of re-gate 4's DECLARED rule, whose
  weakness is that the rule is prose rather than a control; (2) *the boundary where bytes become a
  document* — R22 #2 (device names), R23 #3 (BOM) and R24 #1 (literal NUL), which want §2.1's
  preprocessing as a unit at one site rather than one strip per report; (3) *a matcher's context
  boundary is a magic constant or absent* — R22 #3 (unbounded phrases), R24 #2 (a 70-character
  backward window) and the 40-character slice in `classify` recorded above, which want the boundary
  derived from the grammar. Option A (collapse) is taken for all three, each with a structural guard,
  rather than four symptom fixes — the entry for that round records what each guard cannot see.
  ⚠ **PR #510 Codex R24 (2026-09-08) — the collapse round.** Four findings, all real, fixed as the
  three families above plus one located defect, behind a prereq split. **Touch-time split first**
  (`ef97295d`): `plan_memo_selftest_controls.py` had reached 961 lines and this round adds controls to
  it. Seam = **the measure** — a control that states a COST and counts it with the deterministic
  witnesses moves to `plan_memo_selftest_work.py`, which is now the ONLY importer of `_count_calls` /
  `_count_lines` / `_CountedList`, so "is this a work control?" is an import list rather than a
  judgement (`deep_nesting_control` stays: it counts nothing). Eight bodies moved byte-identically
  (AST-segment checked); 961 → 671.
  **Family 1** (`82552f2d`) — the last raw-text reader. `9z&#32;notes.md owns it.` renders
  `9z notes.md owns it.` and so names `9z`, but `file_and_cite_spans` masked the whole source run and
  the ownership claim vanished (0 sites, against 1 for the rendering-identical prose). Root: `dispose`
  itself asked TWO questions of the raw source; it is two stages now, and stage 2 asks both
  rendered-text questions of ONE text. ⚠ A second member, in no review, was found by enumerating the
  family rather than the reports: the decoration exception's `id_only` also read raw content, so
  `**&#57;z**7z` reported 0 sites where `**9z**7z` reports 1. **Structural guard**:
  `render_equivalence_control` replaces each character of one prose IN TURN by its §2.5 numeric
  reference and requires the census not to move — and the set of positions it must skip is read off
  `inline_pass`'s own code object rather than hand-listed, so under-exclusion is RED and never
  silently weaker. What it cannot see, stated: two-position disagreements, cells, shapes that prose
  does not reach, raw lines.
  **Family 2** (`91bfe590`) — `Memo._preprocess`, CommonMark §2's input preprocessing as ONE unit
  enumerated from the section: U+0000 → U+FFFD, line endings normalised (the file opened with
  `newline=""`), tabs deliberately NOT expanded, and the BOM carried as this program's own encoding
  rule claiming no spec sentence. Before it, a link whose destination was `child\0.md` (a literal NUL
  in the name) was rejected as a control character instead of linking the file, so that memo and its
  violations left the population at rc 0. ⚠ Written WITHOUT the bracket-and-parenthesis form on
  purpose: this checker reads its own plan, code spans are lexed over the whole PARAGRAPH rather than
  per line, and an example spelled as a link inside a long paragraph pairs its backticks somewhere
  else and becomes a LIVE link to a memo that does not exist — measured, it added a sixth FATAL to
  this document before it was rewritten.
  ⚠ **Three of my briefing premises were wrong here and the delegate said so rather than absorbing
  them.** (i) The §2.1 attribution: the subsection number is undeterminable in this tree (the corpora
  carry section NAMES only and hold no example from the quoted sections), so the unit cites **§2**,
  which is certain, and names each transformation by the sentence it implements — no number invented.
  (ii) I claimed the NUL substitution makes `_CONTROL` a guard nothing can trip; it does not, because
  stage (c) tests the PERCENT-DECODED name, so `%00`–`%1f` and `%7f` decode back to controls that the
  unit never touches (verified here: `child%00.md` → not a sibling, and both `STAGE_C` mutants still
  kill its control). (iii) §2's line-ending requirement was already met — by accident, through
  `read_text`'s universal newlines — so it was a requirement nothing PROVED; the control that proves
  it writes **bytes**, since a text write would translate `\n` and make the CRLF arm a claim about the
  host instead of about the checker.
  **Family 3** (`96b3cbb4`) — both context windows are `endpos`/anchor-bounded now
  (`_APPOSITIVE.search(field, 0, m.start())`, `LICENSE_BEFORE.search(m.text, 0, m.start)`), with no
  width constant and no block-length copy in either input; the forward side keeps its slice
  deliberately, since it runs to the end and truncates nothing. ⚠ **A fourth premise of mine was
  wrong**: the 70 was never what held the mention-only direction. The DASH is — `Unlike Slice 7z,
  **UMBRELLA, …**` fails on the comma at any width (verified), while a 76-character slug with the dash
  now attributes where it returned `None` before. **Structural guard**:
  `anchored_matcher_width_control` — an ANCHORED pattern is never handed a subject a NUMBER truncated;
  the predicate is "the receiver resolves to a compiled pattern" rather than a method-name or
  argument-position list, and a local assigned from such a slice counts, so it fires on the
  variable-hidden shape too. Cannot see: a textual anchor test, non-global receivers, taint beyond one
  level inside a function, and truncation inside a helper (`Block.window` / `roles()`, which is not a
  finding: those patterns carry no edge anchor and rank rather than filter).
  ⚠ **A deletion the mutation proof forced.** Porting `_TRAILING_NOUN` into `LICENSE_BEFORE` produced
  a mutant that SURVIVED. It is unreachable: wherever a row noun precedes a declared id, the ANCHORED
  pass reads that site, its span starts AT the noun and it wins the dedup — and dropping the
  `_TRAILING_NOUN` substitution at `3a9f61a0` ALSO turns no control red and leaves the census
  byte-identical, so it was already dead there. Deleted rather than ported, because a clause nothing
  can reach reports coverage the rule does not have.
  **R24-3** (`59f71999`, P3) — a §6.6 span crossing a line ending seeded its whole content against the
  OPENER's line, sending a reader to the wrong place for content the ordinary naming scan deliberately
  masks (so the seed is its only diagnostic). `_inline_raw` yields one entry per LINE now, each located
  through `Paragraph.locate` at its own offset; the control computes the expected line FROM the
  fixture, since a constant would be a claim about `HEADER`'s length.
  ⚠ Two "this mechanism is unreachable" comments quoted the control-registry SIZE at the moment of
  measuring (540, 554) — a count that is stale the moment the next control lands, leaving a reader
  unable to tell a wrong claim from an old one. Both name the MUTATION and the two commands now
  (`a8178006`). ⚠ And one of them had left a *live justification for a dead case* at the rule itself
  (`d420b632`): `stream()`'s drop-over-blank precedence still argued from "a link tail holding a `.md`
  file token", which is precisely the overlap family 1 removed — since the tokens are read off the
  rendering, no `file` span is emitted inside a dropped tail at all. The ordering stands with NO
  witness; it is kept because `disp` is a max over 0 < 1 < 2 and the ordering is what makes that max
  total, and the docstring now says how to look for a shape that would restore one, with the
  denominator stated (four shapes tried, none overlapped — a sample, not a proof).
  **555 controls, 288 mutants / 0 survived / 0 crashed**, 0 `(unknown control)`; conformance 295 / 0 /
  0 + 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; and the #506 census `--worklist` **byte-identical** —
  zero changed lines, which is what a round of root fixes with no census movement should look like.
  ⚠ **Two from the re-gate's own backlog, in no review** (`dd26b5b7`). (a) The row-kind test had TWO
  spellings: the anchored pass asked `t.kind not in ROW_KINDS` — the grammar's closed set — while the
  BARE pass and the residue scan asked its complement (`t.kind == "cite"` / `!= "cite"`). They agree
  exactly while the kinds are {slug, short, cite}, so no behaviour rides on the collapse; what rides
  on it is the NEXT kind, which added to `KINDS` and not to `ROW_KINDS` would be excluded by the
  positive spelling and admitted by the complement — the two passes disagreeing about what a row id
  is, with nothing comparing them. Measuring the complement rather than fixing the site I remembered
  found the second one. (b) The escaped-bracket control measured SCHEMA findings containing `linked
  memo not found`, a phrase NO producer emits (the chokepoint says `linked memo unavailable`).
  ⚠ **My first reading of this was wrong and the mutation proof is what corrected it**: I recorded it
  as a control that could never go red, but a `schema` measure is scored
  `got == expect and (res.rc == 2) == (got > 0)`, so the control DID go red under its mutant — with
  the escape defeated the link goes live, rc becomes 2 while the count stays 0, and the second
  conjunct breaks. What was actually dead is the COUNT half: all the discrimination rested on the rc
  conjunct and the needle was decorative, so a later relaxation there would have left the control
  silently vacuous. Both halves carry now, with a positive partner exercising the count. The property
  that pointed at it — the only substring needle every one of whose cases expects 0, 1 of 31 tuple
  measures — is a POINTER, not a proof of vacuity, and this control is the counter-example proving
  the difference.
  ⚠ **E3 re-measured**, since the figure carried in the backlog (74 of 436) is not comparable to the
  current registry: `523` fixture controls, of which **204 are named by no mutant**
  (`{c.name for c in CASES} - {name for row in MUTANTS for name in row[4]}`). That is the honest size
  of "a control that has never gone red", and it is the largest open item on this PR's own list.
  ⚠ **PR #510 Codex R25 (2026-09-08)** — two findings, both real, and **both landed inside limits R24
  had DECLARED**. Family 1's guard says in its own docstring that it cannot see "positions holding a
  character `inline_pass` branches on"; R25-2 is a backslash, exactly such a character. Family 2 wrote
  its discriminator down as "a device opens successfully, whereas a reserved character raises
  `OSError` and is already reported at rc 2"; R25-1 is a counter-example to that sentence. **An honest
  blind spot is a map of where the next finding lands, not a disclaimer** — the lesson of the round,
  and worth more than either fix (`memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md`).
  #2 (P2) a §6.7 hard line break left a literal backslash in the stream: `Slice\` + newline +
  `C owns it` gave **0 sites** where the soft-break and two-space spellings each give 1 (`C`), because
  `_is_escape` accepts a backslash only before ASCII punctuation, so the pair recorded nothing and
  `NOUN_ANCHOR` could not cross the literal backslash. Fixed as a **mark, not a substitution**: the
  spec says the BREAK renders the line break and the backslash is markup, and since a §6.8 soft break
  reaches the stream as the source `\n` standing as itself, the backslash now renders nothing and the
  line ending stands — the two spellings agree BY THE SAME MECHANISM rather than by two constants that
  coincide. `_is_hard_break` is its own predicate, because widening `_is_escape` would change the link
  grammars, where a backslash before a line ending escapes nothing. The guard is extended at the level
  of the PROPERTY, not the function: no single-character re-spelling can spell a line break, so
  `render_equivalence_control`'s exclusion of `\` is correct and stays (`&#92;` before a line ending
  really does render a literal backslash plus a soft break — a false alarm, not a missed defect), and
  the new property is invariance of the verdict under re-spelling any one line break as each of
  CommonMark's three: 12 re-spellings of 4 breaks, **0 disagreements after, 2 before**.
  #1 (P2) an NTFS **alternate data stream** passes the sibling guard: `notes%3Achild.md` resolved to
  the local file `notes:child.md`, which on Windows opens the stream SUCCESSFULLY if it exists, so the
  census can scan unrelated content and still exit 0. The finding also falsifies the argument R22
  wrote down — "on Windows a name holding one of `*?\"<>:|` raises `OSError`, so stage (e) reports it"
  is false for an ADS path — so that sentence is gone and the predicate is drawn on the POLICY instead
  of on the failure mode. `_is_reserved_component` carries the character half now, for the whole class
  rather than the reported member. ⚠ **This REVERSES R8**, and the reversal is written into the
  control's own text rather than only a commit message: what decided it, what survives of R8 (the
  scheme test still reads the raw path, and `notes%3Achild.md` still has NO scheme — it is stage (c)
  that refuses it now), and what it costs (a POSIX memo genuinely named `notes:child.md`, or holding
  any of the six others, stops being a sibling and is dropped without a report — the standing polarity
  `/abs/x.md`, `C:\x.md`, `sub\child.md` and `NUL.md` already carry). For the six characters other
  than `:`, whether Windows raises is stated as **not determinable from this tree**, cited to
  Microsoft rather than guessed.
  ⚠ **The largest consequence, and it SHRANK the proof**: stage (a), the URL scheme test, is now
  behaviourally subsumed — every scheme ends in `:`, decoding never removes one, and a `:` in any
  component is refused at (c). Two ratified mutants (`F3`, `R8-1`) became **equivalent** — no longer
  killable by any control, which is a permanent FAIL if kept — and were deleted with the reasoning
  recorded where they stood. Verified twice independently: 367 destinations by the author, and **370
  by a differently-constructed sweep** (9 schemes × 8 bodies × 5 encodings plus relative controls,
  comparing `sibling_path` as written against a variant with stage (a) removed), **0 disagreements**
  both times. Stage (a) is KEPT — it asks the URL standard's own question, first and for the right
  reason — and its subsumption is a fact about the current character set: if a destination is ever
  found that (a) refuses and (c) admits, those rows come back with it.
  **570 controls, 289 mutants / 0 survived / 0 crashed**, 0 `unknown control`; conformance 295 / 0 / 0
  + 335 / 0 / 0; `scripts/trip-wires.sh` rc 0; census `--worklist` **byte-identical for the third
  consecutive round**. Two more touch-time splits landed as standalone pure moves: the PROPERTY
  controls, and the sibling-resolver controls — the first `Case` module carved on a **subject** rather
  than a review round, since that resolver had been reported at R3, R4, R5, R8, R19, R22 and R25.
  ⚠ **This document's own checker run, settled.** Running the checker on THIS plan exits 2 with four
  `no table matched schema` FATALs, and that is BY DESIGN, not a defect: `SCHEMAS` matches one
  document family's exact header rows (#506's memo), so any other plan memo is a schema miss — which
  is exactly why the MEMO run is not a trip-wire and only `--self-test --mutants` is (the checker's
  own module docstring says so under WHERE THIS RUNS). What WAS a defect is now fixed: a fifth FATAL,
  an unavailable `child.md`, came from a prose EXAMPLE whose backticks paired elsewhere in a long
  paragraph, making it a live link to a memo that does not exist. Second sighting of that shape in one
  day — the other was in this very ledger — so the rule is written where an author will hit it: code
  spans are lexed over the PARAGRAPH, not the line, and an example spelled as a link inside a long
  paragraph is a live link until proven otherwise. Re-check with
  `python3 .claude/tools/plan-memo-umbrella-check.py docs/plans/2026-08-plan-memo-umbrella-checker.md`:
  four FATALs is the floor, five means someone spelled a link.
  ⚠ **A correction the delegate got wrong, checked rather than accepted**: it reported the plan's
  `51 seed(s)` figure as irreproducible. It is the tool's OWN summary line (`0 mechanical finding(s)
  gate the exit status; 51 seed(s) and 714 reported naming site(s) do not`) — read it with `grep -a`,
  since the log carries a NUL from a fixture and plain `grep` treats the file as binary and prints
  nothing. That `grep` failure is real and worth knowing; the conclusion drawn from it was not.
- **Slice 2**: §4 #4–#6 each with positive + mutant controls, I-E's connective set each a control
  plus the `Unlike Slice 7z` negative; the flipped self-reference control documented; R94 threads
  #4/#5/#6 resolved on #506; slot CLOSE −1.

## §7 Slices

- **Slice 0 — touch-time split (standalone prereq commit, stacked first; planned IN this document, like
  Slice 1 — no per-PR memo of its own)**: `plan-memo-umbrella-check.py`
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
  (Codex R1–R16, the design re-gates) and appends to the same `CASES`; the runner is the one import
  site of both. ⚠ Touch-time split before Codex R15 (`plan_memo_selftest_mutants.py` had reached 981
  lines): the mutant registry is split at the SAME seam — `plan_memo_selftest_mutants.py` keeps the
  row shape, `run` and every pre-converge mutant; `plan_memo_selftest_mutants_pr510.py` holds the PR
  #510 review-round mutants and appends to the same `MUTANTS`; the runner imports both. ⚠ Touch-time split before design re-gate 3 (`plan_memo_tables.py` had reached 879
  lines): `Memo` (the Phase-1 driver `_parse` / `_quote`, Phase-2 resolution, file I/O, the sibling
  resolver) AND `Population` move to `plan_memo_memo.py` — `Population` cannot stay behind, since
  `Population → Memo → admit_table` would then be an import cycle (`admit_table` stays with the
  schemas in `plan_memo_tables.py`); seam = row grammar / schemas / disposition vs the document driver
  and the memo set. One import site per consumer (`plan-memo-umbrella-check.py` imported `Population`
  from `plan_memo_memo` — ⚠ superseded after R22, when `Population` moved to a module of its own, the
  last entry below; the row grammar from `plan_memo_tables`); the runner's loader gains the module
  in dependency order and the MUTANTS rows whose substring moved (45 of the 59 `TABLES` rows) target
  `MEMO`; two re-injections (`is_empty`, `is_blank`) are qualified with `__import__` because the name
  is no longer imported where the row patches (a crash is a FAIL, not a kill). Behaviour-preserving:
  285 controls / 163 mutants 0 / 0, census and the 717-site worklist identical to `8d08b333`'s. ⚠ Touch-time
  split after Codex R20 (`plan_memo_umbrella_selftest.py` had reached 1,083 lines): seam = harness vs
  controls — `plan_memo_selftest_harness.py` (the module loader `load` / `unload` / `patched_module`, the
  fixture runner `run_on`, the `Case` factory `measure` / `control`, the work witnesses `_count_calls` /
  `_count_lines` / `_CountedList`) and `plan_memo_selftest_controls.py` (every function-shaped control,
  `empty_registry_fails` beside its proof `empty_registry_control`, and `registry()`, the one name → (kind,
  control) table); the runner keeps `run()` only. One import direction: runner → controls → harness (the
  controls import the cases modules; the mutant runner imports the harness, not the runner). The two
  MUTANTS rows whose file was the runner (R1-2 the emptiness guard, R12-D the `link_label` counter's
  binding) target `CONTROLS`, exec'd from patched text by `patched_module` and read through the patched
  module's `registry()`. Behaviour-preserving: 409 controls / 214 mutants 0 / 0, census 48 / 717 / 37 / 6
  identical to `883b89d3`'s. ⚠ Touch-time split before Codex R21 (`plan_memo_selftest_cases_pr510.py` had
  reached 949 lines): the control registry is split at the SAME review-round seam a third time, and the
  seam is where the converge turned from Phase 1 to Phase 2 — `_cases_pr510.py` keeps Codex R1–R16 and the
  design re-gates (the lexical substrate, the block grammar, the one pipeline);
  `plan_memo_selftest_cases_inline.py` holds every round from R17 on, which closed the INLINE construct
  family (R17 §6.6 raw HTML, R19 §6.4 images, R21 §6.5 autolinks and the closed §6.1–§6.9 list of §3.0b)
  together with what landed beside them (R19's display name and path syntax, R20's row-kind grammar and
  file token, R21's out-of-field marker / KIND-SPELLING / schema id kinds). Three modules, one `CASES`
  list, one import site (the controls module). Behaviour-preserving: 410 controls / 215 mutants 0 / 0,
  census 48 / 717 / 37 / 6 identical to `e76f2335`'s. ⚠ Touch-time split before design re-gate 4
  (`plan_memo_selftest_mutants_pr510.py` had reached 953 lines and the round adds mutants to it): the
  MUTANT registry is split at **the same R17 seam the cases modules use**, so the two registries are
  split alike and a round's control and its mutant sit in modules of the same name —
  `plan_memo_selftest_mutants_pr510.py` keeps R1–R16 and the design re-gates (the lexical substrate,
  the block grammar), `plan_memo_selftest_mutants_inline.py` holds R17 on (the Phase-2 inline construct
  family). Neither half names anything the other defines (measured: 0 cross-references), so the seam
  needs no shared helper; three modules, one `MUTANTS` list, one import site (the runner).
  Behaviour-preserving: 439 controls / 226 mutants 0 / 0, `scripts/trip-wires.sh` rc 0, and the
  census worklist byte-identical to `c6be7995`'s (`diff` clean). ⚠ Touch-time split after Codex R22
  (`plan_memo_memo.py` had reached 1,005 lines, having stood at exactly 1,000 before that round):
  `Population` leaves for `plan_memo_population.py`, undoing the re-gate-3 bundling above now that the
  cycle argument no longer holds — the cycle was `Population → Memo → admit_table` back into
  `plan_memo_tables.py`, and a module of its own has no such edge (it imports `Memo` and
  `plan_memo_tables`, and nothing imports it but the checker). Seam = ONE memo (block structure,
  lexing, the sibling resolver, and the path helpers only `sibling_path` calls) vs the memo SET (the
  transitive walk, the single I/O chokepoint, the `ids` map every scan reads). The halves share no
  imported name, and the check is the import lines themselves rather than a word count: what
  `plan_memo_memo.py` takes from `plan_memo_tables.py` is now `admit_table` ALONE, while `Population`
  takes `MARKER_RE` / `POINTER` / `SCHEMAS` / `UNDETERMINED` / `attributed_to_other` / `dispose` /
  `is_blank_id_cell` / `split_units` / `stream` and no `admit_table` (the two `stream` hits left in
  `plan_memo_memo.py` are the English word inside two docstrings, which is why the count is not the
  test). 1,005 → **794 + 225**. The 18 MUTANTS rows whose substring moved target a new `POPULATION`
  constant (13 in `_mutants.py`, 2 in `_mutants_pr510.py`, 3 in `_mutants_inline.py`); the `MEMO`
  rows keyed on the shared `STAGE_C` anchor stay, since stage (c) stayed. ⚠ `STAGE_C` was itself a
  defect this split surfaced, and it was MINE: the R17 registry split had left it as two
  byte-identical declarations, one per derived registry, and R22 changed that very source line — so
  both copies had to move in lockstep with nothing enforcing it, and a missed one degrades to
  `(unknown control)`, which the runner prints and walks past. It is declared once now, in the base
  registry beside the file-name constants both registries already import, and an AST check reports
  **no** remaining constant declared by both derived registries. Behaviour-preserving, and
  the class is byte-checked rather than reviewed: `class Population` is identical between the two
  files (10,437 characters, 191 lines, compared programmatically against `89e65c8d`), the only other
  edits to `plan_memo_memo.py` are its docstring and the narrowed `from plan_memo_tables import
  admit_table`, 517 controls / 264 mutants 0 / 0 with **0** `(unknown control)`, `scripts/trip-wires.sh`
  rc 0, and the census worklist byte-identical to `89e65c8d`'s (`cmp` clean).
- **Slice 1 — lexical substrate + one pipeline + one population** (I-A/B/C/F; §3 all rows; §4
  #1–#3; interim connection; header/docstring rewrite). Touch set: `plan_memo_tables.py` (lexer,
  `split_row`, `find_tables`, `links`, `code_spans`, `Memo`), `plan-memo-umbrella-check.py` (`check()`,
  keep-set, guard deletion), `plan_memo_roles.py` (guard deletion only), selftest, the trip-wire,
  `scripts/trip-wires.sh`, `ci.yml` comment, `mise.toml` comment, CLAUDE.md sentence. **Per-PR plan = THIS
  document**: §2–§7 are the umbrella AND Slice 0 + Slice 1's per-PR plan in one memo, and the plan-review the
  Status line records (three rounds, converged 2026-08-22) is Slice 1's pre-implementation review — no second
  memo exists or is owed. Under CLAUDE.md *Edge-dense work*'s base case ("承認済 umbrella 配下で plan-review を
  通った narrowly-scoped per-PR slice は terminal 単位") Slice 1 is that slice, and lands first (this PR, #510).
  ⚠ Until PR #510 Codex R16 this bullet read "Plan-review on a per-PR plan before implementation; lands
  first", as if a separate per-PR memo preceded Slice 1; none did — the reviewed plan was this document.
- **Slice 2 — prose predicates** (I-D/E; §4 #4–#6). Touch set: `plan_memo_roles.py` (licensing,
  connective grammar), `plan-memo-umbrella-check.py::_anchored` (self_id), `plan_memo_tables.py::
  _attributed_to_other` (moves to roles), selftest. **Its own per-PR plan-memo, plan-reviewed before
  implementation** — the prose half has no canonical algorithm (§0) and is reviewed against a different
  reference than §2–§7 here, which are Slice 1's; lands second.

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
