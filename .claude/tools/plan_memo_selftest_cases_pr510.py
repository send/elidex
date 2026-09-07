#!/usr/bin/env python3
"""PR #510 converge-round controls for `plan-memo-umbrella-check.py --self-test`.

The second half of the control registry, split from `plan_memo_selftest_cases.py`
at the review-round seam: that module holds the fixture builder, the record
shape and the pre-converge controls (the four kinds, the assertion / lexing /
exit-status / `/code-review high` / `/elidex-review` Stage 6 families); this one
holds every control written against a PR #510 review round (Codex R1-R13 and
the design re-gate over R4-R9), indexed by round, and appends to the SAME
`CASES` list through the same `case` / `acase` / `rcase` spellings -- one
registry, one import site (the runner imports this module for its side
effect).  A control's mutant lives in `plan_memo_selftest_mutants.py` under the
same round label.
"""

from plan_memo_selftest_cases import SIB, VIOLATION, acase, build, case, rcase

# ------------------------------------------------- PR #510 Codex R1 controls --

# R1-1: §2.4 parity in the row split -- only an ODD backslash run escapes `|`
SLOT4 = ("| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
         "| `#11-zz-gamma` | Terminal. Acceptance: must. | %s | 2026-12-31 |")
rcase("POSITIVE", "(row) `a\\\\|b` holds an UNESCAPED pipe (§2.4: `\\\\` is a literal backslash): "
                  "5 cells under a 4-cell header is a width miss, rc 2",
      build(extra=SLOT4 % "a\\\\|b"), "", 2)
rcase("NEGATIVE", "(row) `a\\|b` is one cell (the odd backslash escapes the pipe): rc 0",
      build(extra=SLOT4 % "a\\|b"), "", 0)
rcase("NEGATIVE", "(row) a trailing `\\\\|` is a literal backslash then the trailing pipe: rc 0",
      build(extra=(SLOT4 % "now")[:-1] + "\\\\|"), "", 0)

# R1-3: §6.3 -- links may not contain links; the inner-most link is the one
NEST = {"child.md": VIOLATION + "\n"}
case("POSITIVE-NOVEL", "(link) nested inline links: the INNER link is the link, the outer tail is "
                       "text -- `child.md` joins the population, absent `parent.md` is not linked",
     build(), "See [outer [child](child.md)](parent.md).", 1, files=NEST)
case("POSITIVE-NOVEL", "(link) a reference link nested in inline brackets: the inner reference is "
                       "the link, the outer tail is text",
     build(), "See [outer [child][c]](parent.md).\n\n[c]: child.md", 1, files=NEST)

# R1-4: an orphan definition split over its permitted continuation line is
# still an orphan (§4.7 invalidates it: it interrupts a paragraph), so the
# shortcut that names it is unanswered
case("POSITIVE", "(def) a would-be MULTILINE definition that interrupts a paragraph is an orphan: the "
                 "shortcut naming it is a schema miss, not an exempt citation-style shortcut",
     build(), "text\n[sib]:\nslice-9z-sib.md\nSee [sib]", 1, **SIB,
     measure=("schema", "unresolved reference 'sib'"))


# ------------------------------------------------- PR #510 Codex R2 controls --

# R2-1: §4.7 -- the title may sit on the line after the destination
case("NEGATIVE", "(def) a next-line title is part of the definition, not prose: an id in it is no site",
     build(), '[sib]: slice-9z-sib.md\n"9z owns it"\n\nSee [sib].', 0)
rcase("NEGATIVE", "(def) a next-line title holding `[x](missing.md)` is a title, not a link: rc 0",
      build(), '[sib]: slice-9z-sib.md\n"see [x](missing.md)"\n\nSee [sib].', 0)
case("POSITIVE", "(def) a next line that is NOT a valid title is prose (the definition ends at the "
                 "destination)",
     build(), '[sib]: slice-9z-sib.md\n"9z owns it\n\nSee [sib].', 1)

# R2-2: §6.4 -- an image is not a link, and a link may wrap one
case("POSITIVE-NOVEL", "(link) a link wrapping an IMAGE `[![alt](img.png)](sib.md)` links the sibling; "
                       "`img.png` is never a memo",
     build(), "See [![alt](img.png)](slice-9z-sib.md).", 1, **SIB)

# R2-3: the reference FORM comes from the lexer's escape-honouring parse
case("POSITIVE", "(link) `[foo\\]][missing]` is a FULL reference (the `]` is escaped): a schema miss, "
                 "not an exempt shortcut",
     build(), "See [foo\\]][missing].", 1, measure=("schema", "unresolved reference 'missing'"))


# ------------------------------------------------- PR #510 Codex R3 controls --
# `links()` is CommonMark "Appendix: A parsing strategy" bracket stack (one pass, no re-parse).

case("NEGATIVE", "(image) `![alt][img]` with a definition is consumed whole: `[img]` is not re-read "
                 "as a shortcut, and the image destination is not a memo",
     build(), "See ![alt][img] here.\n\n[img]: absent-file.md", 0,
     measure=("schema", "unresolved reference"))
case("POSITIVE-NOVEL", "(link) a link wrapping a REFERENCE image `[![alt][img]](child.md)`: the image "
                       "does not deactivate the outer opener, so `child.md` is scanned",
     build(), "See [![alt][img]](child.md).\n\n[img]: pic.png", 1,
     files={"child.md": VIOLATION + "\n"})
case("NEGATIVE", "(link) an escaped `\\[` opens nothing: `\\[x](absent-file.md)` is not a link, rc 0",
     build(), "See \\[x](absent-file.md) here.", 0, measure=("schema", "linked memo not found"))

# R3-2: a root-relative destination is a site URL, never a sibling on disk
rcase("NEGATIVE", "(rc) a root-relative `/guide.md` is not a sibling on disk (nothing probed): rc 0",
      build(), "See [site docs](/guide.md).", 0)

# re-gate MIN-3: one label grammar (`link_label`) decides what a shortcut label is
rcase("NEGATIVE", "(link) bracket text holding unescaped brackets is not a label (§6.3), so "
                  "`[the [x] walk][]` is no collapsed reference: rc 0",
      build(), "See [the [x] walk][] here.", 0)


# ------------------------------------------------- PR #510 Codex R4 controls --

# R4-1: an undefined reference-style IMAGE is literal image syntax (§6.4), never a memo miss
rcase("NEGATIVE", "(image) an undefined reference image `![diagram][missing-image]` is literal syntax, "
                  "not an unresolved memo reference: rc 0",
      build(), "See ![diagram][missing-image] here.", 0)

# R4-2: a percent-encoded destination names the decoded file
case("POSITIVE-NOVEL", "(link) a percent-encoded destination `slice%20sib.md` links the file "
                       "`slice sib.md`, as `<slice sib.md>` does",
     build(), "See [the walk](slice%20sib.md).", 1, files={"slice sib.md": VIOLATION + "\n"})

# R4-3: code spans and brackets are ONE inline pass (Appendix A); the inline
# tail is parsed by lookahead on the raw text
case("POSITIVE-NOVEL", "(span) a backtick inside a link DESTINATION is consumed by the link, not a "
                       "code span: `[sib](slice`x`.md)` links the sibling",
     build(), "See [the walk](slice`x`.md).", 1, files={"slice`x`.md": VIOLATION + "\n"})
rcase("NEGATIVE", "(span) a backtick BEFORE the `]` opens a code span that swallows it: "
                  "`[not a `link](absent.md)`` is code, no link, rc 0",
      build(), "See [not a `link](absent-file.md)` here.", 0)


# ------------------------------------------------- PR #510 Codex R5 controls --

# R5-1/4: Phase 1 (block structure) owns reference definitions and block ends
case("POSITIVE-NOVEL", "(def) a definition is read from RAW lines at a block start: `[sib]: slice`x`.md` "
                       "keeps its backticks in the destination and the sibling is scanned",
     build(), "[sib]: slice`x`.md\n\nSee [sib].", 1, files={"slice`x`.md": VIOLATION + "\n"})
rcase("POSITIVE", "(table) a reference definition right after a schema table is a ROW of it (GFM "
                  "Example 202: a pipe-less line after the rows is a row; §4.7: a definition cannot "
                  "interrupt a block) -- one cell under a 4-cell header, width miss rc 2",
      build(extra=SLOT4 % "now" + "\n[sib]: slice-9z-sib.md"), "See [sib].", 2, **SIB)

# R5-2: the decoded path is re-validated
rcase("NEGATIVE", "(rc) a percent-encoded ABSOLUTE destination `%2Ftmp%2Fchild.md` is rejected after "
                  "decoding (never probes `/tmp/child.md`): rc 0",
      build(), "See [x](%2Ftmp%2Fchild.md).", 0)

# R5-3: declared ids are atomic tokens in an id-only run
case("POSITIVE", "(span) a `#11-` slug is ATOMIC in an id-only run: `` `#11-zz-alpha / 9z` `` is the "
                 "document spelling two ids, both reported",
     build(), "The same thing happened to `#11-zz-alpha / 9z`.", 2)


# ------------------------------------------------- PR #510 Codex R7 controls --

# R7-1: a definition is parsed over the rest of its block (§4.7: a label may
# span lines -- here five -- but not a blank line)
case("POSITIVE-NOVEL", "(def) a label spanning FIVE lines is a definition (§4.7 / §6.3: a label may span "
                       "lines); the later shortcut resolves and the sibling is scanned",
     build(), "[the\nfive\nline\nwalk\nlabel]: slice-9z-sib.md\n\nSee [the five line walk label].", 1, **SIB)
# R7-3: a title crossing a blank line is no title, so the line is no definition
# (§4.7 "may not contain a blank line"); the shape is an orphan and the later
# shortcut is a schema miss; child.md is never walked
# commonmark.js 0.31.2 decides this shape: `[sib]: child.md "title\n\nmore"` is
# a paragraph (`<p>[sib]: child.md "title</p><p>more"</p>`), not a definition,
# and `[sib]` later is literal text -- so it is an exempt shortcut (rc 0).  The
# load-bearing assertions stay: the sibling is NOT walked (its violation is not
# reported) and the text is not masked.
case("NEGATIVE", "(def) `[sib]: child.md \"title` whose title crosses a BLANK line is not a definition "
                 "(commonmark.js: a paragraph): `[sib]` later is an exempt shortcut, rc 0, and the sibling "
                 "is NOT walked -- its violation is not reported",
     build(), '[sib]: slice-9z-sib.md "title\n\nmore"\n\nSee [sib].', 0, **SIB)


# ------------------------------------------------- PR #510 Codex R8 controls --

# R8 root: one sibling-path resolver, stages in spec order
case("POSITIVE-NOVEL", "(link) `notes%3Achild.md` has no scheme (WHATWG URL: a scheme is read BEFORE "
                       "decoding): it is the local file `notes:child.md`, and it is scanned",
     build(), "See [the walk](notes%3Achild.md).", 1, files={"notes:child.md": VIOLATION + "\n"})

# R8-3: the far side of a `.` after a bare id is the ASCII id class
case("POSITIVE-NOVEL", "(bare) `9z.次の工程` bounds the id: the far side of the `.` is not an ASCII id "
                       "character, so the site is reported",
     build(), "The integrator is 9z.次の工程へ渡す.", 1)
case("POSITIVE-NOVEL", "(bare) `9z.é` bounds the id (a dotted number is ASCII on both sides)",
     build(), "The integrator is 9z.élu.", 1)

# R8-4: edge pipes are detected with the space/tab class, not `str.strip()`
case("NEGATIVE", "(table) a row opening with an NBSP before its `|` is not edge-piped: the NBSP is a "
                 "cell, the header is 7 wide over a 6-cell delimiter, no table",
     build(), "\u00a0| # | Slice | Primary module(s) | Slot | Tier | Deps |\n|---|---|---|---|---|---|\n"
              "| **Wz** | **UMBRELLA, not a terminal unit.** x | `w.rs` | — | T1 | — |\n\n"
              "Wz owns the close rule.", 0)
# sweep: §4.9 blank line = spaces or tabs only
case("NEGATIVE", "(span) an NBSP-only line is NOT blank (§4.9: spaces or tabs only), so it does not end "
                 "the paragraph and the code span crosses it",
     build(), "open `here\n\u00a0\n9z owns it` there", 0)


# ------------------------------------------------- PR #510 Codex R9 controls --

# FAMILY 1 (b): §4.3 setext headings end the paragraph (the reviewer's case)
case("POSITIVE-NOVEL", "(setext) `Heading\\n===` is a heading; the `===` underline ends the paragraph, "
                       "so a code span opened in the heading does not reach the next paragraph's site",
     build(), "Heading `open\n===\nSlice 9z owns it` here", 1)
case("POSITIVE", "(setext) the reviewer's case: `Heading\\n===\\nSlice `9z` owns it` reports the site",
     build(), "Heading\n===\nSlice `9z` owns it", 1)
case("POSITIVE", "(setext) a `---` after paragraph text is the heading's underline (§4.3 over §4.1, "
                 "Example 59) and ends the paragraph like the thematic break it is not",
     build(), "open `here\n---\n9z owns it` there", 1)
case("NEGATIVE", "(setext) `==` after a list item is NOT an underline (§4.3 Examples 92-94): the item's "
                 "paragraph continues and a code span crosses it",
     build(), "- a `x\n==\n9z owns it` end", 0)
# FAMILY 1 (c): the LEX-UNSUPPORTED? seed -- since R13 the raw lines of an HTML
# block only: a block quote is a container whose content is parsed (§5.1) and
# indented code a raw extent like a fence (§4.4), neither seeded
acase("POSITIVE", "(lex-seed) an HTML-block opener holding a declared id is a seed",
      build(), "LEX-UNSUPPORTED?", 1, prose="<div>9z owns it</div>")
acase("NEGATIVE", "(lex-seed) an HTML-block line with neither a `|` nor a declared id is no seed",
      build(), "LEX-UNSUPPORTED?", 0, prose="<div>\na remark about nothing in particular\n</div>")

# FAMILY 3: ASCII boundaries by property (the reviewer's cases)
case("POSITIVE-NOVEL", "(ascii) `次のSlice Cが所有する` reaches the naming worklist: the row-noun anchor is "
                       "not `\\b` (no Unicode word boundary before `Slice`)",
     build(), "次のSlice Cが所有する。", 1)
case("POSITIVE-NOVEL", "(ascii) `次は#11-zz-alphaが所有する` reaches the naming worklist: the slug anchor "
                       "is an ASCII class, not `\\w`",
     build(), "次は#11-zz-alphaが所有する。", 1)
case("NEGATIVE", "(ascii) `١.` (an Arabic-Indic digit) is not a list marker (§5.2: ASCII digits)",
     build(), "open `here\n\u0661. 9z owns it` there", 0)


# ------------------------------------------ design re-gate R4-R9 controls --
# Every block-structure expectation below was checked against commonmark.js
# 0.31.2 (`node cm.js '["<md>"]'`) before being written.

# IMP-1: ONE block-boundary rule -- a definition's continuation line cannot
# be a block start; the six probe shapes
# The label line carries a naming site: when the shape is NOT a definition the
# line is paragraph text and the site is reported (1); a definition is a block
# of its own, never scanned (0) -- so each control discriminates the two reads.
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n---` is a setext heading (`<h2>…</h2>`), not a "
                 "definition: the label line is paragraph text and its site is reported",
     build(), "[Slice 9z owns it]:\n---\n\nSee [foo].", 1)
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n#` is a paragraph and an ATX heading, not a definition",
     build(), "[Slice 9z owns it]:\n#\n\nSee [foo].", 1)
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n>` is a paragraph and a block quote, not a definition",
     build(), "[Slice 9z owns it]:\n>\n\nSee [foo].", 1)
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n***` is a paragraph and a thematic break, not a "
                 "definition",
     build(), "[Slice 9z owns it]:\n***\n\nSee [foo].", 1)
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n|a|b|\\n|--|--|` -- the table header ends the run "
                 "(local policy over pure CommonMark, which has no tables): a paragraph and a table, "
                 "not a definition with the header row as its destination",
     build(), "[Slice 9z owns it]:\n|a|b|\n|--|--|\n|c|d|\n\nSee [foo].", 1)
case("NEGATIVE", "(block) `[Slice 9z owns it]:\\n    code` IS a definition (§4.4: indented code cannot "
                 "interrupt a paragraph; commonmark.js: destination `code`): a block of its own, not "
                 "scanned",
     build(), "[Slice 9z owns it]:\n    code\n\nSee [foo].", 0)
rcase("NEGATIVE", "(table) a list item right after a schema table ends it (a block start), so it is "
                  "not a 1-cell body row: rc 0",
      build(extra=SLOT4 % "now" + "\n- an item"), "", 0)
case("POSITIVE-NOVEL", "(block) `[foo]:\\nslice-9z-sib.md` IS a definition (destination on the next "
                       "line): the sibling is walked",
     build(), "[foo]:\nslice-9z-sib.md\n\nSee [foo].", 1, **SIB)
case("POSITIVE", "(block) `[Slice 9z owns it]:\\n<div>` is a paragraph and an HTML block (type 6 "
                 "interrupts a paragraph), not a definition",
     build(), "[Slice 9z owns it]:\n<div>\n\nSee [foo].", 1)
case("POSITIVE", "(block) `[foo]:\\n- item`: the list item interrupts, so `[foo]:` is a paragraph and a "
                 "code span opened before the marker does not cross it",
     build(), "open `x\n- 9z owns it` end", 1)

# IMP-2: an orphan is a VALID definition that cannot take effect; a gloss is prose
rcase("NEGATIVE", "(cite) `[C1]: ECMA-262 §1 says so, and the table cites it.` at a block start is prose "
                  "(commonmark.js: a paragraph), and the citation shortcut stays exempt: rc 0",
      build(), "[C1]: ECMA-262 §1 says so, and the table cites it.\n\nPer [C1] the probe must return 3.", 0)
rcase("NEGATIVE", "(cite) the same gloss mid-paragraph is prose: rc 0",
      build(), "Notes follow.\n[C1]: ECMA-262 §1 says so.\n\nPer [C1] the probe must return 3.", 0)
rcase("NEGATIVE", "(def) a label-and-colon line that is NOT a valid definition (junk after the "
                  "destination) at a block start is prose, not an orphan: `[sib]` later is exempt, rc 0",
      build(), "[sib]: slice-9z-sib.md junk here\n\nSee [sib].", 0, **SIB)

# MIN-1: every line of an HTML block is seeded, to its §4.6 end condition
acase("POSITIVE", "(lex-seed) `<pre>\\nSlice 9z owns it\\n</pre>`: the inner line holding the id is "
                  "seeded (type 1 ends at `</pre>`)",
      build(), "LEX-UNSUPPORTED?", 1, prose="<pre>\nSlice 9z owns it\n</pre>")
acase("POSITIVE", "(lex-seed) `<!-- c\\n|9z|\\n-->`: a `|` line inside a comment block (type 2 ends at "
                  "`-->`) is seeded",
      build(), "LEX-UNSUPPORTED?", 1, prose="<!-- c\n|9z| x\n-->")
acase("NEGATIVE", "(lex-seed) a type-6 block ends at a blank line: the paragraph after it is not seeded",
      build(), "LEX-UNSUPPORTED?", 0, prose="<div>\nplain\n\nSlice 9z owns it")


# ------------------------------------------------ PR #510 Codex R10 controls --

# #1: an HTML block is a RAW extent like a fence (the reviewer's input)
# The reviewer's exact input (`<pre>\n`\n</pre>\nSlice `9z` owns it`) reports
# the site under BOTH readings (the raw backtick pairs with the one before
# `9z`, leaving `9z` outside the span), so the discriminating shape puts the
# prose backtick AFTER the id: a paragraph reading swallows `9z`, the raw
# reading leaves it.
case("POSITIVE", "(html) `<pre>\\n`\\n</pre>\\nSlice 9z` owns it`: the backtick inside the raw HTML "
                 "block does not pair with the prose one -- the site is reported",
     build(), "<pre>\n`\n</pre>\nSlice 9z` owns it", 1)
case("POSITIVE", "(html) the reviewer's input `<pre>\\n`\\n</pre>\\nSlice `9z` owns it` reports the site",
     build(), "<pre>\n`\n</pre>\nSlice `9z` owns it", 1)
case("POSITIVE", "(html) `text\\n<span>\\n9z owns it` -- a type-7 opener cannot interrupt a paragraph "
                 "(commonmark.js): the lines stay paragraph text and the site is reported",
     build(), "text\n<span>\n9z owns it", 1)
case("NEGATIVE", "(html) `<span>\\n9z owns it` at a block start IS a type-7 block to the blank line: raw",
     build(), "<span>\n9z owns it\n\nafter", 0)
case("POSITIVE", "(html) `<div>\\nx\\n</div>\\ny` -- a type-6 block runs to the BLANK line, not to "
                 "`</div>`: `y` is raw, the paragraph after the blank is prose",
     build(), "<div>\nx\n</div>\n9z is raw here\n\n9z owns it", 1)
case("POSITIVE", "(html) `<pre></pre>\\n9z owns it` -- the opener meets the end condition itself, so the "
                 "block is that one line and the next line is prose",
     build(), "<pre></pre>\n9z owns it", 1)

# #2: an accepted id cell with trailing prose is scanned, the row's own id suppressed
case("POSITIVE", "(id) the id cell's trailing prose is scanned: `**7z** — Slice 9z lands first` reports "
                 "`9z` (the row's own `7z` is suppressed)",
     build(i7z="**7z** — Slice 9z lands first"), "", 1)
case("NEGATIVE", "(id) the id cell's own id is not a site: `**7z** — MERGED` reports nothing",
     build(i7z="**7z** — MERGED"), "", 0)

# #3: a bare `.md` file name is read by path syntax
case("NEGATIVE", "(file) `9z+notes.md` is one file name: no site",
     build(), "Read 9z+notes.md for the walk.", 0)
case("NEGATIVE", "(file) `9z@notes.md` is one file name: no site",
     build(), "Read 9z@notes.md for the walk.", 0)
case("NEGATIVE", "(file) `(9z).md` is one file name (a balanced parenthesis pair): no site",
     build(), "Read (9z).md for the walk.", 0)
case("POSITIVE", "(file) a link's visible text is not swallowed into the destination token: "
                 "`[Slice 9z](slice-9z-sib.md)` still reports `9z`",
     build(), "See [Slice 9z](slice-9z-sib.md) for the walk.", 1)


# ------------------------------------------------ PR #510 Codex R11 controls --
# Every block-structure expectation below was checked against commonmark.js
# 0.31.2 (`node cm.js '["<md>"]'`) before being written.

# #1: "is this line at a block start" is the driver's block STATE (no run
# open), never a look at the previous raw line.  A table is not a paragraph
# (GFM §4.10: broken at "beginning of another block-level structure"), so a
# type-7 opener right after its rows opens an HTML block.
rcase("NEGATIVE", "(table) a type-7 HTML opener right after a schema table ENDS it (a table is not a "
                  "paragraph): `<span>` + prose are raw lines, not one-cell rows -- no width miss, rc 0",
      build(extra=SLOT4 % "now" + "\n<span>\n9z owns it"), "", 0)
case("NEGATIVE", "(table) the prose inside the type-7 block after a schema table is raw, not a site",
     build(extra=SLOT4 % "now" + "\n<span>\n9z owns it"), "", 0)
acase("POSITIVE", "(lex-seed) the raw line holding a declared id inside the type-7 block after a table "
                  "is seeded",
      build(extra=SLOT4 % "now" + "\n<span>\n9z owns it"), "LEX-UNSUPPORTED?", 1)
case("NEGATIVE", "(html) `# h\\n<span>\\n9z owns it`: after a one-line block no paragraph is open, so the "
                 "type-7 opener is a block start (commonmark.js: a heading and a raw block): no site",
     build(), "# h\n<span>\n9z owns it", 0)
case("NEGATIVE", "(html) `Heading\\n===\\n<span>\\n9z owns it`: after a setext heading no paragraph is open "
                 "(commonmark.js: a heading and a raw block) -- the look-back at the previous line missed "
                 "this: no site",
     build(), "Heading\n===\n<span>\n9z owns it", 0)
case("POSITIVE", "(html) `[sib]: slice-9z-sib.md\\n<span>\\n9z owns it`: a definition keeps the paragraph "
                 "open (§4.7; commonmark.js: `<span>` is paragraph text), so the site is reported",
     build(), "[sib]: slice-9z-sib.md\n<span>\n9z owns it", 1)
case("POSITIVE", "(html) `- item\\n<span>\\n9z owns it`: the item's paragraph is open (lazy continuation), "
                 "so type 7 does not open a block: paragraph text, the site is reported",
     build(), "- item\n<span>\n9z owns it", 1)

# #2: a failed reference is recorded ONCE; its label bracket is re-scanned
# (§6.3 Example 571) but the tail is never consumed
rcase("NEGATIVE", "(image) the reviewer's input `![alt][missing]` + an orphan `[missing]: image.md` in the "
                  "same paragraph: the re-scanned `[missing]` is the same failed site, not a shortcut for "
                  "the orphan rule -- rc 0, `image.md` never walked",
      build(), "See ![alt][missing] here.\n[missing]: image.md", 0)
case("POSITIVE-NOVEL", "(image) `![alt][missing][Slice 9z]` with `[Slice 9z]` defined: the failed image's "
                       "label is re-scanned and `[missing][Slice 9z]` is a link (§6.3 Example 571; "
                       "commonmark.js) -- its tail is masked, the sibling is walked: 1 site, not 2",
     build(), "See ![alt][missing][Slice 9z] here.\n\n[Slice 9z]: slice-9z-sib.md", 1, **SIB)


# ------------------------------------------------ PR #510 Codex R12 controls --

# A: a setext underline is a boundary only where a paragraph is open.  After
# a table's rows no paragraph is open (a table is not a paragraph), so `===`
# is nothing to underline with -- GFM §4.10 reads a pipe-less line as a row
# (Example 202) -- while `---` is the beginning of a thematic break, "another
# block-level structure", which ends the table.  commonmark.js cannot decide
# this (no tables: it reads the rows as a paragraph and both lines as its
# underline); cmark-gfm continues a table on `===` and finalises it before a
# thematic break, and that is the local policy stated here.
rcase("POSITIVE", "(table) `===` right after a schema table is a one-cell body row (§4.3: no paragraph to "
                  "underline; GFM §4.10 Example 202: a pipe-less line is a row): width miss, rc 2",
      build(extra=SLOT4 % "now" + "\n==="), "", 2)
rcase("NEGATIVE", "(table) `---` right after a schema table is a thematic break (§4.1: no paragraph is open "
                  "so §4.3 does not apply; GFM §4.10: another block-level structure ends the table): rc 0",
      build(extra=SLOT4 % "now" + "\n---"), "", 0)
case("POSITIVE", "(setext) a bare `===` IS a paragraph, so `===\\n---` is the heading `<h2>===</h2>` "
                 "(commonmark.js) and the code span opened in it does not reach the next site",
     build(), "=== `open\n---\nSlice 9z owns it` here", 1)

# B: §2.2 -- indentation is measured in columns, a tab to the next multiple of 4
# (since R13 an indented line at a block start is a RAW extent, so the measure
# decides whether the site exists at all)
case("NEGATIVE", "(indented) `\\tSlice 9z owns it`: a tab is four columns (§2.2), so the line is indented code at "
                 "a block start -- a raw extent, no site",
     build(), "\tSlice 9z owns it", 0)
case("NEGATIVE", "(indented) ` \\tSlice 9z owns it`: a space then a tab is four columns (§2.2) -- raw, no site",
     build(), " \tSlice 9z owns it", 0)
case("POSITIVE", "(indented) `   Slice 9z owns it`: three spaces are three columns -- paragraph text, the site "
                 "is reported",
     build(), "   Slice 9z owns it", 1)
rcase("NEGATIVE", "(table) `\\t| Slot | ... |` over `\\t|---|...|`: a tab-indented header and delimiter row "
                  "are indented code (§4.4, four columns), not a table -- the wide row under them is no "
                  "width miss: rc 0",
      build(extra="\n".join("\t" + l for l in (SLOT4 % "now").split("\n")) + "\n| a | b | c | d | e |"), "", 0)

# C: the anchored pass reads the disposed stream, so a match never straddles a
# mask boundary
case("NEGATIVE", "(anchor) `` `Slice `C owns it ``: the row noun is inside a code span, so on the disposed "
                 "stream there is no `Slice C` to anchor on -- 0 sites (the bare `C` is a declared miss)",
     build(), "The `Slice `C owns it.", 0)
case("POSITIVE", "(anchor) `` Slice `C` owns it ``: an id-only code span stands in the stream, so the "
                 "anchored `Slice C` is still a site",
     build(), "Slice `C` owns it.", 1)


# ------------------------------------------------ PR #510 Codex R13 controls --
# Every block-structure expectation below was checked against commonmark.js
# 0.31.2 (`node cm.js '["<md>"]'`) before being written; the shapes no site
# can discriminate (which block a line lands in) are the runner's
# block-sequence control.

# THE ROOT: §4.4 indented code is a RAW extent (one opener / extent rule with
# fences and HTML blocks), §5.1 block quotes are CONTAINERS (the same Phase 1
# over the content).  The 27 conformance exclusions were exactly these.
case("NEGATIVE", "(indented) `    Slice 9z owns it` at a block start is a raw extent (§4.4), like a fence: no site",
     build(), "    Slice 9z owns it", 0)
case("POSITIVE", "(indented) `text\\n    Slice 9z owns it`: an indented line cannot interrupt a paragraph (§4.4; "
                 "commonmark.js: one paragraph) -- paragraph text, the site is reported",
     build(), "text\n    Slice 9z owns it", 1)
rcase("NEGATIVE", "(table) an indented line right after a schema table opens an indented code block (a block-level "
                  "structure; no paragraph is open), not a one-cell row: rc 0",
      build(extra=SLOT4 % "now" + "\n    Slice 9z owns it"), "", 0)
case("POSITIVE-NOVEL", "(quote) `> [sib]: slice-9z-sib.md`: a definition inside a block quote registers (§5.1 "
                       "container, Example 218) -- the sibling is walked and its violation reported",
     build(), "> [sib]: slice-9z-sib.md\n\nSee [sib].", 1, **SIB)
case("POSITIVE-NOVEL", "(quote) a slot table inside a block quote is a table: its row declares its id",
     build(extra="\n".join("> " + l for l in (SLOT4 % "now").split("\n"))), "", 1,
     measure=("id", "#11-zz-gamma"))
case("POSITIVE", "(quote) `> Slice 9z owns it`: the quote's paragraph is scanned at its real line and the "
                 "marker is not content -- the site is reported",
     build(), "> Slice 9z owns it, says the quote.", 1)
case("NEGATIVE", "(quote) `> open `here\\nSlice 9z owns it` there`: the marker-less line is lazy continuation "
                 "text of the quote's paragraph (§5.1), so the span crosses it: no site",
     build(), "> open `here\nSlice 9z owns it` there", 0)
case("NEGATIVE", "(quote) `> Heading `open\\n===\\nSlice 9z owns it` here`: a lazy `===` is the quote "
                 "paragraph's text, not an underline (§5.1, Example 93) -- one paragraph, the span masks the "
                 "site",
     build(), "> Heading `open\n===\nSlice 9z owns it` here", 0)
case("POSITIVE", "(quote) `> open `x\\n# Slice 9z owns it` there`: an ATX heading is never lazy -- the quote "
                 "ends, the heading is a block of its own and its site is reported",
     build(), "> open `x\n# Slice 9z owns it` there", 1)

# #3: GFM §4.10 "If greater, the excess is ignored" -- a non-schema row's
# excess cells never enter the lexical population
case("NEGATIVE", "(table) an excess body cell of a non-schema table is ignored (GFM §4.10): its `9z owns it` "
                 "is never lexed -- no site",
     build(extra="| a | b |\n|---|---|\n| 1 | 2 | 9z owns it |"), "", 0)
rcase("NEGATIVE", "(rc) an excess body cell of a non-schema table holding `[x](absent-file.md)` is ignored (GFM "
                  "§4.10): no link, nothing walked, rc 0",
      build(extra="| a | b |\n|---|---|\n| 1 | 2 | [x](absent-file.md) |"), "", 0)
case("POSITIVE", "(table) the cells within the header's width of a non-schema row are scanned: `9z owns it` in "
                 "the second of three cells under a two-cell header is a site",
     build(extra="| a | b |\n|---|---|\n| 1 | 9z owns it | extra |"), "", 1)

# #4: §4.6 start conditions read case PER CONDITION
case("POSITIVE", "(html) `<![cdata[` is no CDATA opener (§4.6 condition 5 is exact; commonmark.js: a paragraph): "
                 "the next line is prose and the site is reported",
     build(), "<![cdata[\n9z owns it\n]]>", 1)
case("NEGATIVE", "(html) `<![CDATA[` opens a type-5 block to `]]>` (Example 190): raw, no site",
     build(), "<![CDATA[\n9z owns it\n]]>", 0)
# ⚠ the probes interrupt a PARAGRAPH: at a block start `<PRE>` / `<DIV>` are
# type-7 openers too (an open tag alone on its line), which the case fold does
# not decide -- the first R13 probes sat at a block start and their mutants
# survived; types 1 and 6 interrupt, type 7 cannot (commonmark.js:
# `text\n<PRE>\nx\n</pre>` and `text\n<DIV>\nx` are a paragraph and a raw block)
case("NEGATIVE", "(html) `text\\n<PRE>\\n9z owns it\\n</pre>`: `<PRE>` opens a type-1 block (§4.6 condition 1 is "
                 "case-insensitive), which interrupts the paragraph -- raw to `</pre>`, no site",
     build(), "text\n<PRE>\n9z owns it\n</pre>", 0)
case("NEGATIVE", "(html) `text\\n<DIV>\\n9z owns it`: `<DIV>` opens a type-6 block (§4.6 condition 6 is "
                 "case-insensitive), which interrupts the paragraph -- raw to the blank line, no site",
     build(), "text\n<DIV>\n9z owns it", 0)

# LEXED-FLAT's one hidden-prose class is seeded: indented code right after a
# list item's paragraph (§5.2 Example 108 -- CommonMark reads the item's
# next paragraph; the umbrella memo's 2955 chunk is this shape)
acase("POSITIVE", "(lex-seed) `- item\\n\\n    Slice 9z owns it`: an indented line after a list item's paragraph "
                  "is the item's content under CommonMark (Example 108) and indented code under LEXED-FLAT -- "
                  "seeded",
      build(), "LEX-UNSUPPORTED?", 1, prose="- item\n\n    Slice 9z owns it")
acase("NEGATIVE", "(lex-seed) `para\\n\\n    Slice 9z owns it`: indented code after a plain paragraph is a code "
                  "block under CommonMark too -- raw, no seed",
      build(), "LEX-UNSUPPORTED?", 0, prose="para\n\n    Slice 9z owns it")
case("NEGATIVE", "(html) `<!doctype` opens a type-4 block (§4.6 condition 4: `<!` + an ASCII letter, either "
                 "case) to the `>` line: raw, no site",
     build(), "<!doctype\n9z owns it\n>", 0)
