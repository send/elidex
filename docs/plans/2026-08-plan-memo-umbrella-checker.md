# Umbrella plan — `plan-memo-umbrella-check` carved out of #506 into a 2-slice prerequisite program

**Status**: plan-review **converged** 2026-08-22 (IMP 16 → 10 → 3 across three rounds; R3's three were mechanism decisions, applied below; remaining MINs applied). Implementation order: Slice 0 → Slice 1 (this PR) → Slice 2. **Implementation record (Slice 0 `718626e9`, Slice 1 `7931798d`)**: premises of this plan the implementation found false are marked ⚠ inline below; measurements in §6 are the re-run values. Branch `vm-p4-plan-memo-checker` (worktree
`elidex-wt-vmp4checker`, base `origin/main`). Files carried verbatim from #506 @ `190d2adb` **at the
carry commit `5e9439b4`** (`git diff --quiet 5e9439b4 190d2adb -- .claude/tools/` = identical there, not
at HEAD): `.claude/tools/plan-memo-umbrella-check.py` 811 lines, `plan_memo_tables.py` 407,
`plan_memo_umbrella_selftest.py` 396 (`wc -l`, 1,614 total). Those three are a fact about the CARRY
commit and do not move.

⚠ **The per-file inventory of the CURRENT tree used to be enumerated here and has been REMOVED, at
Codex R26's P3.** It went stale three times — every touch-time split adds a module and moves two
counts, and the split commits land between the moment the paragraph is written and the moment it is
pushed, so it was wrong for the very tree it named (it claimed 20 files / 11,427 lines against a tree
holding 22 / 11,981, and named the wrong largest file, which is the figure a reader would use to pick
a split target). A number that must be re-derived before every push, and that has been wrong every
time it was not, is an argument rather than a measurement, so what stands here is the INVARIANT and
the command that decides it:

- **every `.claude/tools/plan*.py` is under the 1000-line touch-time bound** — `wc -l
  .claude/tools/plan*.py | sort -n | tail -5` names the file closest to it, which is the split target;
- the only files over 1000 lines in that directory are the two vendored CommonMark corpora
  (`commonmark-0.31.2-{block,inline}-examples.json`), which are generated data and outside the
  cohesion test;
- no `crates/` change, in any round.
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

⚠ **A SECOND concurrent lane touches the same one-line list, and no obligation named it until PR #510
Axis 5 (2026-09-20)**: the `stale-claim-detector` lane (worktree `elidex-wt-staleclaim`) adds
`claim-provenance-trip-wire.sh` to `scripts/trip-wires.sh`'s `REQUIRED_WIRES`, the list whose entire
purpose is that a wire cannot be added or lost without a line — so `--ours` / `--theirs` on it is the
one resolution neither lane may take. **Whichever lands second APPENDS its line**; both wires then
stand. Measured, because the first guess was that this is a budget problem and it is not: this wire
is ~26 s and `claim-provenance-trip-wire.sh` is ~6 s on the same host, against the `timeout-minutes`
re-derived at ~4x headroom — so the collision is a MERGE hazard on the inventory, not a cost one.
⚠ And the budget block in `ci.yml` is derived against THIS wire alone; that is now stated there, so
the second wire's arrival does not read as drift in the figure.

⚠ **THREE plan-memo checkers are in flight at once, and until PR #510 Axis 5 not one of them named
another.** This one (the umbrella row-kind census + naming-site scan), `claim-gate-plan-check.py`
(worktree `elidex-wt-claimcheck` — quantitative-claim provenance and staleness) and `plan-xcheck.py`
(worktree `elidex-wt-decinline` — the layout memo cross-check); the latter two have reconciled with
each other and neither knows about this one. Each also ships an always-run wire, which is how three
tools become one CI job's budget. **The boundary, stated so the next author does not have to guess**:
this tool answers *"does this memo's row-kind census parse, and is every naming site licensed"*;
`claim-gate` answers *"is a number in a memo still true"*; `plan-xcheck` answers *"do two layout memos
agree"*. ⚠ They are NOT collapsed here, and deliberately: CLAUDE.md's *One issue, one way* demands the
collapse only once "why N" can be WRITTEN, and nobody has written it — three populations (one memo
family / any memo's figures / two named memos) that today share no predicate. The obligation this
raises is the cross-reference, not the merge. ⚠ **And it is discharged in ONE of three directions,
which this paragraph first reported as done** (PR #510 Axis 3 + Axis 5, measured independently):
only `.claude/tools/plan-memo-umbrella-check.py` — this PR's — names the other two.
`claim-gate-plan-check.py` names `plan-xcheck.py` and not this one; `plan-xcheck.py` names neither.
This PR cannot edit those trees, so the reciprocal is **OWED on the two lanes**
(`claim-gate-plan-check` in `elidex-wt-claimcheck`, `plan-xcheck.py` on `layout-decorated-inline`)
and is carried into their memos rather than asserted here. **Trigger (an EVENT)**: whichever of
those two lanes next touches its tool's header. **Re-eval: 2026-12-31.** **Trigger for revisiting the collapse (an EVENT)**: the first predicate two of the three need to
share. Re-eval: 2026-12-31.

## §2 Coupled invariants (edge-dense)

- **I-A Lexical masking, and the RENDERED text** (design re-gate 4) — the ONE text every predicate
  over a block reads is the block **as the document renders it** (`plan_memo_stream.stream`): a
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

| Spec section | Step | Branch | Touch (site name @ `190d2adb`; implemented as `plan_memo_links.py::link_destination` / `link_title` / `inline_pass` (Phase 2) and `plan_memo_blocks.py::reference_definitions` / `fenced_lines` / `split_row` + `delimiter_width` (Phase 1; split from the lexer at the 2026-08-23 re-gate, touch-time 1000-line rule)) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| CommonMark §6.3 Links | inline link | bare destination = nonempty, not starting with `<`, no space / ASCII control, parens balanced or escaped; `<dest>` = no line ending, no *unescaped* `<`/`>`; backslash escapes ASCII punctuation only (§2.4) | `plan_memo_tables.py::_link_destination` | ✗ (`isspace`/`ord>31` ≠ spec classes; `\` skips any char) — Slice 1 | no |
| CommonMark §6.3 Links | inline link | title `"…"` / `'…'` / `(…)` with escapes | `plan_memo_tables.py::_link_title` | ✗ (`(` inside `(…)` title unguarded) — Slice 1 | no |
| CommonMark §6.3 Links | reference link | full / collapsed / shortcut (shortcut = label not followed by `[]` or a link label — ⚠ this plan once said `[a][undefined]` = shortcut `[a]` + literal; §6.3 / Example 571 say the opposite (571 = `[foo][bar][baz]` with `[foo]` defined and `[bar]` not: "`[foo]` is not parsed as a shortcut reference, because it is followed by a link label"; 570 is the full-reference case), `[undefined]` IS a link label so `[a]` is not a shortcut; implemented per spec with a NEGATIVE control); label = 1–999 chars, ≥1 non-blank; match = casefold + strip + collapse; duplicate definitions: first wins | `plan_memo_lexer.py::links` / `_reference_tail` (⚠ pre-split name was `plan_memo_tables.py::links`) | ✗ (collapse missing; collapsed label found by nearest `[`, not matching `[`) — Slice 1 | no |
| CommonMark §4.7 Link reference definitions | definition | label non-blank, no unescaped `[`; optional one line ending before destination; `<dest>`; nothing after destination/title | `plan_memo_blocks.py::reference_definitions` (⚠ pre-split name was `_REF_DEF`) | ✗ (`[ \t]*`, `[^\]]+`, `\S+`) — Slice 1 | no |
| CommonMark §6.1 Code spans | masking | opener/closer = backtick strings of equal length; unmatched strings literal | `plan_memo_tables.py::code_spans` | ✗ (next single backtick closes) — Slice 1 | no |
| CommonMark §4.5 Fenced code blocks (⚠ all lexer touch sites below live in `plan_memo_lexer.py` / `plan_memo_blocks.py`, not `plan_memo_tables.py` — tables.py would have crossed ~800 lines; seam = lexing vs inventory; `dispose` (mask disposition) stays in tables.py because it needs ids) | masking | ≥3 ``` or ~~~, not mixed; ≤3 spaces indent; closer same char, ≥ length, ≤3 spaces indent, only spaces/tabs after; info string of a backtick fence has no backtick; unclosed runs to EOF | (NEW) `plan_memo_tables.py::fenced_spans` | ✗ (absent) — Slice 1 | no |
| GFM §4.10 Tables | recognition | header/delimiter equal width else not a table; delimiter cell = ≥1 hyphen with optional leading/trailing colon; leading/trailing pipe optional; ends at blank line or block start | `plan_memo_tables.py::find_tables` / `is_row` / `is_separator` | ✗ (no width compare; `is_row` requires leading `|`; `[:\- ]*` admits empty / colon-only cells) — Slice 1 | no |
| GFM §4.10 Tables | cell split | unescaped `|` splits (incl. inside backticks, Example 200); `\|` → cell content `|` (backslash consumed); spaces between pipes and content trimmed | `plan_memo_blocks.py::split_row` | ✗ (keeps `\|` with the backslash — then `` `9z \| 7z` `` is masked as code while `9z | 7z` is an id-only mention) — Slice 1 | no |
| GFM §4.10 Tables | body row width | spec: pad/truncate; **local policy**: ≠ header ⇒ exit 2 | `find_tables` (admission, one site) | ✗ — Slice 1 | no |
| **PR #510 rows (clauses the R1–R3 rounds added; ✓ = a named control exists)** | | | | | |
| CommonMark Appendix A "A parsing strategy", Phase 2 "look for link or image" | bracket stack | one left-to-right pass over `[` / `![` openers with an *active* flag; on `]` pop the nearest opener, try inline → full → collapsed → shortcut; a LINK deactivates every `[` opener before it ("links may not contain links", §6.3), an image does not; a failed opener is literal text | `plan_memo_lexer.py::links` | ✓ controls "(link) nested inline links: the INNER link is the link…", "(link) a reference link nested in inline brackets…", "(link) a link wrapping a REFERENCE image `[![alt][img]](child.md)`…", "links() is linear: 30 nested brackets are one inline_pass call" | no |
| CommonMark §6.4 Images | image | `![` opens an image (an unescaped `!` before `[`); not a link; destination never a sibling; alt text prose; tail masked as kind `image`. **R19**: a RESOLVED image's description is plain text (§6.4: the description is rendered as the `alt` attribute's plain string content), so in ONE rule at the point the image closes (`inline_pass`, offset containment: the entries recorded past the image's `[` — the trailing ones, since brackets nest and each list is appended in closing order) every bracket construct inside the description is the description's: a link is DEMOTED to a masked tail in `Lexed.images` (never a memo link — its destination never joins the population — and never prose: the alt text is the link's TEXT), a nested image stays masked, a failed reference names no lost memo (resolved, it would have been demoted). An UNRESOLVED image is literal `![…]` text and the link inside it IS a link. commonmark.js 0.31.2, measured: `![alt [docs](absent.md)](image.png)` → `<img alt="alt docs">`; `![alt [docs](child.md)][missing]` → `![alt <a href="child.md">docs</a>][missing]`; `![a ![b [c](x.md)](i.png)](j.png)` → `alt="a b c"`; `![a ![b [c](x.md)](i.png)][missing]` → `![a <img alt="b c">][missing]`; `[a ![b [c](x.md)](i.png)](y.md)` → `[a <img alt="b c">](y.md)` (the inner link deactivated the outer `[` before the image demoted it — the §6.3 rule of the row above, no second rule). ⚠ Until R19 the inner link joined the population AS IT CLOSED, so the reviewer's `absent.md` inside a resolved image's description was a false unavailable-memo miss, rc 2 | `plan_memo_links.py::_is_image` / `inline_pass` (the `is_img` arm) / `Lexed.images`; `plan_memo_stream.py::dispose` (`image`) | ✓ controls "(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)`…", "(image) `![alt][img]` with a definition is consumed whole…", the R19 "(image) …" family (the reviewer's input rc 0; `child.md` not walked and the image's `.md` destination not probed; the unresolved image KEEPS its link, `child.md` walked; a nested image; the outer failing while the inner resolves; a link outside the inner description; `[a ![b [c](child.md)](i.png)](parent.md)` links nothing; a failed reference inside the description rc 0; the demoted tail masked — `9z` in its destination is no site); mutants R19 #1 (the immediate add re-injected; a plain drop instead of the masked demotion; the failed-reference rule dropped; demotion re-injected on the FAILED image too) | no |
| CommonMark §4.7 Link reference definitions | next-line title | the title may follow on the line after the destination; an invalid next line leaves the definition ending at the destination (one attempt, same-line and next-line) | `plan_memo_blocks.py::reference_definitions` | ✓ controls "(def) a next-line title is part of the definition…", "(def) a next-line title holding `[x](missing.md)`…", "(def) a next line that is NOT a valid title is prose…" | no |
| CommonMark §4.7 | orphan detection | exactly the spec-grounded class: a line that parses as a VALID definition but cannot take effect because it is not at a block start ("a link reference definition cannot interrupt a paragraph"); a label-and-colon line that is NOT a valid definition is prose (commonmark.js: `[C1]: ECMA-262 §1 says so` is a paragraph; `[sib]: child.md "title\n\nmore"` is a paragraph) and a shortcut naming it is exempt; linear — the runs are joined once, one parse per line | `plan_memo_memo.py::Memo._parse` (`Memo.orphans`; ⚠ pre-split name `plan_memo_tables.py::Memo._blocks`), `plan_memo_blocks.py::definition_block` | ✓ controls "(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan…", "Phase-1 orphan detection is linear: <= 4 link_label calls per line" | no |
| CommonMark §2.4 Backslash escapes | row split parity | only an ODD backslash run escapes a `|` (`a\\|b` is two cells); the trailing-pipe check reads the same parity (`_escaped`, one helper) | `plan_memo_blocks.py::split_row` | ✓ controls "(row) `a\\|b` holds an UNESCAPED pipe…", "(row) `a\|b` is one cell…", "(row) a trailing `\\|`…" | no |
| §5 (local policy over the disposition exception) | id-only code spans | an id-only run is tokenised by the declared-id GRAMMAR longest-first (a `#11-` slug is atomic; `` `#11-zz-alpha / 9z` `` spells two ids), with separators between tokens | `plan_memo_stream.py::id_only` (`_ID_RUN_TOKEN`) | ✓ control "(span) a `#11-` slug is ATOMIC in an id-only run…" | no |
| (no spec clause — a tokenisation fact of these documents) | bare `.md` file name | read by PATH SYNTAX: the maximal run — **possibly empty (R20)** — of non-whitespace characters ending in the lexer's `FILE_SUFFIX` (inline delimiters `[` `]` `<` `>` `` ` `` `\|` excluded so link text and code spans are not swallowed; parentheses only as a balanced pair), bounded by spaces / tabs / line ends or the cell edge; no trailing-punctuation rule is needed because the token ENDS at the suffix (the GFM §6.9 autolink rule is moot). ONE notion of "is a file name", defined by the lexer (`FILE_SUFFIX`) and consumed by `sibling_path` stage (d): the stem is unconstrained on both sides, so `.md` alone is a file name in prose and a sibling in a destination; ⚠ until R20 the token arm required a one-character stem while `sibling_path` did not, and beside a declared id `md` the prose `Read .md for details` reported `md` | `plan_memo_lexer.py::_TOKEN` / `FILE_SUFFIX`, `plan_memo_memo.py::Memo.sibling_path` (d) | ✓ controls "(file) `9z+notes.md`…", "(file) `9z@notes.md`…", "(file) `(9z).md`…", "(file) a link's visible text is not swallowed…", "(file) `.md` alone is a file name…", "(file) `notes.md` beside a declared no-owner id `md`…", "(file) bare `md` (no suffix)… IS a site", "(link) `[x](.md)` links the sibling file named `.md`…" | no |
| CommonMark §2.1 / §4.9 / GFM §4.10 | ASCII classes at every boundary | a blank line = spaces or tabs only (§2.1 — the DEFINING sentence, which `plan_memo_blocks.py` and §5's own row both place there; §4.9 is where its EFFECT on leaf blocks is spelled); edge pipes and cell trimming use the same space/tab class; the far side of a `.` after a bare id is the ASCII id class (`9z.次の工程` reports `9z`); `is_empty`'s `isalnum` is deliberately Unicode (a letter in any script fills a cell) | `plan_memo_blocks.py::is_blank` / `split_row`, `plan_memo_ids.py::_glued` (⚠ pre-R14 name `plan-memo-umbrella-check.py::_glued`), `plan_memo_tables.py::is_empty` | ✓ controls "(span) an NBSP-only line is NOT blank…", "(table) a row opening with an NBSP…", "(bare) `9z.次の工程`…", "(bare) `9z.é`…" | no |
| §5 (local policy: the id grammar of these documents) — **R14** | ONE id-token grammar for every reader | the three kinds (short / `#11-` slug / `[C19]` citation, either case), the decoration, and the ONE two-sided boundary — a decorated side is bounded by its decoration; an undecorated side by the complement of the kind's continuation class (short: ASCII alnum + the dotted-number rule, a hyphen BOUNDS; slug: alnum + `_` + `-` on BOTH sides; citation: its brackets) — are spelled once in `plan_memo_ids.py` and CONSUMED by the bare and anchored naming scans, the raw-line seed, the id cell, the kept-slug exception inside a code span, the lexer's citation mask and the reference walk's citation exemption (R8/R9/R14: the boundary had been spelled six ways, disagreeing on a hyphen on a raw line, a slug's right side, and a citation's case); the orphan-definition exemption is by the orphan's exact bracket `(line, column)`, never by line | `plan_memo_ids.py::tokens` / `is_cite_label` / `kind_of`; readers `plan-memo-umbrella-check.py::_bare` / `_anchored` / `lex_unsupported_seed`, `plan_memo_tables.py::bare_id` / `code_mask`, `plan_memo_lexer.py::_TOKEN`, `plan_memo_memo.py::Memo.unresolved_references` (`Memo.orphans`) | ✓ controls "(lex-seed) a raw HTML line `9z-owner`…" / "…`owner-9z`…", "(slug) `#11-zz-alphaZZ`…", "(slug) `` `tool #11-zz-alpha_extra` ``…", "(cite) `[c1]` beside a declared no-owner id `c1`…", "(cite) adjacent lowercase citations `[c1][c2]`…", "an orphan definition exempts its OWN bracket only…", and the PROPERTY control "the id character classes are spelled once, in plan_memo_ids.py (a source-text sweep)" — a sweep over the other modules' string constants for the grammar's spellings, which by construction cannot see a class spelled in another order (`[A-Za-z0-9-]` is the HTML tag-name grammar, `[a-zA-Z0-9+.-]` the URL scheme grammar; neither is an id class), a class built by concatenation, a hand-written character test, `\b` under `re.ASCII`, or a comment — **nor KIND coverage (R20)**: a composer built on `SHORT_ID` alone spells nothing twice; `ROW_KINDS` / `ROW_ID` (slug \| short — a citation keys a citation-table row but declares no kind and is no row in this sense) is the grammar's ONE "a row id here" alternation, composed by `ROW_NOUN_ID` → `_APPOSITIVE`, `OWNS_TWO` and the anchored reading (`_anchored`, `t.kind in ROW_KINDS`), and the PROPERTY control "every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the spelling sweep)" probes each composer with a sample of every kind in the grammar's tuple (a kind without a sample is red); ⚠ until R20 `_APPOSITIVE` composed `decorated_id(SHORT_ID)`, so `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributed nothing: the pointer row was an umbrella, no `UMBRELLA-MARK` finding, exit 0 | no |
| CommonMark §6.3 | one label grammar | the text of a collapsed / shortcut reference is a label iff `link_label` reads it from the opener (no second walker); the full form carries `raw` out of `_reference_tail` | `plan_memo_links.py::_reference_tail` / `link_label` | ✓ control "(link) bracket text holding unescaped brackets is not a label (§6.3)…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 2 | one inline pass | code spans (§6.1) and brackets are recognised together, left to right; a backtick string opens a span as met and the scan jumps past it; an inline-link tail is parsed by lookahead on the RAW text (a backtick inside a destination is consumed by the link; one before the `]` opens a span that swallows it); no code pre-mask | `plan_memo_lexer.py::inline_pass` (`code_spans` / `links` are views over it) | ✓ controls "(span) a backtick inside a link DESTINATION is consumed by the link…", "(span) a backtick BEFORE the `]` opens a code span that swallows it…", "(link) a link inside a code span is not a link (A x B)" | no |
| CommonMark §6.4 Images | unresolved reference image | `![alt][missing]` is literal image syntax, never an unresolved memo reference (the opener's image flag travels with the unresolved record); the failed reference is recorded ONCE — its label bracket is re-scanned (the scan resumes after the literal `]`; §6.3 Example 571: `[missing][baz]` may be a link, so the tail is NOT consumed) but a shortcut it closes as is the same site, not a second record, so an orphan `[missing]: image.md` later in the paragraph does not turn `[missing]` into a memo miss (R11) | `inline_pass` (`relabel`) → `Memo.unresolved_references` | ✓ controls "(image) an undefined reference image `![diagram][missing-image]`…", "(image) the reviewer's input `![alt][missing]` + an orphan…", "(image) `![alt][missing][Slice 9z]` with `[Slice 9z]` defined…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1 → Phase 2 | block structure before inline structure | Phase 1 over RAW lines (`Memo`): fences, reference definitions as blocks of their own recognised only at a block start and parsed over the REST OF THEIR RUN (the lines up to the next `block_end`, joined once — §6.3 label up to 999 chars and may span lines, `link_title`; a valid definition inside a paragraph is an orphan, `Memo.orphans`), GFM tables ending at the same `block_end` (a reference definition right after a table is a ROW of it — GFM Example 202 — not a definition), paragraphs; Phase 2 (`inline_pass`) over each paragraph's / cell's content only, with `defs` from Phase 1 | `plan_memo_memo.py::Memo._parse` (its `definition_at`); `plan_memo_blocks.py::definition_block` / `run_end` | ✓ controls "(def) a definition is read from RAW lines at a block start…", "(table) a reference definition right after a schema table is a ROW of it (GFM Example 202…) — one cell under a 4-cell header, width miss rc 2" (⚠ until R20 this cell cited the control as proving the definition "ENDS the table", the opposite of the mechanism column and of the control's own expectation), "(def) a definition cannot interrupt a paragraph…", "Phase-1 orphan detection is linear…" | no |
| CommonMark "Appendix: A parsing strategy", Phase 1; §4.4; §4.6; §5.1; GFM §4.10 | block start = block state | "is this line at a block start" is the driver's state (a run open or not), the ONE context bit `block_end` / `raw_opener` take; a type-7 HTML opener or an indented line after a table's rows, a one-line block or a setext heading opens a raw extent; after a run line (paragraph text, a definition, a list-item line, a quote's lazy line) it is text — no caller looks back at the previous raw line (R11); inside a block quote the same state, re-entered (R13) | `plan_memo_blocks.py::raw_opener` / `block_end` / `run_end`, `plan_memo_memo.py::Memo._parse` / `_quote`, `plan_memo_tables.py::admit_table` | ✓ controls "(table) a type-7 HTML opener right after a schema table ENDS it…", "(html) `# h\n<span>\n9z owns it`…", "(html) `Heading\n===\n<span>\n9z owns it`…", "(html) `[sib]: slice-9z-sib.md\n<span>\n9z owns it`…", "(html) `- item\n<span>\n9z owns it`…" | no |
| WHATWG URL §4.4 "URL parsing" — the basic URL parser's *scheme start state* (`#scheme-start-state`) and *scheme state* (`#scheme-state`), then §1.3 "Percent-encoded bytes" *percent-decode* on a string (`#string-percent-decode`) — + **local policy** (CommonMark §6.3 / GFM say nothing about siblings on disk) | ONE destination → sibling resolver | `Memo.sibling_path`, stages in spec order: (a) scheme on the RAW path — `#scheme-start-state` step 1 ("If c is an ASCII alpha, append c, lowercased, to buffer, and set state to scheme state") and `#scheme-state` step 1 ("If c is an ASCII alphanumeric, U+002B (+), U+002D (-), or U+002E (.), append c, lowercased, to buffer") / step 2 ("Otherwise, if c is U+003A (:)" → the scheme) read the input's code points AS WRITTEN; `%` is in neither class, so at `%` the parser leaves for the *no scheme state* and `notes%3Achild.md` has no scheme — it is the file `notes:child.md`; percent-decoding is no step of the parser (the *path state*, `#path-state`, percent-ENCODES and keeps `%xx`), (b) percent-decode — `#string-percent-decode` ("Let bytes be the UTF-8 encoding of input. Return the percent-decoding of bytes"; `urllib.parse.unquote`), a consumer's operation on the parsed path (`slice%20sib.md` = `slice sib.md`) — ⚠ until R17 this row and the docstring cited "WHATWG URL" bare, (c) the DECODED name must be RELATIVE on every platform, and hold no C0 control / DEL (`child%00.md` would make `resolve()` raise) — **R19**: ONE platform-independent reading, Windows path syntax (`pathlib.PureWindowsPath`, the superset: `/` and `\` both separate, a drive letter / UNC prefix / root ANCHORS), which is the URL standard's own reading of a special-scheme path (`file` is a special scheme): `#path-state` step 1 ends a segment at `/` or, "url is special and c is U+005C (\)", at a backslash (invalid-reverse-solidus validation error), and step 1.4.1's drive-letter rule is, in the spec's words, "a (platform-independent) Windows drive letter quirk" — so `PureWindowsPath(name).anchor` must be empty: `/x`, `//host/x` (a site URL joined to the memo's directory would probe the host's filesystem root), `\x`, `C:\temp\x`, `\\server\share\x` and the drive-relative `C:x` (raw `C:x.md` is already scheme `c` at (a); percent-encoded `C%3Ax.md` decodes to a drive anchor here, as does the one-letter `n%3Ax.md`, where the multi-letter `notes%3Ax.md` of (a) is a file name) are rejected; ⚠ until R19 (c) rejected a leading `/` only, so `C%3A%5Ctemp%5Cchild.md` and `%5Cchild.md` passed and on Windows `parent / name` discarded the memo's directory, (d) `.md`, (e) the name's PARTS joined beside the memo under the same syntax — `\` is a separator everywhere, never a POSIX name character: `sub%5Cchild.md` is the sibling `sub/child.md` on every platform (R19) — then `_resolve` = `resolve()` with `OSError` or (Python 3.9–3.12 symlink loop) `RuntimeError` read as an UNAVAILABLE sibling, reported by the population's one I/O chokepoint (`Memo()` under `OSError | UnicodeDecodeError` — I/O ONLY, `read_text(encoding="utf-8")`; ⚠ until R16 `RuntimeError` too, which read a parser `RecursionError` — a `RuntimeError` — as an unavailable memo, rc 2, no census; a parser exception is a crash, crash = FAIL) as the exit-2 unavailable-memo miss; `linked_files` dedups with a set | `plan_memo_memo.py::Memo.sibling_path` / `_resolve` / `linked_files`, `Population.__init__` | ✓ controls "(link) `notes%3Achild.md` has no scheme…", "(link) a percent-encoded destination…", "(rc) a percent-encoded ABSOLUTE destination…", "a decoded destination with a C0 control character is rejected…", "an OSError from resolve() is the unavailable-sibling schema miss…" (OSError and RuntimeError injected), "an undecodable sibling is the unavailable-linked-memo schema miss…", "linked_files is linear: <= N Path.__eq__ calls over N distinct siblings…", "a RuntimeError raised while PARSING a memo is a crash out of check(), never the unavailable-memo miss" (R16; mutant re-injects the broad except), the R19 "(rc) …" family (`C%3A%5Ctemp%5Cchild.md`, `%5Cchild.md`, the percent-encoded UNC, the raw `\\server\share\x.md` — §2.4 decodes it to the `\`-rooted `\server\share\x.md`, commonmark.js's href `%5Cserver%5Cshare%5Cx.md` — drive-relative `C:child.md` raw and encoded, the one-letter `n%3Achild.md`) and "(link) `sub%5Cchild.md`…" (walks `sub/child.md`); mutants R19 #3 (the `/`-only test re-injected; the POSIX reading `parent / name` re-injected) | no |
| CommonMark §2.5 Entity and numeric character references + §6.3 (destination) — **R16** | ONE destination normalisation | `normalize_destination`: ONE left-to-right pass over the destination's raw text at the ONE site both forms and both grammars return through (`link_destination`, bare and `<…>`; the inline link and the §4.7 definition both call it) — a §2.4 escape yields its character; a §2.5 reference — `&` + an HTML5 entity name + `;` (`html.entities.html5`, looked up WITH the `;`, so HTML's legacy semicolon-less `&copy` is literal, Example 29, and `&MadeUpEntity;` is literal, Example 30), `&#` + 1–7 digits + `;`, `&#x` / `&#X` + 1–6 hex digits + `;` — yields its character, with U+0000, code points above U+10FFFF and surrogates → U+FFFD; the two grammars meet at a character exactly once (`\&#46;` is a literal `&#46;`; a decoded `&`, `&#x26;#46;`, is never re-read as a reference). NOT decoded: a label (§6.3 matching is on the raw label — `[foo&auml;]` ≠ `[fooä]`, commonmark.js measured), a title (`link_title` reads shape only, never text), a code span (`inline_pass` jumps past it). ⚠ Until R16 backslash-only: `[child](child&#46;md)` / `[sib]: child&#46;md` reached `sibling_path` as the literal, no `.md` suffix, the sibling silently outside the population, rc 0 | `plan_memo_links.py::normalize_destination` / `_CHAR_REF` / `_reference` / `_codepoint`, `link_destination` (both returns) | ✓ controls "(link) `[child](child&#46;md)`…", "(link) `[child](<child&#46;md>)`…", "(link) `[child](child&period;md)`…", "(link) `[child](child&#x2E;md)`…", "(def) `[sib]: child&#46;md`…", "(link) `[x](slice&#37;20sib.md)`…" (§2.5 then `sibling_path` stage b, spec order), "(link) `[x](child&#0;.md)`…", "(link) `[x](child&#x110000;.md)`…", "(link) `[x](child&copy.md)`…", "(link) `[x](child&#46md)`…", "(link) `[x](child\&#46;md)`…", "(link) `[x](child&#x26;#46;md)`…", "(link) `[x](child&MadeUpEntity;md)`…", "(span) `` `[c](child&#46;md)` ``…", "(label) `[foo&auml;]: child.md` then `[fooä]`…"; mutants R16 #2 (backslash-only re-injected; `html.unescape` re-injected; the U+0000 rule dropped; `;` optional; decoding re-injected in `normalize_label`) | no |
| CommonMark §6.6 Raw HTML — **R17** | ONE tag grammar, a span of the one inline pass | `_HTML_TAG` in `inline_pass`, tried at every `<` the scan meets (left to right with backtick strings and brackets, commonmark.js's order: `<a href="`">b` c` is a tag then a literal backtick, `` `x <span title="`">b `` a code span then text): an **open tag** (`<` + a tag name — an ASCII letter then ASCII letters / digits / `-` — + attributes + optional spaces / tabs / one line ending + optional `/` + `>`; an **attribute** = at least one space / tab / line ending (≤1 line ending), an attribute name — ASCII letter / `_` / `:` then letters / digits / `_` / `.` / `:` / `-` — and an optional value spec `=` with optional whitespace around it and a value: **unquoted** = a nonempty string without spaces, tabs, line endings, `"`, `'`, `=`, `<`, `>`, `` ` `` — so `<span title=[x](y.md)>` IS a tag (⚠ the R17 brief presumed a negative), **single-quoted**, **double-quoted**), a **closing tag** (`</` + name + optional whitespace + `>`), an **HTML comment** (0.31's `<!-->`, `<!--->`, or `<!--` + a string not containing `-->` + `-->`: `<!-- a -- b -->` is one, 0.30 forbade it), a **processing instruction** (`<?` … `?>`), a **declaration** (`<!` + an ASCII letter + no `>` + `>`, either case) or a **CDATA section** (`<![CDATA[` … `]]>`, exact case). The span is RAW + SEEDED, the §3.0 disposition of a §4.6 block line applied to the same kind of text: never inline-parsed (a bracket inside an attribute value or a comment is no link delimiter — `<span title="[child](absent.md)">` once made a false unavailable-memo miss, rc 2), never a naming site (an id inside an attribute is masked, kind `html`), recorded in `Memo.raw` with the `inline` reading and seeded under the one raw-line rule when it holds a `\|` or a declared id; a `<` the grammar refuses is text and the brackets after it are read (`<3 [x](y.md)`, `<a href="x" [x](y.md)>`, `< span>`, `</ span>`, `<a b="c"d>` (this suite's own fixture, from `plan_memo_selftest_cases_inline.py`, not a spec example — the spec's optional-whitespace-before-attribute case is Example 622, `` <a href='bar'title=title> ``, read off the vendored inline corpus rather than recalled), `<! …>`, `<![cdata[`, `\<span …>`). §4.6 start condition 7 ("a complete open tag … or a complete closing tag") reads the same `OPEN_TAG` / `CLOSING_TAG` — ⚠ until R17 the tag grammar was spelled a second time in `plan_memo_blocks.py` | `plan_memo_html.py::_HTML_TAG` / `OPEN_TAG` / `CLOSING_TAG` / `inline_pass` (`html`), `Lexed.html`, `plan_memo_stream.py::dispose` (`html`), `plan_memo_memo.py::Memo._inline_raw` (seed), `plan_memo_blocks.py::_HTML_BLOCK` (t7) | ✓ controls "(html) the R17 reviewer's shape `<span title=\"[child](absent.md)\">text</span>`…" and the "(html) …" R17 family (each arm positive, each negative rc 2 or a reported site, the two left-to-right probes, the cell, `[<span>x</span>](…)`), "(lex-seed) `<span title=\"Slice 9z owns it\">`…" (+ the `inline` reading); **falsifier = the section's own list, `Raw HTML` Examples 613–632, vendored in `commonmark-0.31.2-inline-examples.json`** ⚠ (this cited `commonmark-0.31.2-inline-html-examples.json` until R38's design re-gate — that file was added at R17 and **deleted at R21** `dbb2644c` when the inline corpora were merged, so the locator had been dead for seventeen rounds while the substance stayed true; the 20 examples are in the merged file. ⚠ `symbol_attribution_control` cannot reach this class: it reads `module.symbol` attributions, never a vendored DATA-file name) — the runner's inline conformance control: the spans Phase 2 masks are exactly the text the html emits verbatim (each span verbatim in order in its `<p>` body, and the body's unescaped `<` count — minus two per code span — equals the spans'); mutants R17 #2 (the `<` arm dropped; quoted values dropped; the comment arm dropped; 0.30's comment exclusion re-injected; the PI / CDATA arms dropped; `<!` + anything; optional whitespace before an attribute; the seed record dropped; the `html` disposition dropped) | no |
| CommonMark §6.2 Emphasis and strong emphasis (+ GFM 0.29 Strikethrough) — **design re-gate 4** | which delimiter runs PAIR | a delimiter run is a maximal run of `*` / `_` (CommonMark) or one-or-two `~` (GFM: "a matching pair of one or two tildes", so three or more is literal — `a~~~b~~~c` renders verbatim, measured on GitHub's pipeline); LEFT-FLANKING = not followed by Unicode whitespace and either not followed by Unicode punctuation or preceded by whitespace or punctuation (right-flanking mirrors it; the classes are Unicode by the spec's own words — P* or S* for punctuation, 0.31's definition — never ASCII, since the surrounding text of these memos is Japanese as often as not); `*` opens iff left-flanking and closes iff right-flanking, `_` adds §6.2 rules 5–6 (`snake_case` stays intact), `~` reads the `*` conditions and pairs EQUAL lengths only; then the Appendix's `process_emphasis` — each closer matched to the nearest live opener at or above the bracket's bottom, the RULE OF THREE on the ORIGINAL run lengths, `openers_bottom` so a failed search is never repeated, the delimiters between a matched pair removed — run where the Appendix runs it: when a link or an image closes (over the delimiters inside it) and once at the block's end, so emphasis never crosses a link's text boundary. A pair inside a RESOLVED image's description is DEMOTED (§6.4: plain string content, no `<em>` — the R19 rule, one more construct). What the caller wants is not a tree but the CHARACTER SPANS the delimiters occupy, since those are what the stream drops | `plan_memo_emphasis.py::run_at` / `_matches` / `process`, pushed by `plan_memo_lexer.py::inline_pass`, disposed by `plan_memo_stream.py::dispose` | ✓ | no |
| (no spec clause — this checker's disposition, over CommonMark's rendering) — **design re-gate 4** | ONE stream: the block as the document renders it | each construct contributes text, nothing, or its character (`RENDERS_TEXT` + the §2.4 / §2.5 substitution, §3.0b's `Renders` column); a DROP beats a BLANK where they overlap (the fact of the spec beats this checker's policy — a link tail holding a `.md` file token settles it); the two id-decoration exceptions (`id_only` over a code span and over a `**` pair); the offset map back to raw coordinates; and the residue where a unit straddles a blanked span, reported as `[LEX-SPLIT?]` and, in a declaring field, as a schema miss | `plan_memo_stream.py::stream` / `Stream` / `dispose` / `split_units`, `plan_memo_population.py::Population._kind_residue`, `plan-memo-umbrella-check.py::lex_split_seed` / `Block.at_raw` | ✓ | no |

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
stream is the block AS THE DOCUMENT RENDERS IT (`plan_memo_stream.stream`), so each construct
contributes to it either **text** (its own characters, or — for a MASKED one — the same count of
blanks, since the checker refuses to read them but a reader does not read across them either),
**nothing** (the span is dropped and the text on both sides of it is ONE run), or **its character**
(a substitution). The column is not commentary: `RENDERS_TEXT` in `plan_memo_stream.py` is the same
table, read by the one stream builder, and a mutant per row turns it red. Before that re-gate the
stream was the source text with the masks blanked, which read `Slice 9**z**` — a document that
renders `Slice 9z` — as the row `9`, and `**UMBRELLA, not a *terminal* unit.**` as no declaration
at all: a fabricated site on one row, a lost site on another, and a row silently out of the umbrella
census at rc 0.

| § | Inline construct | Disposition | Renders | Phase-2 site | Control / corpus |
|---|---|---|---|---|---|
| §2.4 | Backslash escapes | LEXED — one parity helper; an ODD run escapes, and the same helper answers the row splitter (`\|`) and the inline pass | **its character** — the escape and the §2.5 reference are the two spellings of one substitution, at the one site the stream builds; the backslash renders nothing. EXCEPT a `*` or a `` ` ``, which stands as written: the stream carries exactly one kind of markup (a kept id decoration), and substituting `\*\*C\*\*` would spell a bold `**C**` the document does not have (measured — the umbrella memo quotes a `grep` pattern in that shape) | `plan_memo_links.py::_is_escape` / `inline_pass`, `plan_memo_stream.py::stream` | spec examples (13); "(row) `a\\|b` holds an UNESCAPED pipe"; "(kind) …§2.4 an escaped comma DECLARES the umbrella…"; "(render) `\*\*C\*\*`…" |
| §2.5 | Entity and numeric character references | LEXED **everywhere** (design re-gate 4) — in a link DESTINATION by `normalize_destination`, because that text must equal a file name (R16), and in PROSE by the same grammar in the one inline pass | **its character**, under the §2.4 row's one rule and its one exception | `plan_memo_links.py::normalize_destination` / `_CHAR_REF` in `inline_pass` | spec examples (17); "(§2.5) a character reference in PROSE renders its character…" (`Slice &#57;z` IS a site: 1, where the row above measured 0 — **the re-measured cost of the old reading was never `0 sites`, it was a lost site**); "(kind) …§2.5 a character reference for the comma…"; "(render) `&#42;&#42;C&#42;&#42;`…" |
| §6.1 | Code spans | LEXED → MASKED (backtick strings of equal length). Disposition exception: an id-only span is the document SPELLING an id, not code (`plan_memo_tables.py`). **Inside a resolved §6.4 description** (R42-1): the span contributes its CONTENT and neither delimiter — the backtick runs are marks — and the id-only test runs FIRST, so the decoration exception survives into alt text exactly as the `**` one does | **text** (blanked) — the one construct that renders characters this checker refuses to read (I-A), so it bounds what it sits between and a unit read ACROSS it is the `[LEX-SPLIT?]` residue, not a decision | `inline_pass` / `_code_closer` | spec examples (22); the code-span family; "(render) `Slice W`z` owns it`…" + its seed |
| §6.2 | Emphasis and strong emphasis (+ GFM 0.29 strikethrough, the same delimiter machinery) | LEXED (design re-gate 4) — the spec's delimiter runs, flanking rules, rule of three and `process_emphasis`; a `~` run of one or two is the GFM extension's, three or more is literal. Disposition exception, the code span's own: a `**` pair whose content is only declared ids is the document DECORATING an id, so its delimiters STAND and bound it (`**9z**7z` is `9z` then `7z`, never the token `9z7z`) | **nothing** for a matched pair; an UNMATCHED run is literal **text** and bounds what it sits between | `plan_memo_emphasis.py` (`run_at` / `process`), pushed by `inline_pass`, disposed in `plan_memo_stream.py::dispose` | spec examples (132, `Emphasis and strong emphasis` 350-481); the "(render)" family (the LOST-SITE and FABRICATED-SITE probes over `W**z**` / `W*z*` / `W~~z~~`, the literal `W*z`, the intraword `W_z_`, `W~~~z~~~`, `**9z**7z`, `*9z*7z`); 8 mutants. **The old row's "Cost: none measured" was false**: the cost was a fabricated site on one row and a lost site on another, and, through the same split, a lost kind |
| §6.3 | Links | LEXED — the Appendix bracket stack; inline / full / collapsed / shortcut; a link deactivates every earlier `[` | **nothing** for the `[` and the tail (`](dest)` prints no character); the link TEXT is the document's text there and stays prose (B×D) | `inline_pass` | spec examples (90); the link family; "(render) §6.3 a link's brackets `W[z](…)`…" |
| §6.4 | Images | LEXED — not a link; destination never a sibling; a RESOLVED description is plain text, so a link inside it — and, since design re-gate 4, an emphasis pair inside it — is demoted (R19: no tag of its own) | **text** (blanked) for the tail: an image puts a picture in the flow, not the letters of its alt text, so its two sides are not one word — while the description is scanned as prose, the stated deviation | `inline_pass` (the `is_img` arm) | spec examples (22); the R19 image family; the demotion is what makes Examples 573 / 576 / 577 / 585 / 589 align (`alt="foo bar"` emits no `<em>`) |
| §6.5 | Autolinks | LEXED → MASKED whole (R21) — ONE token tried at a `<` **before** the tag grammar (the spec's order); its contents are not inline syntax, so a bracket inside it opens nothing and an id inside it is no site. **Inside a resolved §6.4 description** (R42-5a): §6.5 makes the URI the link's TEXT, so the description holds the URI and the angle brackets are marks — the row the image-close branch never demoted, which reported the id inside an autolink NOWHERE | **text** (blanked) — its text IS its URL, which a reader reads | `inline_pass` / `_AUTOLINK` | spec examples (19); the R21 autolink family (12 controls, 5 mutants) |
| §6.6 | Raw HTML | LEXED → **DROPPED** whole in ordinary prose (R17 masked it; design re-gate 4 made the mask a drop) — one tag grammar (open / closing tag, comment, PI, declaration, CDATA) whose tag bodies are also §4.6 condition 7's. Seeded (`[LEX-UNSUPPORTED?]`) when it holds a `\|` or a declared id — **and NOT seeded when demoted**, the rule §6.5 already had | **nothing in prose; its own SOURCE TEXT inside a resolved image description** — a tag or a comment is markup there, not text: `W<b>z</b>` and `W<!-- c -->z` render `Wz`, and a `Deps` cell holding only a comment is EMPTY; but §6.4 reduces a description to the plain string content of its inline children and an `html_inline` node's plain string content is its own source, so `![UMBRELLA, not a <span>terminal unit](i.png)` has the alt text `UMBRELLA, not a <span>terminal unit` (cmark 0.31.2 ESCAPES the brackets into the attribute — they are content) and an id in an attribute there IS a naming site. ⚠ This row is the standing proof that the disposition is a function of (kind, CONTEXT) while `RENDERS_TEXT` is a function of kind alone | `inline_pass` / `_HTML_TAG` | spec examples (20); the R17 raw-HTML family; "(render) §6.6 …`W<b>z</b>` / `W<!-- c -->z`…"; "(cell) a `Deps` cell holding only an HTML comment is EMPTY…" |
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
  ⚠ **THE ROUND-BY-ROUND LEDGER LIVES IN ITS OWN DOCUMENT**:
  [`2026-08-plan-memo-umbrella-checker-rounds.md`](2026-08-plan-memo-umbrella-checker-rounds.md).
  Everything above this line is the ACCEPTANCE CONTRACT — what Slice 1 must be true for — and it is
  stable. Everything in that file is the chronological record of getting there: every Codex round,
  every design re-gate, and the handoff each session starts from. ⚠ **The handoff block moved with
  it**, so a session resuming this work opens THAT file, not this one.
  ⚠ **Why they are separate** (PR #510 R47-3, reported by review and measured): this document had
  reached 2,976 lines with §6 alone spanning 1,790 of them — 60% of the memo — and the two halves
  have opposite lifetimes. The contract is edited when the DESIGN changes; the ledger is appended
  to every round, and this session alone appended to it five times. CLAUDE.md's touch-time rule
  asks for the split at the touch, and the seam is a real cohesion seam rather than a line count:
  a stable contract and an append-only history, which is the same review-round seam the self-test
  registries are already split on four times (§7 Slice 0).
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
  #510 review-round mutants and appends to the same `MUTANTS`; the runner imports both. ⚠ Touch-time split on 2026-09-21 (`plan_memo_selftest_cases_r26.py` went 773 → 924 under the R42-8 / R42-10 controls of that session): the R42 family is carved to `plan_memo_selftest_cases_r42.py` at the REVIEW-ROUND seam this suite splits on four times already, 624 + 328. ⚠ Taken at TOUCH TIME and not at the 1,000-line bound (`memory/feedback_touch-time-split-means-while-writing.md`), and taken as its own commit AFTER the feature commit rather than before it — the honest record, since the seam only became visible once the controls were written. The seam was MEASURED: an AST pass reports the R42 group using exactly TWO names from the rest of the module (the fixture builders `_idcell` / `_kindcell`, which lose their underscore in the same commit because a name that crosses a module boundary is not module-private) and the rest using none of the R42 group's. Control NAME SET identical across the split — **all 693**, `cmp` clean, taken from `plan_memo_selftest_controls.registry()` in two `git clone --local` checkouts of the parent and the split. ⚠ **The first statement of this was a universal over an incomplete extraction**: it read "691 lines of names", because the names were scraped from the RUN's output with a `^  (ok|FAIL)` pattern and the two KNOWN-MISS controls print `RED`, so exactly those two were never compared while the sentence claimed the SET. Re-measured from the registry, which is the population itself rather than a rendering of it, and the claim holds — but it held unverified for two of its members. ⚠ An earlier attempt was also contaminated by the untracked new module surviving a plain `git stash` (it needs `-u`), 693 / 392 0-0, census `--worklist` byte-identical, `MODULES` map back to 34 / 34. ⚠ Touch-time split before design re-gate 3 (`plan_memo_tables.py` had reached 879
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
  ⚠ Touch-time split at the START of the next session (`plan_memo_selftest_properties.py` had
  reached **1,110** lines, from 738 at `bc7cb013` — this PR added 372 lines of property controls
  across one session and took no split, which is the CRIT Axis 5 reported against the header's own
  1000-line invariant; it is discharged HERE, before any other item):
  `plan_memo_selftest_records.py` takes every control whose subject is a SENTENCE somebody wrote
  about this module set, held against what the set IS — the entry point's MODULES map in both
  directions (`_MAP_START` / `_MAP_NAME` / `_mapped_modules` / `_map_population` /
  `module_map_completeness_control` / `module_map_existence_control`), the attribution sweep
  (`_ATTRIB_SPELLINGS` / `_DATED_LOCATOR` / `_defining_module` / `_prose_of` /
  `_attribution_corpus` / `symbol_attribution_control`) and the import seams (`_WORK` /
  `_IMPORT_SEAMS` / `import_seam_control`). 1,110 → **751 + 420**.
  ⚠ **THE SEAM THE PREVIOUS SESSION'S HANDOFF NAMED WAS REFINED, and the refinement is the whole
  point of judging cohesion rather than counting lines.** The handoff said "sweeps the tree for
  CROSS-FILE CONSISTENCY" vs "sweeps the checker AS WRITTEN", and named
  `symbol_attribution_control` / `import_seam_control` / `dash_spelling_sweep_control` as the group.
  That does not partition: `id_spelling_sweep_control` is a cross-file consistency sweep too and
  stays, while `dash_spelling_sweep_control`'s own docstring calls itself "the
  `id_spelling_sweep_control` shape applied to the other character class" — so the handoff's line
  would have split a pair the SOURCE ITSELF calls a pair, which is a line count wearing a cohesion
  label. What does partition is the SECOND SUBJECT: every control that moved compares a written
  sentence with the tree, every control that stayed compares the code with nothing. It takes the
  MODULES-map pair WITH the attribution sweep, which `symbol_attribution_control`'s docstring
  already names as its other half ("the map pair next to it carries both directions for exactly
  this reason"), and it leaves both spelling sweeps together.
  One `registry()` chain, one direction, no cycle: controls → records → properties → invariants
  (the records module merges the property module's fragment exactly as that one merges the
  invariants module's, so `_swept_sources` stays the ONE shared population and is imported, not
  copied). `RECORDS` joins `SELFTEST` in `plan_memo_selftest_mutants.py` with the split rather than
  with the first mutant that needs it, and the **two** MUTANTS rows whose substring moved (both
  R32 seam rows) target it.
  ⚠ **The split CHANGED the graph its own control asserts, and the control said so**: the records
  module imports `ast` and the harness's `HERE`, so both `_IMPORT_SEAMS` rows had to widen — which
  is the first time that table, rather than a prose sentence, was the thing an edit had to move.
  And `symbol_attribution_control` turned RED on its own first run after the carve, naming
  `plan_memo_selftest_invariants.py:16`, whose pre-split name `plan_memo_selftest_properties._IMPORT_SEAMS`
  had to become the records module's: one stale attribution the split created and the suite caught
  unaided.
  ⚠⚠ **AND THEN THE SENTENCE ABOVE DID IT AGAIN — the FOURTH instance on this PR of a record
  breaking the rule it records, and the one that proves the class needs a mechanism rather than
  care.** Written first WITHOUT the `pre-split name` locator, it spelled the dead attribution bare
  in this memo — and `_attribution_corpus` reads `<root>/docs/plans/*.md`, so the always-run wire
  went rc 1 on the very commit whose §7 record asserts `scripts/trip-wires.sh` rc 0. What made it
  reach a commit is not the spelling but the ORDER: the gate was run, the memo was edited AFTERWARDS,
  and a memo edit is a CORPUS edit (`memory/feedback_verified-claims-go-stale-under-own-later-edits.md`).
  The rule this PR now runs under: **the self-test is re-run AFTER the memo edit, never only before
  it** — and it was Axis 5, not the author, that measured it.
  ⚠⚠⚠ **AND IT FIRED AGAIN, in the paragraph that reports it.** Writing §6's discharge note for this
  very finding spelled the same dead attribution bare a second time, the wire went rc 1 again, and it
  was caught only because the new rule above had just been adopted and the wire was re-run after the
  memo edit. That is the case FOR the mechanism and against the care: a rule that had existed for two
  minutes, held by the author who wrote it, in the sentence describing the defect, did not survive one
  paragraph. What survived it is the gate. (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`:
  what works is a checker over the artefact — the R38 dated-locator convention, `` pre-split name `X` ``,
  is the spelling that makes such a sentence legal, and it is now used at both sites.)
  Behaviour-preserving, measured on the two invariants every split on this PR has used: the
  **control NAME SET is identical** (all 657 names and kinds, read off `registry()` rather than off
  the run's printed lines, which wrap) and the **#506 census `--worklist` is byte-identical**
  (`cmp` clean, rc 0, 51 seeds / 715 sites). 657 controls / 364 mutants 0 survived 0 crashed, 0
  `(unknown control)`, `scripts/trip-wires.sh` rc 0. The header's invariant now holds: the largest
  `.claude/tools/plan*.py` is `plan_memo_selftest_mutants_inline.py` at **974**.
  ⚠⚠ **THE COMMAND THAT FIRST "PROVED" THE NAME SET COULD NOT DISCRIMINATE, and the same NUL byte is
  why** (found while fixing the NUL, PR #510 Axis 5). ⚠ The NUL is **not in any source file** — a
  later re-gate measured all 73 tracked files under `.claude/tools` as byte-clean and corrected the
  claim that had stood here; it exists only at RUNTIME, in one control's name, and therefore in any
  dump or log GENERATED from a run. That generated dump holds it, so `grep '^CONTROL' <dump>` treats
  the file as binary and emits NOTHING — the
  attestation `diff`ed two EMPTY streams and printed "identical" for any pair of inputs whatsoever.
  Reproduced: `grep '^CONTROL' base/names.txt | wc -l` = **0** against `grep -a -c` = **657**. The
  claim itself is TRUE — re-measured with `grep -a` the 657 names and kinds are identical, and that
  is the command the record now carries — but for one commit it was an assertion dressed as a
  measurement (`memory/feedback_ao-name-not-section-number-in-briefs.md`'s 「判別しない grep」;
  `memory/feedback_attestation-by-enumeration-not-assertion.md`). ⚠ Two lessons, not one: a
  binary-unsafe verifier is a SILENT false green, and the same defect that made the wire's output
  ungreppable also made the author's own attestation ungreppable — one root, two victims, and only
  the second one was noticed by a person.
  ⚠ Touch-time split at the TERMINAL design re-gate (`plan_memo_selftest_mutants_r26.py` had reached
  **961** lines): the MUTANT registry is split at the review-round seam for the FIFTH time —
  `_mutants_r26.py` keeps R26–R29 (the operating envelope, the file-name token's balanced-pair rule,
  `inline_pass`'s linear contract, the row-noun and straddle properties, the population walk, the
  module map, the id scan, the §4.6 tag list) and `plan_memo_selftest_mutants_r30.py` holds R30 on.
  961 → **523 + 485**. The seam was measured rather than asserted: an AST pass over the boundary
  reported exactly ONE name crossing it (`R31_EMPHASIS_LINEAR`, declared in the R26 half and named by
  a single row in the R30 half), so it moved with the split and the crossing count is **zero** — the
  halves share no imported name and neither imports the other.
  ⚠ **AND IT WAS ONE COMMIT LATE, on a premise this document itself asserted.** A §8 entry written at
  `b8324d06` said "the touch this PR gave `_mutants_r26.py` was a re-point of two rows, not growth —
  so the split is not owed by THIS commit", and set the trigger as "the next commit that adds a
  mutant to it splits it FIRST". That same commit appended two mutant rows (+22 lines, 939 → 961):
  **the trigger had already fired inside the commit that wrote it**, and two independent review axes
  measured that one commit later. So the entry is deleted, the split is taken here, and the invariant
  is a CONTROL — `line_bound_control`, a fold over the `_swept_sources()` population that was already
  there, reporting the largest file every run so no line count is ever transcribed into a document
  again. A prose rule its own author broke inside a single commit is the case for a mechanism
  (`memory/feedback_prose-rules-cannot-fix-unexecuted-claims.md`); the policy's verdict for a ~0-LoC
  mechanical split is "fold", and folding it is what discharges the deferral honestly.
  ⚠ Its mutant's subject is a REAL line count, not the control's threshold: 50 line endings inside
  `plan_memo_lexer.py`'s module docstring take it 961 → 1,011, which the control must report red.
  Mutating the `>= 1000` constant instead would have proved only that the control reads its own
  constant (`memory/feedback_surviving-mutation-means-the-probe-has-another-subject.md`).
  Behaviour-preserving: 660 controls / 369 mutants 0 survived 0 crashed, 0 `(unknown control)`,
  `scripts/trip-wires.sh` rc 0, and the #506 census `--worklist` byte-identical.

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

✅ **THE CAP IS EXCEEDED AND OPTION (d) IS TAKEN — ACCEPTED WITH THIS RATIONALE, BY THE USER, ON
2026-09-21.** `memory/feedback_defer_cap_policy.md` caps a PR at **≤3 OWN** deferrals and counts only
own ones; its PAUSE clause routes the four options (fold / narrow / split / accept with a written
rationale) to the user, and the answer is (d). What follows is the rationale the policy asks for, and
the reason the other three are not available **in this PR**.

⚠ **THE COUNT IS GIVEN AS AN ENUMERATION, NOT AS A FIGURE** — a bare number here has gone stale three
times in this document (`memory/feedback_attestation-by-enumeration-not-assertion.md`). The entries
below, in order: (1) acceptance half of assertion (b); (2) the four assertions' single-home slot;
(3) two KNOWN-MISS bare-id shapes; (4) the always-run wire's cost per review round; (5) the id
grammar's released trailing decoration; (6) Slice 3, Phase 1 offsets; (7) `symbol_attribution_control`'s
existence half; (8) the GFM row splitter; (9) the Markdown library dependency; (10) the `self_id`
suppression homed in §4 row #5; (11) the unbound table making no kind claim; (12) the §6.2 demotion
tag never read; (13) the conformance falsifier's GFM-vs-CommonMark ground truth; (14) the residue
gate's presence comparison; (15) the declaration site that produced no table; (16) the hand-written
table with no detector. **Fourteen own and two pre-existing** — (9) and (10) are the
pre-existing ones, each with its grounding stated where the policy asks for it. Count the list
rather than trusting this sentence: `awk` the `^- \*\*` lines of this section and you must get
sixteen.
⚠ **THE PROSE BELOW CITED TWO ENTRIES, AT FOUR SITES, BY A NUMBERING THE LIST NO LONGER HAS, AND IS
RE-INDEXED** (⚠ this heading said "THREE ENTRIES" at `79309486`; it is two entries — the unbound table
once, the hand-written table three times)
(2026-09-22, the fourth attestation over `9b2e1fa9..00dfd095`). WHY (a) / (b) / (c) were written at
`f87dd8f2`, when the list ended (10) unbound table, (11) hand-written table; inserting (10) `self_id`
at `90ed781a` — and (12)–(15) after it — renumbered the list and not the prose, so WHY (b) called the
unbound table's narrowing "(10)" and WHY (a) / (c) gave the hand-written table's negative-control
requirement to "(11)". Every `(N)` in this section was enumerated against the `^- \*\*` list; those
four were the mismatches. WHY (b)'s now reads (11); WHY (a)'s two and WHY (c)'s one were re-indexed to
(16) at `79309486` and then replaced at `b325c668` — WHY (a)'s by the set its entries' own text
supports, WHY (c)'s by a rule with no number (⚠ this sentence said "they now read (11) / (16)" and went
stale under `b325c668`'s own edit — the sixth attestation). A citation by POSITION goes stale under any
insertion above it, which is why the enumeration names each entry beside its number.
⚠ **The re-index was checked against TODAY's numbering only, and that is half the check** (2026-09-22,
the fifth attestation). A `(N)` is right or wrong in the numbering IN FORCE WHEN ITS SENTENCE WAS
WRITTEN, so every `(N)` in this memo that names a §8 entry was re-checked by `git blame` against the
list at that commit (`f87dd8f2`: (10) unbound, (11) hand-written; `90ed781a` → `799349db`: (10)
`self_id`, (11) unbound, and (12)–(16) as today). Outside §8 the memo cites no §8 entry by number (its
other `(N)` are spec-example counts). Inside it, one sentence was wrong at birth rather than stale: the
R51 audit note under WHAT THE SHAPE SAYS, which judged an `f87dd8f2` sentence by `799349db`'s numbering
— corrected there. And WHY (a)'s set was wrong by CONTENT once re-indexed — corrected there too.
⚠ **(12) AND (13) ARRIVED AT R47 AND BOTH MOVE THE OWN COUNT**, from ten to twelve. Neither is a
deferral of convenience: (12)'s one-line fix overturns a control this plan RATIFIED — the direction
cmark supports is the one the ratified control rejects — and (13) changes what the conformance proof
asserts by giving it a second ground truth. The user's option-(d) acceptance was given at ten; this
records the movement rather than absorbing it, because the policy's whole point is that the number
is a classification and not an edit.
⚠ **THE (12) HALF OF THE SENTENCE ABOVE IS A FALSE REASON FOR A DECISION THAT STANDS** (2026-09-22,
found by the §8 carves' plan-review). The one-line fix does overturn R42's twin; the implication that
fixing (12) REQUIRES overturning it does not hold. Asking the kind question of a keep-free reading
while the attribution subject stays on the naming reading fixes the reproduction and leaves R42's twin
untouched — a PROTOTYPE, not shipped: the reproduction exits 0 today (below) and the prototype makes
it 1. So (12) is still not a fold, but for the edge-dense reason (the fix changes the kind READING
and the residue MEASURE together and needs readings to be values first), not the overturn. The
classification and the count are unchanged: (12) remains own.
⚠ **(10) ARRIVED BY AUDIT, NOT BY REVIEW** (PR #510 R45). It is a real deferral of this PR's subject
that a reader auditing §8 could not find, because its home (§4) has no trigger column and no date
— so the enumeration was complete over §8 and incomplete over the PR. Adding it moves the total and
not the CAP, which counts own deferrals only; that it changes nothing about the decision is what
makes writing it down cheap, and leaving it out dishonest.

⚠ **Two entries left the list this session and NEITHER is a tally edit** — and the test of that is
that the decision is (d), so removing them buys nothing: the §6.3 / §6.6 / §6.4 spec-prose entry is
**settled** (below: one reading fixed, two closed as non-defects, against two reference
implementations), so it is no longer a deferral of anything; and the standalone *"Phase 1 hands a
container's content to itself as a LIST OF STRINGS"* entry was a **duplicate** of Slice 3, whose own
text says the two halves are ONE carve and which was written at R38 to replace it. The R38 re-slice
was applied to the prose and not to the list. The earlier corrections that cancelled (the touch-time
split folded because the work was done; the GFM splitter re-classified own on a measurement) stand.

**WHY (a) FOLD IS NOT AVAILABLE.** The policy's *容認しない pattern* — "~30–150 LoC で本 PR に fold
できるが念のため defer 化" — is the case for folding, and this PR has already taken it **twice by
measurement**: the §6.4 carve folded at R42-5 once the fix measured ≈45 lines across two modules with
one reader each, and the §6.6 reading folded this session at ~20 lines once a reference implementation
settled it. What is left does not fit that shape. (5), (6), (12) and (14) — the entries whose OWN
text puts them under the edge-dense rule — are each explicitly
`/elidex-plan-review`-before-implementation **by CLAUDE.md's edge-dense rule, not by judgment** — (5)
intersects the id grammar, the grammar↔scan agreement property, the `_glued` boundary rule, the
decoration-run cost contract and both mention passes; (6) intersects the block grammar, both container
passes, the raw-extent seed and the Phase-1 cost contract; (12) intersects the id-grammar decoration
exception, §6.4's demotion, `MARKER_RE`, the residue gate and the `keep` set; (14) intersects the gate,
`_kind`'s ordering, the attribution pass, I-A's straddle rule and a ratified control.
⚠ **This set read "(5), (6) and (16)" at `79309486`** (re-indexed from "(11)", which at `f87dd8f2` was
the hand-written table), and (16)'s own text claims neither edge-dense nor plan-review, while (12) and
(14) — added after the sentence — say BY RULE. Re-derived from the entries' text (the fifth
attestation); (11) and (15) name Slice 2's plan-review as OWNER but not by the rule, so they are not
in it. Folding any of them here is precisely the *single PR + skipped
plan-review* that `memory/feedback_edge-dense-mandatory-plan-review-and-split.md` exists to prevent.
(1), (2) and (3) are in the ratified plan from BEFORE implementation and were reviewed as design.

**WHY (b) NARROW SCOPE IS NOT AVAILABLE — AND WHERE IT WAS TAKEN.** Narrowing removes a deferral only
when the narrowed remainder is empty. It was taken where it could be: (11) is a narrowing, its
census-claim half shipped with four controls and five mutants and only the no-claim residual carried.
Narrowing the rest does not reduce the list — (4) is the wire's own budget with a measured trigger,
(7) is blocked on §3's `(NEW)` / `✗ (absent)` conventions, (8) is trigger-only on a condition
(`two of them on main`) that is measurably not met.

**WHY (c) SPLIT THE PR IS NOT AVAILABLE.** (c) is already the shape of every entry that is re-sliced
to a named owner with a trigger and a re-eval date — read off each entry's own **Owner** / **Trigger**
/ **Re-eval** lines, not restated here (⚠ this said "three entries — (5), (6) and (16)" until the fifth
attestation, a hand-written set that the entries added since had outgrown) — which is what
`memory/feedback_defer-accumulation-signals-mis-drawn-slice.md` asks for. Splitting what remains
would mean cutting the checker itself, and the pieces do not separate: every entry names the SAME
program, and the split this PR could take at a real seam it has taken **six times** already as
standalone touch-time commits (§7 Slice 0). A further split would be a split of the deferral LIST,
not of the work — which changes no reader's decision and loses the one home that ties the entries to
the design they came from.

⚠ **WHAT THE SHAPE SAYS, stated because the number alone would hide it.** **The own entries whose
own text names a review round, a design re-gate or a review axis as their origin** — (4)–(7) and
(11)–(16), ten of the fourteen — were produced by the REVIEW LOOP rather than by the plan, and that is
the honest reading of why the cap is breached:
⚠ **THE SET THAT STOOD HERE, "(4)–(8) and (11)–(15)", FOLLOWED NO STATED RULE** (2026-09-22, the fourth
attestation). It counted (8), whose text names no round — it is own because a MEASUREMENT reclassified
it — and left out (16), which names Axis 5; the total happened to be ten either way, and the set was
not re-derivable from anything. The rule is now the sentence's first clause, so a reader re-derives
the set from each entry's parenthesis rather than trusting it.
⚠ **THIS SENTENCE WAS STALE IN THREE WAYS AT ONCE AND IS CORRECTED HERE** (PR #510 R51 audit). It
read *"Six of the ten own entries … (4, 5, 6, 7, 10 and the row splitter)"*: the headline had moved
to fourteen own two commits earlier; it listed **(10)**, which the enumeration above classifies
**pre-existing**, so it cannot be an own entry at all; and (10)'s own paragraph says it *"ARRIVED BY
AUDIT, NOT BY REVIEW"*, which this sentence contradicted. A count, a classification and a provenance
all wrong in one clause — and every one of them re-derivable from the list ten lines above it. That
is the argument for the enumeration form, made against the paragraph that introduced it. a 40-round external convergence on a program
whose subject is itself a checker generates carves faster than a cap written for feature PRs
anticipates. That is an argument for (d) here, and an argument for
`memory/feedback_defer-accumulation-signals-mis-drawn-slice.md` being applied to the NEXT slice
boundary — not for reasoning the number down, which the policy forbids outright
("数合わせのための slot 削除 / merge は禁止: 判定は分類であって編集ではない").
⚠ **TWO OF THE R51 NOTE'S THREE "WAYS" ARE FALSE — it judged an `f87dd8f2` sentence by `799349db`'s
numbering** (2026-09-22, the fifth attestation). When *"Six of the ten own entries … (4, 5, 6, 7, 10 and
the row splitter)"* was written, at `f87dd8f2`, the list's (10) was the UNBOUND TABLE — own, and derived
from review round R42-10 — so listing it was right, and the `self_id` entry the note measured it against
did not yet exist (it was inserted as (10) at `90ed781a`). What the note had right is the first way
only: the count was stale (ten own at birth, fourteen when the note was written). The sentence went
stale by RENUMBERING, not by misclassifying — which is the very failure a citation by position invites,
committed here by the note that corrected it.

- **(own)** Acceptance half of assertion (b) — "not implementable here"; slot
  `#11-plan-memo-acceptance-falsifiability-check` is minted in #506's memo (§5 mention `190d2adb:…:1218`, §8 row `:2711`)
  and is **not yet in the slot SoT ledger** (`memory/project_open-defer-slots.md` = 0 hits); its
  ledger registration is owed at #506's landing, not here. No date — trigger = #506 landing.
- **(own)** The four assertions' owner, `#11-plan-memo-spec-field-single-home-check` (cited by the tool headers
  as "the single-home slot #506's memo §8 mints") — the same pre-agreed commitment: minted in #506's
  memo §8 (`190d2adb:…:2710`), **not in the slot SoT ledger** (0 hits), registration owed at #506's
  landing, not here; the headers cite that origin rather than presenting the slot as registered.
  ⚠ **Trigger = #506's landing, and no date — the same trigger-only form as the entry above it**,
  which this one had left unstated (PR #510 R45: an audit over all eleven entries found exactly two
  carrying neither a date nor an explicit trigger-only declaration, this and the wire's budget).
  Stated, because an entry a reader cannot schedule is indistinguishable from one nobody re-reads.
- **(own)** Two KNOWN-MISS bare-id shapes (numeric / single letter) — declared in the self-test; trigger = a
  memo minting such an id; no slot (seed boundary, not a platform gap); no date — trigger-only.
- **The always-run wire's cost grows with REVIEW ROUNDS, not with the program** (PR #510 R32 — **own**).
  Measured on one clean `git clone --local`, same session: `2c713e51` 11.80 / 12.13 / 12.19 s against
  `7d43d7cd` 26.05 / 26.04 / 27.25 s — **2.2×**, and the `ci.yml` block that is the declared single
  home had stood ~2× stale for two rounds because R31 added 12 controls and 17 mutants without
  re-measuring. The timeout is re-derived there (5 → 10 min) under that block's own rule. ⚠ What is
  NOT fixed is the driver: the generated corpus costs ~3.6 s and runs once in `--self-test` plus once
  per mutant naming it, and **six** rows name it (measured by loading `MUTANTS`; a `grep -c` counts
  the constant's definition and import lines and says ten). Self-test alone is ~6.5 s of ~29 s.
  ⚠⚠ **THE `~29 s` IS STALE AND THE VERDICT THAT CALLED IT AN FP WAS WRONG** (PR #510, 2026-09-20).
  It was re-measured on a clean clone as 28.53 / 29.34 / 27.46 and declared to hold; a review axis
  then measured **25.67 – 26.80** at the same head under the same stated condition — **ranges that do
  not overlap**. Arbitrated on a third clean `git clone --local` at `7121476e`, sequential, nothing
  else of this project running: **this wire 24.88 / 24.93 / 25.05**, the whole job (all five wires)
  26.10 / 26.22 / 26.73, self-test alone ~6.5 s. The wire is **~25 s**; the FP verdict is retracted.
  ⚠ **What the episode shows is not who measured badly — it is that the ABSOLUTE is not reproducible
  on this host to better than ~20%**: three "quiet clean clone" runs of the same command at nearby
  heads gave 24.9, 26.1 and 28.5 s. And the concurrency attribution (31.96 / 35.05 / 39.85 "measured
  concurrently with other work") is **not supported**: the whole job measured quiet lands in that same
  neighbourhood, so SUBJECT (one wire vs five) explains as much as load does. The figure that has
  never moved is the RATIO — self-test ≈ 6.5 s of the wire — which is what the derivation rests on;
  an absolute is written only with its SUBJECT beside its condition. Making
  that not grow means a mutant run answering only "does this control go red", which a smaller corpus
  can do — but corpus size is the proof's STRENGTH, so shrinking it per-row is a change to what the
  proof asserts and needs its own measurement per row. Trigger = the round whose head measures under
  2× against the re-derived 10 minutes, or the next round that adds a mutant naming the growth
  property. No slot: this is the wire's own budget, not a platform gap. ⚠ **And no date — this is
  trigger-only, stated rather than left to inference** (PR #510 R45, the second of the two entries
  the hygiene audit found carrying neither).
  ⚠ **The "six rows name it" figure is WITHDRAWN**: the entry never named the constant or the
  command, `MUTANTS` is now 399 rows across ten modules, and no stated reading discriminates six
  from ten. A figure whose command is not written down cannot be re-derived, which is the same
  defect as a figure whose predicate is not written down.
  ⚠⚠ **And the budget is what is actually blocking the generated property from subsuming three
  per-shape controls** — R32's design re-gate measured all three, so the next PR starts from the
  measurement and not from a re-derivation:
  - **C (R31-2, emphasis)**: +3 atoms read off `plan_memo_emphasis.DELIMS` (derived, not written
    against the symptom) → 3,577 → **4,030 probes, 3.4 → 4.6 s**, **RED** on the defect, green on the
    fix. Cost through the six naming mutants ≈ **+8 s**.
  - **D (R31-3, raw seed)**: the corpus **UNCHANGED**, driven through `check()` instead of
    `_read_block` — an 8-line driver — is green on the fix and **RED** on the defect over 97 probes
    (~2.5 s), naming `plan-memo-umbrella-check.py:466` at 1.59×, *including a single-atom shape
    (`'[C1] '`) no review round reported*. The full pair corpus through `check()` is ~90 s, so the
    affordable form is an atoms-only arm. This would subsume `linear_raw_seed_control`.
  - **E (R31-4, licensing)**: `_count_pattern_spans` is the first witness whose subject is the C `re`
    engine, and its population is mechanically ENUMERABLE — which is the whole of the argument. One
    control wrapping every enumerated pattern over a growing pipeline probe, with the ≥1-application
    lower bound the existing control already uses, is the general form. The watched half is measured
    and is **one**: `grep -rn '_count_pattern_spans(' .claude/tools/` returns a single instantiation
    (`plan_memo_selftest_pipeline.py`, on the licensing pattern).
    ⚠ **THE DENOMINATOR THAT STOOD HERE IS GONE (PR #510 Axis 5, 2026-09-20).** It read "**39
    module-level `re.Pattern` globals** across the 13-module set" and it reproduces under NO
    enumeration: module-level names bound to `re.compile` give **41** over every `plan*.py` and
    **34** over the thirteen non-selftest modules; counting every `re.compile` call in a module-level
    statement (tuples and dicts included) gives **60** and **47**. A number whose value depends on
    which convention the reader assumes is an argument, not a measurement
    (`memory/feedback_convention-dependent-figures-are-argument.md`), and it was the stated SCOPING
    BASIS of this carve. What the carve needs is the population's DEFINITION and the command that
    enumerates it — "the module-level names bound to `re.compile` over the module set", by
    `ast.parse` over `tree.body`, not a grep for `re.Pattern` — so that is what stands, and the
    control derives its own denominator at run time the way every other enumerating control here
    does.
  ⚠ **What was wrong was not the decision but the stated reason.** The R31 ledger measured "does the
  property *as written* cover D and E?" and treated the answer as settling "should a per-shape control
  be written?" — the deciding question is whether the property can be **parameterised**, and it can.
  The three docstrings that recorded a reachability limit now record the cost instead
  (`plan_memo_selftest_growth.py`, `plan_memo_selftest_work.py`, `plan_memo_selftest_pipeline.py`).
- **The id grammar releases a REJECTED core's trailing decoration** (PR #510 R36-3 — **own**; **real,
  reproduced, NOT fixed here**). `prefix**C** owns it` reports nothing while `prefix **C** owns it` reports a site:
  in `plan_memo_ids.tokens` the rejected `prefix` core takes the opening `**` as its right decoration,
  `C` cannot reclaim it, comes out `balanced=False`, and the bare pass drops it as an undecorated
  single letter — exit 0 on an ownership claim.
  ⚠ **A fix was written, measured, and REVERTED because it broke a shipped invariant**: releasing the
  decoration to the rejected core's end turned `id_scan_grammar_agreement_control` red on
  `9z#11-a**[C1]` (3 disagreements). Asking the grammar directly shows why — `decorated_id`'s own
  composition ALSO gives `C` no left decoration in `prefix**C**`, so the scan and the grammar agreed
  and **the defect is in the grammar**, not in the scan that mirrors it.
  ⚠ **That makes it edge-dense by CLAUDE.md's own test** — it intersects the id grammar, the
  grammar↔scan agreement property, the `_glued` boundary rule, the decoration-run cost contract
  (R29-2) and both mention passes — so it is `/elidex-plan-review`-before-implementation **by rule,
  not by judgment**.
  ⚠ **Trigger, rewritten at the R38 re-gate because the first one was CIRCULAR**: "trigger = that
  plan-review" makes the WORK its own occasion, so a reproduced silent miss (`prefix**C** owns …` →
  rc 0) had nothing scheduling it. The occasion is **Slice 2's plan-review**, which is already
  scheduled and already opens this area (I-D/I-E, the licensing predicate and `_anchored`); the
  decoration-release rule is decided there or explicitly carried with a reason.
  **Re-eval: 2026-12-31.** No slot: it is this checker's own grammar, not a platform gap.
- **▶ SLICE 3 — Phase 1 carries OFFSETS, not strings** (PR #510 R29-1 partial, R36-1 — **own**). ⚠ **This was
  written first as a third §8 entry appended to the one whose trigger it fired, and the R38 design
  re-gate refused it**: the entry cited
  `feedback_defer-accumulation-signals-mis-drawn-slice` — "the boundary wants re-drawing rather than
  another entry" — and was another entry, re-deferring an already-fired trigger with no new trigger,
  no date, no slot, and a home ("carried to the next slice") that existed in no ledger. §6's Slice-2
  acceptance row and §7's Slice 2 touch set name neither `blocks.py` nor the container passes. So the
  two halves are ONE carve with a real owner:
  **Scope**: Phase 1 hands a container's content to itself as a LIST OF STRINGS, so the characters
  materialised are Σ(content length) over the nesting levels. Both halves have now fired — the QUOTE
  half at R29-1 (partly discharged: `quote_content` was the marker TEST and the BUILD in one
  function; `quote_marker` is that test now) and the LIST half at R36-1 (`item_marker()` rebuilds the
  whole suffix at every nested `- ` marker: 0.36 / 1.40 / 5.54 / 21.58 s over 2,000 / 4,000 / 8,000 /
  16,000). ⚠ **Splitting test from build does NOT fix the list half** — measured at R38: it removes
  2/3 of the characters materialised (48.0M → 16.0M at n = 4,000) and moves wall time ~1%, leaving
  the exponent unchanged. The remedy is a "line" carrying an OFFSET rather than a string, through
  every `blocks.py` predicate and both container passes.
  **Owner**: its own PR under CLAUDE.md's edge-dense rule (≥3 intersecting invariant axes: the block
  grammar, both container passes, the raw-extent seed and the Phase-1 cost contract), so
  `/elidex-plan-review` precedes implementation.
  **Trigger (an EVENT, not an action)**: a memo in this family whose list or quote nesting exceeds
  depth 1 — today the deepest real nesting is 1, which is why it does not bite yet and is measurable
  at any time by `grep`. **Re-eval: 2026-12-31**, so it is scheduled rather than trigger-only.
  No slot: this is the checker's own Phase 1, not a platform gap.
- **`symbol_attribution_control` ships only the COMPLETENESS half** (R38 design re-gate — **own**). It checks
  that an attribution names the module that DEFINES the symbol; it cannot see an attribution whose
  symbol exists **nowhere**, because `if sym not in home: continue`. The map pair next to it carries
  both directions for exactly this reason (`module_map_existence_control`: *"the rename half the
  completeness direction cannot see"*). With the denominator cleaned (a `` `mod.py` `` mention is no
  longer misread as the symbol `py`), the skip arm is **nine distinct `module.symbol` PAIRS over fourteen
  SITES** — ⚠ and the sentence that stood here, *"exactly nine attributions, all in §3's coverage
  map"*, was false in both halves (PR #510 R45, replayed through the control's own loop). "All
  in §3" is refuted by the list's own first member: of the FIFTEEN sites, fourteen are in THIS MEMO
  and exactly one is in a real module — a docstring in `plan_memo_selftest_properties.py`.
  ⚠ **Three particulars of that correction were themselves wrong when written, and are corrected
  here**: the site count was given as fourteen (it is fifteen — the fifteenth is the paragraph that
  wrote the correction); the one real site was attributed to `plan_memo_selftest_records.py`; and
  `grep -c 'plan_memo_ids\._TOKEN' <this memo>` was said to return exactly one line, which was true
  when measured and became **two** in the same commit, because this entry is the second. A
  correction that lands inside its own corpus falsifies itself
  (`memory/feedback_document-landing-invalidates-its-own-measurements.md`); the command is what
  stands, not the number. A universal falsified by the
  paragraph asserting it is the shape
  `memory/feedback_universal-claims-need-the-complement-measured.md` names, and the enumeration is
  now of PAIRS with the site count beside it: `plan_memo_ids._TOKEN`, `plan_memo_lexer._TOKEN`, `plan_memo_lexer.links`,
  `plan_memo_tables.` × `_link_destination` / `_link_title` / `code_spans` / `fenced_spans` /
  `find_tables` / `links`. Measured: **zero definitions in the tree** for all seven distinct symbols.
  ⚠ **The existence half is NOT a one-line addition**, which is why it is carved rather than written
  in the same breath: §3 marks a PLANNED site with its own convention (`(NEW) …` / `✗ (absent) —
  Slice 1`), and `fenced_spans` is one of those — legitimately undefined. A half that flags planned
  sites is a control that must be silenced, which is how a gate stops being read. It needs the map's
  conventions read first, and this session has already produced three detectors whose populations
  were wrong on the first run. **Trigger (an EVENT)**: the next round that reports a dead §3 pointer,
  or the next touch-time split, which is what makes one. **Re-eval: 2026-12-31.** No slot: it is this
  checker's own map.
- **(own** — ⚠ **the `(pre-existing)` grounding written here first was FALSE, and the policy asks for
  the grounding precisely so the cap cannot be re-litigated** (`memory/feedback_defer_cap_policy.md`:
  「pre-existing 判定の根拠 … も併記」). It read "the duplication is on `main` across three branch
  families". Measured: `git grep -c split_row origin/main` = **0 hits**, and `origin/main` carries
  exactly **one** GFM row splitter, `elidex-plan-review/preflight.py::_parse_table_row`. The other
  three are on UNLANDED branches and one of them — `plan_memo_blocks.py::split_row` — is introduced
  **by this PR**. That makes the deferral own. ⚠ **The count that stood here — "10 own /
  1 pre-existing, not 9 / 2" — is REMOVED rather than refreshed**: it was true when the entry was
  written and the list has grown five times since, so a tally inside an ENTRY is a second home for
  a figure the section's own enumeration already carries. The classification of THIS entry (own, on
  the measurement above) is what belongs here; the count belongs to the enumeration and nowhere
  else. The entry's own trigger ("two of them on `main`") is consistent with one and stands.**)**
  GFM row splitter duplicated four ways across three branch families — trigger = two of
  them on `main`; Slice 1's `split_row` is the candidate canonical copy; no slot; no date — trigger-only.
- **(pre-existing** — a standing project choice predating this PR**)** Markdown library dependency
  (§5) — trigger-only (see §5); no slot; no date.
- **(pre-existing, and HOMED IN §4 ROW #5 — listed here so §8 is a complete index of this
  PR's deferrals)** `self_id` exclusion hides a forbidden self-attached role. A row's id is discarded from
  EVERY cell by both mention passes, not only from the leading declaration in its id cell, so
  `9z owns integration.` written inside 9z's own Slice cell reports nothing while the identical
  sentence in prose reports one site — both at rc 0. ⚠ **Grounding for the pre-existing call**
  (`memory/feedback_defer_cap_policy.md` asks for it so the cap cannot be re-litigated): the row is
  in the RATIFIED plan from before implementation — `git log -S 'self_id\` exclusion hides a
  forbidden self-attached role'` returns `aee896dd`, the umbrella-plan commit whose subject records
  the plan-review converging in three rounds, and the branch's first code commit is `5e9439b4`. It
  is therefore the same category as entries (1)–(3), not a loop-accumulated deferral. ⚠ What DID
  happen in the loop is that R33-3 WIDENED it: the `(+ control flip)` clause was added at
  `2c14695d`, because the fix turns an existing NEGATIVE control red — suppressing the id alone
  would ship a FABRICATED finding.
  ⚠ **Why it is written here at all**: §4's columns are `# | Site | Defect | Invariant | Slice |
  Closed by` — there is no trigger column and no date, so a reader auditing THIS section for the
  PR's deferrals would not find it (PR #510 R45). **Owner**: Slice 2. **Trigger (an EVENT)**: Slice
  2's implementation, which owns the `_anchored` predicate this sits in. **Re-eval: 2026-12-31.**
  Counted in the enumeration above as (10), pre-existing — not against the cap, which counts own
  deferrals only.
- **(own)** **AN UNBOUND TABLE THAT MAKES NO KIND CLAIM IS STILL SILENT** (PR #510 R42-10, **the
  CENSUS-CLAIM half is FIXED in this PR; this is the residual**). The schema-miss gate asks only of
  `main` — deliberately, since a linked detail memo may hold no slot ledger — so a linked memo whose
  slice table binds to NO schema left the census with no diagnostic at all and the run exited **0**.
  ⚠ **What is now closed**: a table binding to no schema while carrying a `KIND_PHRASES` phrase makes
  a CENSUS CLAIM, and that contradiction is `Population._unbound_claims`, with four controls and five
  mutants. ⚠ **The predicate was picked by MEASUREMENT and two candidates were refused, both of them
  the ones this entry previously carried as unmeasured**:
  - *the first column tokenises as row ids* — exactly right on the four fixtures and **151** unbound
    tables over the corpus below (a landing record's `obj` / `R1`…`R7` review tables are id-shaped
    and legitimate). Refused, and pinned by a NEGATIVE control plus a mutant that re-injects it.
    ⚠ **The READING is part of the predicate and the first statement of it here was loose enough to
    be three different numbers**: 151 is *every body row's first cell*; *any* body row's gives 333,
    and *any row including the header* more again. The mutant that re-injects the predicate was
    written to the second reading while this sentence quoted the first, so the figure and the thing
    it justified disagreed. Both now say *every body row*
    (`memory/feedback_convention-dependent-figures-are-argument.md`: a number whose value depends on
    the convention the reader assumes is an argument, not a measurement);
  - *the header NEAR-MISSES a schema's* — silent on that corpus too, and refused for a reason no
    corpus count shows: it fires on a renamed header whose table **declares nothing**. Pinned by its
    own NEGATIVE control and a mutant.
  - the shipped predicate — *a kind phrase in any cell, read off the RENDERED cell* — fires **0**
    times over those same 141 memos / 511 tables / **507 unbound**, while the phrases themselves DO
    occur in that corpus, so the zero is a silence and not an empty population.
    ⚠ **The occurrence count is deliberately not transcribed here, and the reason is that writing it
    once already falsified it**: the figure was **77**, and the memo edit in the very commit
    asserting it removed one `KIND UNDETERMINED` occurrence, making it **76** at the commit that
    claimed 77. This document is IN the corpus, so that number moves whenever this paragraph is
    edited — the exact failure `memory/feedback_document-landing-invalidates-its-own-measurements.md`
    names, cited three lines below and not applied to the number three lines above it. What the
    argument needs is *nonzero*, which the command settles at read time.
    ⚠ **Every figure in this entry is a DATED MEASUREMENT with its corpus beside it, never a
    standing claim** — 2026-09-21, the `docs/plans/*.md` of this worktree and of
    `elidex-wt-vmp4plan`, a corpus that exists on no other disk. The commands, so a reader
    re-derives rather than trusts: the sweep is the checker itself
    (`plan-memo-umbrella-check.py <memo>` over each file, counting the `binding to NO schema`
    line); the positive control is `KIND_PHRASES` `findall` over the same file list; the
    denominators come from `Memo(f).tables` over it.
  ⚠ **AND THE GATE FOUND ONE INSIDE THIS SUITE'S OWN FIXTURES** — R31-1's `$`-header NEGATIVE is a
  linked memo whose table binds to nothing while carrying an umbrella row, the identical class
  reached independently and with a different header. Its site-count measure also asserted `rc != 2`,
  and that implicit half had been blessing the silence. The fixture and the subject are untouched and
  the measure moved to the gate; replacing the marker to quiet it would have taken the
  discrimination with it (`memory/feedback_control-rewritten-to-bless-the-defect.md`).
  **Scope of what remains**: an unbound table making NO kind claim. Its ordinary rows are lost just
  as silently, and no predicate measured here separates one from a documentation table that happens
  to key its rows — which is the 151 above. The gate's docstring states this as what it cannot see.
  **Owner**: Slice 2's plan-review, which owns the prose predicates.
  **Trigger (an EVENT)**: a round that reports a declaration lost from an unbound table carrying no
  kind phrase, or a second corpus measurement that separates the two populations.
  **Re-eval: 2026-12-31.** No slot: it is this checker's own gate.
- **(own)** **THE §6.2 DEMOTION TAG IS RECORDED AND NEVER READ, and in alt text that loses a census
  claim** (PR #510 R47, blind-spot audit — **real, reproduced, NOT fixed here**). `dispose` unpacks
  the emphasis entries discarding the kind (`…, use, _k`) where `code`, `autolinks`, `html` and
  `images` all read their `"demoted"` tag. Reproduced with a declared single-letter row id:
  `![UMBRELLA, not **a** terminal unit](img.png)` exits **0** while cmark's alt is
  `UMBRELLA, not a terminal unit` — the marker VERBATIM — and the single-star twin `*a*` exits 1,
  so the checker distinguishes two spellings the reference renders identically. The `**` survives
  into the stream because `id_only("a")` is true and the decoration exception fires.
  ⚠ **Why it is NOT fixed in the round that found it**: the one-line fix (read the tag) also makes
  `![**9z**7z](i.png)` read as the single token `9z7z` — which is cmark's answer — and that turns a
  SHIPPED control red, the `**` twin R42 ratified as reporting TWO ids. Changing it is overturning a
  recorded decision, not patching a slip, and the direction cmark supports is the one the ratified
  control rejects.
  ⚠ **That is a reason about THIS fix, not about every fix** (2026-09-22, the carves' plan-review): the
  root is that the KIND is read off a stream whose decoration exception depends on the keep-set —
  outside any image, a declaring field reading `UMBRELLA, not **a** terminal unit` makes the row
  TERMINAL when `a` is in the keep-set and UMBRELLA when it is not; the rc shows the flip only when the
  row carries a `Deps` edge (0 declared / 1 undeclared), and without one both exit 0 (⚠ restated: the
  first wording put the difference in the rc, which holds only with the edge) — and a keep-free kind
  reading fixes it without touching R42 (prototype). Not folded here all
  the same: edge-dense, as the next line says.
  ⚠ **Edge-dense by CLAUDE.md's own test**: it intersects the id-grammar decoration exception, §6.4's
  demotion, `MARKER_RE`, the residue gate (`kind_disagreements` cannot see it — both readings agree,
  measured) and the `keep` set. `/elidex-plan-review` before implementation, BY RULE.
  ⚠ **The `kindcell` control family has the matching hole**: link / empty link / nested image / code
  span / raw HTML / plain are all covered and emphasis — the one construct whose tag is dropped — is
  not. **Owner**: Slice 2's plan-review. **Trigger**: already fired. **Re-eval: 2026-12-31.**
- **(own)** **THE CONFORMANCE FALSIFIER COMPARES A GFM READER AGAINST A PURE-CommonMark RENDERER**
  (PR #510 R47 — the same fabricated-falsifier class as the §6.4 one this PR fixed, in the one
  construct that PR did not reach). `a ~~b~~ c` gives *"the html emits 0 `<del>`, Phase 2 claims 1
  GFM strikethrough pair"*: the lexer reads GFM strikethrough, the vendored html is cmark's and
  cmark has no such extension. Corpus-unreachable (zero `~` pairs in the 335 inline examples) and
  pre-existing, so nothing has ever been red.
  ⚠ **It is a DECISION, not a patch**: `cmark-gfm 0.29.0.gfm.13` is now installed on this host (it
  settled the delimiter FP this round), so the falsifier could compare GFM constructs against a GFM
  renderer — but that makes the conformance corpus two corpora with two ground truths, which is a
  change to what the proof asserts. **Owner**: Slice 1's next touch of the conformance module.
  **Trigger**: the next round that reports a falsifier disagreement on a GFM-only construct, or a
  `~` pair entering the corpus. **Re-eval: 2026-12-31.** No slot: it is this checker's own proof.
- **(own)** **THE RESIDUE GATE COMPARES PRESENCE, AND A CLEAN PHRASE BESIDE A STRADDLING ONE HIDES
  THE STRADDLE** (PR #510 R48-1 — **real, reproduced, a fix written and REVERTED**).
  `kind_disagreements` asks `bool(hit) == bool(other)`, so a declaring field carrying BOTH a clean
  phrase and one straddling a masked span reads as agreement. Measured:
  ``Slice 7z — **UMBRELLA, not a `terminal` unit.**`` alone is **rc 2**; adding
  `Then **UMBRELLA, not a terminal unit.**` after it takes the run to **rc 1** with only a
  non-gating `LEX-SPLIT?` seed — the reader attributes the FIRST marker to `7z` (a POINTER) while
  the stream misses it and reads the later one as self-declaring (an UMBRELLA). The kind itself
  flips, which is exactly the doubt the gate exists for.
  ⚠ **The obvious fix — compare COUNTS — was written, measured, and reverted**: it turns the R23
  NEGATIVE control red, and that control is RIGHT. Its fixture
  ``KIND UNDETERMINED.  Also KIND UNDETER`MINED`.`` also has 2 occurrences against 1, and there the
  kind is undetermined under BOTH readings, so the census genuinely is not in doubt. Counting
  cannot separate the two cases; the property that does is whether the two readings declare the
  same KIND **and the same ATTRIBUTION**.
  ⚠ **And that question cannot be asked where the comparison lives.** `kind_disagreements` is
  per-phrase and has no row; `_kind`'s ordering (marker > undetermined > pointer) and
  `attributed_to_other` are the Population's, and `_kind` has side effects (`self.attributed`), so
  asking it twice is not free. The fix is a real re-slice of the residue gate, not a predicate
  tweak — it intersects the gate, `_kind`'s ordering, the attribution pass, I-A's straddle rule and
  a ratified control, so it is `/elidex-plan-review`-before-implementation **BY RULE**, the same
  disposition and for the same reason as the id-grammar decoration release above.
  **Owner**: Slice 2's plan-review, which owns the prose predicates and already opens `_anchored`.
  **Trigger**: already fired (reproduced above). **Re-eval: 2026-12-31.** No slot: it is this
  checker's own gate.
- **(own)** **A DECLARATION SITE THAT PRODUCED NO TABLE AT ALL IS INVISIBLE TO THE LOST-DECLARATION
  GATE** (PR #510 R50 — **real, reproduced, a fix written and REVERTED**). The gate walks
  `memo.tables`, which is what Phase 1 ADMITTED. A linked memo carrying the EXACT slice header with
  a malformed delimiter row — five cells under a six-cell header — forms no GFM table (§4.10), so
  the gate never sees it. Measured: the sibling's umbrella row and its nonempty `Deps` stayed out of
  `ids` (4 tables against 5, `Wz` and `Tq` absent) and the run exited **0** with the declaration
  gone. It is the I-C class one layer below the R47-2 fix: that one caught a table that bound to no
  schema, this is a header that became no table.
  ⚠ **A second arm over the raw lines was written and measured, and it is REVERTED for two
  independent reasons.**
  **(i) It over-fires on a position where no declaration can live.** A schema-shaped line inside an
  INDENTED CODE BLOCK is code, not a declaration, and a raw-line population cannot tell — a shipped
  control (`` `\t| Slot | … |` over `\t|---|…|` ``) went red. The population must be "lines where a
  table could have been admitted", which is a Phase-1 block-context question and not a `"|" in line`
  test. That is the population-by-symptom-vocabulary trap the §8 entry below this one names.
  **(ii) It requires overturning FOUR ratified controls.** ⚠ *Two particulars below are false, and the
  reason (a design question) stands* (2026-09-22): only one of the four measures `rc` (the `rcase`);
  the other three measure `sites` / `id` and carry the harness's implicit rc half
  (`grep -n 'res.rc != 2' .claude/tools/plan_memo_selftest_harness.py`); and "FOUR" is a floor, not
  the count — a prototype of the arm (on a per-line reading since rejected) turned more red, so the
  count is for the owning slice to re-derive on the arm it adopts. "(table) header and delimiter of unequal
  width are not a table", "(table) a delimiter cell is ≥1 hyphen; `:` alone is not", "(rc) a slice
  header over a one-cell delimiter row is not a table…", and the lazy-quote header control all
  assert **rc 0** beside their own subject. Their subject (is this a table?) survives the change and
  only the measure would move — the shape R31-1's `$`-header control took successfully — but
  deciding that EVERY schema-shaped non-table is a miss is a design question, not a measure fix: a
  memo may legitimately show a table's shape in prose, and this checker's own documents do.
  ⚠ **What the revert cost is known and small**: the arm reported **0** files over the 141-memo
  corpus, so nothing real is being missed today; what is missing is the gate.
  **Scope**: the lost-declaration gate's second arm, with a population defined by Phase-1 block
  context, and a decision on whether a schema-shaped non-table is a miss everywhere or only in a
  LINKED memo (the reproduced case).
  **Owner**: Slice 2's plan-review, which owns this gate's other open question (R48-1) — the two
  should be decided together, since both are about what the gate's population IS.
  **Trigger**: already fired (reproduced above). **Re-eval: 2026-12-31.** No slot: it is this
  checker's own gate.
- **The HAND-WRITTEN TABLE has no detector** (PR #510 Axis 5, 2026-09-20 — **own** deferral).
  ⚠ **THIS ENTRY FIRST SAID "the class is now four deep" AND SCOPED THE CARVE BY THE SYMPTOM
  VOCABULARY** — "a module-level name bound to a container whose docstring or comment carries the
  'what it cannot see' form" — which selects exactly the tables that had already declared themselves
  and leaves every other one invisible BY CONSTRUCTION
  (`memory/feedback_checks-must-not-be-defined-by-the-symptom-vocabulary.md`, the rule this very
  entry cites). Measured by the STRUCTURAL property instead — a module-level name bound to a literal
  container or to a `frozenset` / `set` / `tuple` / `dict` / `list` constructor, over
  `.claude/tools/plan*.py` by `ast.parse` — the population is **dozens**, and ⚠ the exact figure
  depends on the predicate. **THE TWO NUMBERS THAT STOOD HERE — 59 and 38 — ARE WITHDRAWN, NOT
  CORRECTED** (PR #510 R45): re-derived from the predicate exactly as this entry words it they come
  out **97 / 84**, and no variant tried (uppercase-only names, excluding bare tuple literals,
  non-selftest modules, the thirteen-module set) reproduces 59 or 38. A number nobody can reach from
  the stated predicate is an argument dressed as a measurement — the very thing this paragraph cites
  `memory/feedback_convention-dependent-figures-are-argument.md` for, written two sentences after
  citing it. What is authoritative is the PREDICATE and the command that runs it, and the four below
  are a **SEED, not an inventory**. The four that fired so far:
  `_IMPORT_SEAMS` (which import seams are checked), `_ATTRIB_SPELLINGS` (which attribution spellings
  are read), `_TRAILING` (which trailing characters are decoration) and `_ID_SPELLINGS` (which id
  character classes the sweep knows). Every one states its own "HONESTLY, what it cannot see" and
  every one of those sentences says the same thing: *a shape this table does not list*. The
  INSTANCES found so far are fixed; the CLASS has no mechanism, and
  `memory/feedback_declared-blind-spots-are-where-the-next-finding-lands.md` says a declared blind
  spot is a map of the next finding. ⚠ It is now MEASURED rather than predicted: the touch-time split
  one commit before this entry had to widen `_IMPORT_SEAMS` — the first edit in this PR's life that
  had to move that table rather than a prose sentence — and it landed inside the very territory the
  previous round had declared undetected.
  **Scope**: one control whose population is the STRUCTURAL set above (every module-level name bound
  to a literal container or such a constructor, read off the AST — never a grep for the declaration
  sentence), asserting that each table a control READS has a stated POPULATION and a stated
  COMPLEMENT — i.e. that the table's own reach is derived, not asserted. The "what it cannot see"
  sentence is the ASSERTION the control checks for, never the selector that decides who is checked. ⚠ **Not a one-liner, and that is why it is carved rather than written here**: the
  question "is this table's complement measured" is itself a claim about a complement, so the control
  can be written to pass vacuously, which is the failure
  `memory/feedback_control-rewritten-to-bless-the-defect.md` names. It needs a negative control that
  is RED before the mechanism exists.
  **Owner**: this checker's own self-test, not a platform gap — no slot.
  **Trigger (an EVENT)**: the next round that reports a miss traceable to one of the four tables, or
  the fifth table. **Re-eval: 2026-12-31.**
