#!/usr/bin/env python3
"""PR #510 converge-round controls for `plan-memo-umbrella-check.py --self-test`.

The second half of the control registry, split from `plan_memo_selftest_cases.py`
at the review-round seam: that module holds the fixture builder, the record
shape and the pre-converge controls (the four kinds, the assertion / lexing /
exit-status / `/code-review high` / `/elidex-review` Stage 6 families); this one
holds every control written against a PR #510 review round (Codex R1-R16 and
the design re-gates over R4-R9), indexed by round, and appends to the SAME
`CASES` list through the same `case` / `acase` / `rcase` spellings -- one
registry, one import site (the controls module imports this module for its
side effect).  A control's mutant lives in `plan_memo_selftest_mutants.py` under the
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
case("POSITIVE-NOVEL", "(link) `notes%3Achild.md` has no scheme (WHATWG URL §4.4 #scheme-start-state / "
                       "#scheme-state read the input as written and `%` is in neither class; "
                       "#string-percent-decode is a later, separate operation): it is the local file "
                       "`notes:child.md`, and it is scanned",
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
# can discriminate (which block a line lands in) are the controls module's
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

# #3: GFM §4.10 "If there are greater, the excess is ignored" -- a non-schema row's
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
case("NEGATIVE", "(html) `<![CDATA[` opens a type-5 block to `]]>` (Example 182): raw, no site",
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

# design re-gate 3 IMP-2: ONE seed rule for raw lines -- an indented-code line
# holding a `|` or a declared id is seeded like a raw HTML line, with its reading
acase("POSITIVE", "(lex-seed) `para\\n\\n    Slice 9z owns it`: indented code after a plain paragraph is raw under "
                  "CommonMark too, but holds a declared id -- seeded by the one raw-line rule",
      build(), "LEX-UNSUPPORTED?", 1, prose="para\n\n    Slice 9z owns it")
acase("NEGATIVE", "(lex-seed) an indented-code line with neither a `|` nor a declared id is no seed",
      build(), "LEX-UNSUPPORTED?", 0, prose="para\n\n    a remark about nothing in particular")
acase("POSITIVE", "(lex-seed) a 4-space-indented slot row after a table's rows is raw (cmark-gfm: `<pre><code>`) "
                  "and is SEEDED -- it holds a `|` -- never a silent skip (I-C)",
      build(extra=SLOT4 % "now" + "\n    | `#11-zz-delta` | Terminal. Acceptance: must. | now | 2026-12-31 |"),
      "LEX-UNSUPPORTED?", 1)
acase("POSITIVE", "(lex-seed) a tab-indented slot row after a table's rows is raw (§2.2: four columns) and is SEEDED",
      build(extra=SLOT4 % "now" + "\n\t| `#11-zz-delta` | Terminal. Acceptance: must. | now | 2026-12-31 |"),
      "LEX-UNSUPPORTED?", 1)
rcase("NEGATIVE", "(rc) an indented slot row after a table's rows is raw, so its id is not declared and the run is "
                  "rc 0 (the seed, not the census, carries it)",
      build(extra=SLOT4 % "now" + "\n    | `#11-zz-delta` | Terminal. Acceptance: must. | now | 2026-12-31 |"), "", 0)
# design re-gate 3 MIN-9: a lazy HEADER row is the table's header where a
# paragraph is open (cmark-gfm reads it out of the paragraph's last line);
# a lazy DELIMITER row opens nothing
case("POSITIVE-NOVEL", "(quote) a slot table inside a block quote whose HEADER row is lazy is a table (cmark-gfm: "
                       "the header is read out of the quote paragraph's last line): its row declares its id",
     build(extra="> para\n" + "\n".join(("> " if k else "") + l for k, l in enumerate((SLOT4 % "now").split("\n")))),
     "", 1, measure=("id", "#11-zz-gamma"))
case("NEGATIVE", "(quote) a slot table inside a block quote whose DELIMITER row is lazy is one paragraph (cmark-gfm: "
                 "the delimiter arrives in an unmatched container): no id declared",
     build(extra="\n".join(("" if k == 1 else "> ") + l for k, l in enumerate((SLOT4 % "now").split("\n")))),
     "", 0, measure=("id", "#11-zz-gamma"))
case("NEGATIVE", "(html) `<!doctype` opens a type-4 block (§4.6 condition 4: `<!` + an ASCII letter, either "
                 "case) to the `>` line: raw, no site",
     build(), "<!doctype\n9z owns it\n>", 0)


# ------------------------------------------------ PR #510 Codex R14 controls --
# ONE id-token grammar (`plan_memo_ids.tokens`) for every reader; the boundary
# is spelled once, and each reader below is a former second spelling of it.

# #1: the raw-line seed reads the grammar -- a hyphen bounds a short id on a
# raw line exactly as in prose (the seed's own regex rejected it on either side)
case("POSITIVE-NOVEL", "(lex-seed) a raw HTML line `9z-owner`: a hyphen bounds the short id on the raw line as in "
                       "prose, so the line is seeded holding `9z`",
     build(), "<div>9z-owner</div>", 1, measure=("seed", "'9z'"))
case("POSITIVE-NOVEL", "(lex-seed) a raw HTML line `owner-9z`: the hyphen bounds on the left too -- seeded holding `9z`",
     build(), "<div>owner-9z</div>", 1, measure=("seed", "'9z'"))
# #2: a slug is bounded on BOTH sides by the complement of its continuation
# class (`_` and `-` included), in prose and inside a code span
case("NEGATIVE", "(slug) `#11-zz-alphaZZ` in prose is not the id `#11-zz-alpha`: the slug is bounded on its right "
                 "as on its left -- 0 sites",
     build(), "The slot #11-zz-alphaZZ owns nothing here.", 0)
case("NEGATIVE", "(slug) `` `tool #11-zz-alpha_extra` ``: the kept-slug exception inside a code span reads the "
                 "grammar's boundary, so nothing is excepted and the span stays code -- 0 sites",
     build(), "Run `tool #11-zz-alpha_extra` before anything else.", 0)
case("POSITIVE", "(slug) `` `tool #11-zz-alpha` `` inside a code span IS the kept slug, bounded by the closing "
                 "backtick -- 1 site (the exception the control above must not widen)",
     build(), "Run `tool #11-zz-alpha` before anything else.", 1)
# #4: the citation mask and the reference walk's citation exemption are ONE
# predicate, either case: `[c1]` is `[C1]` under §6.3 label matching
case("NEGATIVE", "(cite) `[c1]` beside a declared no-owner id `c1`: a lowercase citation is masked by the same "
                 "predicate the reference walk exempts it by -- 0 sites",
     build(i7z="**c1**", s7z="**UMBRELLA, not a terminal unit.** x"), "Per [c1] the probe must return 3.", 0)
rcase("NEGATIVE", "(cite) adjacent lowercase citations `[c1][c2]` are not a full reference (the exemption is the "
                  "grammar's either-case predicate): rc 0",
      build(), "Per [c1][c2] step 1 the probe must return 3.", 0)
# #3's negative half: the orphan's OWN bracket stays exempt (the positive half
# is the controls module's `orphan_offset_control`)
rcase("NEGATIVE", "(def) `paragraph\\n[sib]: slice-9z-sib.md` alone: the orphan's own label bracket is exempt -- "
                  "rc 0, nothing walked",
      build(), "paragraph\n[sib]: slice-9z-sib.md", 0, **SIB)


# ------------------------------------------------ PR #510 Codex R15 controls --
# THE ROOT (#1): §5.2 list items are CONTAINERS, exactly like block quotes --
# the LEXED-FLAT reading, its item-open bit and the `item` seed reading are
# gone.  Every block-structure expectation below was checked against
# commonmark.js 0.31.2 (`node cm.js '["<md>"]'`) before being written; the
# shapes no site can discriminate (which block a line lands in, a list's
# tightness) are the controls module's block-sequence control, and the spec's own
# `List items` / `Lists` examples (253-326) are the conformance control's.
CHILD = {"child.md": VIOLATION + "\n"}
case("POSITIVE-NOVEL", "(item) the reviewer's input `- item\\n\\n    [child](child.md)`: the indented line is the "
                       "item's SECOND paragraph (§5.2, Example 108: the content indentation is 2), not indented "
                       "code -- the link is found, `child.md` is walked and its violation reported",
     build(), "- item\n\n    [child](child.md)", 1, files=CHILD)
case("POSITIVE", "(item) `- item\\n\\n    Slice 9z owns it`: the item's second paragraph is prose -- the site is "
                 "reported (until R15 the line was indented code, seeded and never scanned)",
     build(), "- item\n\n    Slice 9z owns it", 1)
acase("NEGATIVE", "(item) `- item\\n\\n    Slice 9z owns it` is no raw line: the `item` seed reading is gone with "
                  "the flat reading -- 0 seeds",
      build(), "LEX-UNSUPPORTED?", 0, prose="- item\n\n    Slice 9z owns it")
case("POSITIVE", "(item) `1. item\\n\\n     Slice 9z owns it`: an ordered item's content indentation is 3 (`1. `), "
                 "so five columns are two inside it -- the item's second paragraph, the site is reported",
     build(), "1. item\n\n     Slice 9z owns it", 1)
case("POSITIVE", "(item) `- item\\n\\n  para\\n\\n    Slice 9z owns it`: the item's THIRD paragraph (its second, "
                 "at the content indentation, keeps it open) -- the site is reported",
     build(), "- item\n\n  para\n\n    Slice 9z owns it", 1)
case("POSITIVE", "(item) `- item\\n\\n  > q\\n\\n    Slice 9z owns it`: a block quote inside the item, then the "
                 "item's paragraph -- the site is reported",
     build(), "- item\n\n  > q\n\n    Slice 9z owns it", 1)
case("NEGATIVE", "(item) `- item\\n\\n para\\n\\n    Slice 9z owns it`: a 1-column line after a blank is short of "
                 "the content indentation with no paragraph open, so the item ENDS before it (commonmark.js: a "
                 "paragraph and a code block outside the list) -- the indented line is §4.4 raw, no site",
     build(), "- item\n\n para\n\n    Slice 9z owns it", 0)
acase("POSITIVE", "(item) … and that raw line is seeded under the one raw-line rule, with the §4.4 reading",
      build(), "LEX-UNSUPPORTED?", 1, prose="- item\n\n para\n\n    Slice 9z owns it")
case("NEGATIVE", "(item) `- item\\n\\n  para\\n\\n# h\\n\\n    Slice 9z owns it`: a 0-column heading is a lazy "
                 "candidate that is a block start -- the item ends, the heading and the code block are outside "
                 "it: no site",
     build(), "- item\n\n  para\n\n# h\n\n    Slice 9z owns it", 0)
case("NEGATIVE", "(item) `- item\\n\\n      Slice 9z owns it`: six columns are four inside the item -- indented "
                 "code IN the item (commonmark.js), raw: no site",
     build(), "- item\n\n      Slice 9z owns it", 0)
acase("POSITIVE", "(item) … and that raw line inside the item is seeded (the one raw-line rule reaches into a "
                  "container)",
      build(), "LEX-UNSUPPORTED?", 1, prose="- item\n\n      Slice 9z owns it")
# laziness: §5.2 rule 5 through the SAME mechanism as §5.1
case("NEGATIVE", "(item) `- open `x\\nSlice 9z owns it` end`: the line short of the content indentation is the "
                 "item paragraph's lazy continuation text (§5.2 rule 5), so the span crosses it: no site",
     build(), "- open `x\nSlice 9z owns it` end", 0)
# the §5.2 interruption rule, where a paragraph is open (commonmark.js: each shape)
case("NEGATIVE", "(item) `open `x\\n2. 9z owns it` end`: an ordered item not starting at 1 cannot interrupt a "
                 "paragraph (§5.2) -- one paragraph, the span masks the site",
     build(), "open `x\n2. 9z owns it` end", 0)
case("POSITIVE", "(item) `open `x\\n1. 9z owns it` end`: an ordered item starting at 1 interrupts -- the item's "
                 "paragraph holds one literal backtick and the site",
     build(), "open `x\n1. 9z owns it` end", 1)
case("NEGATIVE", "(item) `open `x\\n*\\n9z owns it` end`: an EMPTY item cannot interrupt a paragraph (§5.2; `*`, "
                 "not `-`, which would be a setext underline) -- one paragraph, the span masks the site",
     build(), "open `x\n*\n9z owns it` end", 0)
# nested containers compose: the same `_parse` in an item in a quote, a quote in an item
case("POSITIVE-NOVEL", "(item) `> - [sib]: slice-9z-sib.md`: a definition inside an item inside a block quote "
                       "registers -- the sibling is walked and its violation reported",
     build(), "> - [sib]: slice-9z-sib.md\n\nSee [sib].", 1, **SIB)
case("POSITIVE-NOVEL", "(item) `- > [sib]: slice-9z-sib.md`: a definition inside a block quote inside an item "
                       "registers -- the sibling is walked",
     build(), "- > [sib]: slice-9z-sib.md\n\nSee [sib].", 1, **SIB)
case("POSITIVE-NOVEL", "(item) a slot table inside a list item is a table (cmark-gfm, measured): its row declares "
                       "its id",
     build(extra="- " + "\n  ".join((SLOT4 % "now").split("\n"))), "", 1, measure=("id", "#11-zz-gamma"))

# #2: an orphan definition keeps its DESTINATION; the population miss is
# raised only where `sibling_path` maps it to a sibling on disk (ONE resolver)
rcase("NEGATIVE", "(def) `paragraph\\n[x]: #section\\n[x]`: the orphan names a section, never a memo -- the "
                  "shortcut is prose, rc 0",
      build(), "paragraph\n[x]: #section\n[x]", 0)
rcase("NEGATIVE", "(def) `paragraph\\n[x]: https://example.com/a\\n[x]`: the orphan names an external URL, never "
                  "a memo -- rc 0",
      build(), "paragraph\n[x]: https://example.com/a\n[x]", 0)
case("POSITIVE", "(def) `paragraph\\n[x]: child.md\\n[x]`: the orphan names a sibling on disk -- the documented "
                 "miss, rc 2 (unchanged)",
     build(), "paragraph\n[x]: child.md\n[x]", 1, measure=("schema", "unresolved reference 'x'"))

# #3: the row noun is ASCII case-insensitive, in ONE place (`ROW_NOUN`)
case("POSITIVE-NOVEL", "(noun) `SLICE C owns it` names the row: the row noun folds case (a bare `C` is the "
                       "declared single-letter miss, so only the anchor can reach it)",
     build(), "SLICE C owns it.", 1)
case("POSITIVE-NOVEL", "(noun) `ROW 9 lands first` names the row (a bare `9` is the declared numeric miss)",
     build(), "ROW 9 lands first.", 1)
case("POSITIVE-NOVEL", "(noun) `UMBRELLA C owns it` names the row",
     build(), "UMBRELLA C owns it.", 1)


# ------------------------------------------------ PR #510 Codex R16 controls --
# #2 (IMP): §2.5 character references in a link DESTINATION are decoded by the
# ONE destination normalisation (`normalize_destination`: §2.4 escapes and §2.5
# references in one pass), at every destination site -- the inline link, bare
# and in angle brackets, and the reference definition, which reads its
# destination through the same `link_destination`.  Until R16 the destination
# was backslash-unescaped only, so `[child](child&#46;md)` reached
# `sibling_path` as the literal `child&#46;md`: no `.md` suffix, the sibling
# silently outside the population, rc 0.  Every expectation below was read off
# commonmark.js 0.31.2 (`node cm.js '["<md>"]'`) before being written.  The
# NEGATIVE half measures SITES (0 = `child.md` not walked), not rc: a NAMING
# site is a seed and never moves rc, so an `rc 0` expectation stays green
# whether the sibling is walked or not -- three R16 mutants survived it.
case("POSITIVE-NOVEL", "(link) `[child](child&#46;md)`: a decimal character reference in the destination is "
                       "decoded (§2.5 / §6.3) -- `child.md` is walked and its violation reported",
     build(), "See [child](child&#46;md).", 1, files=CHILD)
case("POSITIVE-NOVEL", "(link) `[child](<child&#46;md>)`: the angle-bracket destination decodes by the same rule",
     build(), "See [child](<child&#46;md>).", 1, files=CHILD)
case("POSITIVE-NOVEL", "(link) `[child](child&period;md)`: a named entity reference (HTML5 `&period;`) decodes",
     build(), "See [child](child&period;md).", 1, files=CHILD)
case("POSITIVE-NOVEL", "(link) `[child](child&#x2E;md)`: a hexadecimal character reference decodes",
     build(), "See [child](child&#x2E;md).", 1, files=CHILD)
case("POSITIVE-NOVEL", "(def) `[sib]: child&#46;md`: a reference definition's destination is decoded by the SAME "
                       "normalisation -- the sibling is walked",
     build(), "[sib]: child&#46;md\n\nSee [sib].", 1, files=CHILD)
case("POSITIVE-NOVEL", "(link) `[x](slice&#37;20sib.md)`: §2.5 decodes `&#37;` to `%`, then `sibling_path` "
                       "percent-decodes `%20` (stage b) -- the two decoders run in spec order, `slice sib.md` is walked",
     build(), "See [x](slice&#37;20sib.md).", 1, files={"slice sib.md": VIOLATION + "\n"})
case("POSITIVE-NOVEL", "(link) `[x](child&#0;.md)`: U+0000 is replaced by U+FFFD (§2.5, \"for security reasons\") -- "
                       "the memo named `child\\ufffd.md` is walked (a raw U+0000 would be rejected as a C0 control)",
     build(), "See [x](child&#0;.md).", 1, files={"child�.md": VIOLATION + "\n"})
case("POSITIVE-NOVEL", "(link) `[x](child&#x110000;.md)`: an invalid code point is U+FFFD (§2.5) -- `child\\ufffd.md` "
                       "is walked",
     build(), "See [x](child&#x110000;.md).", 1, files={"child�.md": VIOLATION + "\n"})
case("POSITIVE-NOVEL", "(link) `[x](child&copy.md)`: HTML's legacy semicolon-less `&copy` is NOT a reference in "
                       "CommonMark (§2.5 Example 29) -- the destination is the literal `child&copy.md`, and THAT "
                       "file is walked (`html.unescape` would have named `child©.md`)",
     build(), "See [x](child&copy.md).", 1, files={"child&copy.md": VIOLATION + "\n"})
case("NEGATIVE", "(link) `[x](child&#46md)`: `&#46` without `;` is no reference -- the destination is the literal "
                  "`child&#46md`, which names no memo: 0 sites, `child.md` not walked",
      build(), "See [x](child&#46md).", 0, files=CHILD)
case("NEGATIVE", "(link) `[x](child\\&#46;md)`: a backslash-escaped `&` opens no reference (ONE pass: §2.4 and §2.5 "
                  "meet at the character once) -- literal `child&#46;md`, no memo, 0 sites",
      build(), "See [x](child\\&#46;md).", 0, files=CHILD)
case("NEGATIVE", "(link) `[x](child&#x26;#46;md)`: a decoded `&` is a character, never the start of a second "
                  "reference (one pass, no re-scan) -- literal `child&#46;md`, 0 sites",
      build(), "See [x](child&#x26;#46;md).", 0, files=CHILD)
case("NEGATIVE", "(link) `[x](child&MadeUpEntity;md)`: a name not on the HTML5 list is literal text (§2.5 Example "
                  "30) -- no memo, 0 sites",
      build(), "See [x](child&MadeUpEntity;md).", 0, files=CHILD)
case("NEGATIVE", "(span) `` `[c](child&#46;md)` `` is a code span: §2.5's first exception -- nothing in it is a "
                  "destination and nothing is decoded; 0 sites, `child.md` not walked",
      build(), "See `[c](child&#46;md)` here.", 0, files=CHILD)
case("NEGATIVE", "(label) `[foo&auml;]: child.md` then `[fooä]`: §6.3 label matching is on the RAW label (case fold, "
                  "strip, collapse -- no character-reference decoding; commonmark.js: no link) -- the shortcut is "
                  "prose, 0 sites, `child.md` not walked",
      build(), "[foo&auml;]: child.md\n\nSee [fooä].", 0, files=CHILD)


# ------------------------------------------------ PR #510 Codex R17 controls --
# #1 (IMP): a reference definition keeps the RUN open, so a lazy table header
# right after it is the table's header inside the container (cmark-gfm,
# measured shape by shape -- `Memo._parse`'s docstring is the table).  The
# shapes no id can discriminate are the controls module's block-sequence control; the
# reviewer's consequence (a linked memo's schema table dropped, rc 0) is its
# `lazy_header_after_definition_control`.

def _lazy_header(head, table):
    """`head` (marker lines, ending in a line ending) then `table` with its
    HEADER line unquoted -- the quote's lazy candidate -- and every other
    line quoted."""
    return head + "\n".join(("> " if k else "") + l for k, l in enumerate(table.split("\n")))


case("POSITIVE-NOVEL", "(quote) the R17 reviewer's shape: `> [a]: /u` then an UNQUOTED schema header over a QUOTED "
                       "delimiter row is a table in the quote (cmark-gfm: a definition is paragraph text until the "
                       "paragraph ends, and the header is read out of its last line) -- the row declares its id",
     build(extra=_lazy_header("> [a]: /u\n", SLOT4 % "now")), "", 1, measure=("id", "#11-zz-gamma"))
case("POSITIVE-NOVEL", "(quote) two definitions then the lazy schema header: still the table's header, the id declared",
     build(extra=_lazy_header("> [a]: /u\n> [b]: /v\n", SLOT4 % "now")), "", 1, measure=("id", "#11-zz-gamma"))
case("POSITIVE-NOVEL", "(item) `- [a]: /u` then the lazy schema header over an indented delimiter row: the table is the "
                       "item's (cmark-gfm, measured) -- the id declared",
     build(extra="- [a]: /u\n" + "\n".join(("  " if k else "") + l for k, l in enumerate((SLOT4 % "now").split("\n")))),
     "", 1, measure=("id", "#11-zz-gamma"))
case("NEGATIVE", "(quote) `> [a]: /u\\n>` then the lazy schema header: the blank quote line closes the run, so the quote "
                 "ENDS and the header is a paragraph outside it (cmark-gfm) -- no id declared",
     build(extra=_lazy_header("> [a]: /u\n>\n", SLOT4 % "now")), "", 0, measure=("id", "#11-zz-gamma"))
case("NEGATIVE", "(quote) a fenced block then the lazy schema header: no run is open, the quote ends (cmark-gfm) -- no id "
                 "declared",
     build(extra=_lazy_header("> ```\n> x\n> ```\n", SLOT4 % "now")), "", 0, measure=("id", "#11-zz-gamma"))

# #2 (IMP): CommonMark §6.6 raw HTML is a span of the ONE inline pass -- masked
# like a code span (never inline-parsed, never a naming site) and seeded like
# a raw HTML-block line (the plan's disposition for the same kind of text).
# Every grammar arm and every negative below was read off commonmark.js
# 0.31.2 (`node cm.js`) at an INLINE position (`a <…>`) before being written;
# the spec's own §6.6 examples are the controls module's inline conformance control.
ABSENT = "absent-file.md"
rcase("NEGATIVE", "(html) the R17 reviewer's shape `<span title=\"[child](absent.md)\">text</span>`: a bracket inside a "
                  "double-quoted attribute value is no link -- nothing is walked, rc 0",
      build(), 'See <span title="[child](%s)">text</span>.' % ABSENT, 0)
rcase("NEGATIVE", "(html) a single-quoted attribute value `<span title='[x](absent.md)'>` is a tag: rc 0",
      build(), "See <span title='[x](%s)'>t</span>." % ABSENT, 0)
rcase("NEGATIVE", "(html) an UNQUOTED attribute value holding brackets `<span title=[x](absent.md)>` IS an open tag (§6.6: "
                  "an unquoted value excludes only spaces, tabs, line endings, `\"`, `'`, `=`, `<`, `>` and a backtick; "
                  "commonmark.js agrees -- ⚠ the R17 brief listed this shape as a negative): rc 0",
      build(), "See <span title=[x](%s)>t</span>." % ABSENT, 0)
rcase("NEGATIVE", "(html) an HTML comment `<!-- [x](absent.md) -->` is raw: rc 0",
      build(), "See <!-- [x](%s) --> here." % ABSENT, 0)
rcase("NEGATIVE", "(html) a comment may cross the paragraph's line ending (`<!--\\n[x](absent.md)\\n-->`, Example 625's "
                  "shape): rc 0",
      build(), "See <!--\n[x](%s)\n--> here." % ABSENT, 0)
rcase("NEGATIVE", "(html) a processing instruction `<? [x](absent.md) ?>` is raw: rc 0",
      build(), "See <? [x](%s) ?> here." % ABSENT, 0)
rcase("NEGATIVE", "(html) a declaration `<!X [x](absent.md)>` is raw: rc 0",
      build(), "See <!X [x](%s)> here." % ABSENT, 0)
rcase("NEGATIVE", "(html) a CDATA section `<![CDATA[ [x](absent.md) ]]>` is raw: rc 0",
      build(), "See <![CDATA[ [x](%s) ]]> here." % ABSENT, 0)
rcase("POSITIVE", "(html) `<3 [x](absent.md)`: a tag name begins with an ASCII letter, so the `<` is literal and the link "
                  "is read -- the absent memo is the rc-2 miss",
      build(), "See <3 [x](%s)." % ABSENT, 2)
rcase("POSITIVE", "(html) `< span title=\"[x](absent.md)\">`: nothing may stand between `<` and the tag name -- literal, "
                  "the link is read, rc 2",
      build(), 'See < span title="[x](%s)">.' % ABSENT, 2)
rcase("POSITIVE", "(html) `</ span [x](absent.md)>`: a closing tag's name follows `</` directly -- literal, rc 2",
      build(), "See </ span [x](%s)>." % ABSENT, 2)
rcase("POSITIVE", "(html) `<a href=\"x\" [x](absent.md)>`: `[x](…)` is no attribute, so the `<` is literal and the link "
                  "is read, rc 2",
      build(), 'See <a href="x" [x](%s)>.' % ABSENT, 2)
rcase("POSITIVE", "(html) `<a b=\"c\"d=\"[x](absent.md)\">`: an attribute needs whitespace before it (Example 622) -- "
                  "literal, the link is read, rc 2",
      build(), 'See <a b="c"d="[x](%s)">.' % ABSENT, 2)
rcase("POSITIVE", "(html) `<!-- [x](absent.md) --` is not closed: no comment, the link is read, rc 2",
      build(), "See <!-- [x](%s) --" % ABSENT, 2)
rcase("POSITIVE", "(html) `<! [x](absent.md)>`: a declaration needs an ASCII letter right after `<!` -- literal, rc 2",
      build(), "See <! [x](%s)>." % ABSENT, 2)
rcase("POSITIVE", "(html) `<![cdata[ [x](absent.md) ]]>`: `<![CDATA[` is exact case -- literal, rc 2",
      build(), "See <![cdata[ [x](%s) ]]>." % ABSENT, 2)
rcase("POSITIVE", "(html) `\\<span title=\"[x](absent.md)\">`: an escaped `<` opens no tag (§2.4), the link is read, rc 2",
      build(), 'See \\<span title="[x](%s)">.' % ABSENT, 2)
case("NEGATIVE", "(html) `<span title=\"Slice 9z owns it\">x</span>`: an id inside an attribute value is no naming site -- "
                 "the span is masked whole (kind `html`), as a raw HTML-block line is raw",
     build(), 'See <span title="Slice 9z owns it">x</span>.', 0)
case("NEGATIVE", "(html) `<a title=9z-owns>x</a>`: an unquoted attribute value is masked too",
     build(), "See <a title=9z-owns>x</a>.", 0)
case("NEGATIVE", "(html) a cell's `<span title=\"Slice 9z owns it\">` is masked by the same inline pass: no site",
     build(c1='<span title="Slice 9z owns it">x</span>'), "", 0)
case("NEGATIVE", "(html) `<!-- Slice 9z -- owns it -->`: `--` inside a comment is allowed since 0.31 (§6.6: \"a string of "
                 "characters not including the string -->\") -- the whole comment is masked",
     build(), "See <!-- Slice 9z -- owns it --> here.", 0)
case("NEGATIVE", "(html) `<? Slice 9z owns it ?>`: a processing instruction is masked",
     build(), "See <? Slice 9z owns it ?> here.", 0)
case("NEGATIVE", "(html) `<!X Slice 9z owns it>`: a declaration is masked",
     build(), "See <!X Slice 9z owns it> here.", 0)
case("NEGATIVE", "(html) `<![CDATA[ Slice 9z owns it ]]>`: a CDATA section is masked",
     build(), "See <![CDATA[ Slice 9z owns it ]]> here.", 0)
case("POSITIVE", "(html) `<!-- Slice 9z owns it --` is not closed: prose, the site is reported",
     build(), "See <!-- Slice 9z owns it --", 1)
case("POSITIVE", "(html) `<! Slice 9z owns it>`: no ASCII letter after `<!`, no declaration -- the site is reported",
     build(), "See <! Slice 9z owns it>.", 1)
case("POSITIVE", "(html) `<a b=\"c\"d=9z>x</a>`: no whitespace before `d` (Example 622), no tag -- `9z` is prose",
     build(), 'See <a b="c"d=9z>x</a>.', 1)
case("POSITIVE", "(html) `<span title=\"`\">Slice 9z owns it` x`: the tag is read first, left to right, so the backtick "
                 "inside it opens no code span -- the site is reported (commonmark.js)",
     build(), 'See <span title="`">Slice 9z owns it` x.', 1)
case("POSITIVE", "(html) `` `x <span title=\"`9z\">y ``: the code span is read first and swallows the `<`, so no tag masks "
                 "`9z` -- the site is reported (commonmark.js)",
     build(), 'See `x <span title="`9z">y.', 1)
case("POSITIVE", "(html) `[<span>x</span>](slice-9z-sib.md)`: a link may wrap a tag -- the sibling is walked",
     build(), "See [<span>x</span>](slice-9z-sib.md).", 1, **SIB)
acase("POSITIVE", "(lex-seed) `<span title=\"Slice 9z owns it\">`: an inline span holding a declared id is seeded under "
                  "the one raw-line rule",
      build(), "LEX-UNSUPPORTED?", 1, prose='See <span title="Slice 9z owns it">x</span>.')
case("POSITIVE", "(lex-seed) … and that seed carries the `inline` reading",
     build(), 'See <span title="Slice 9z owns it">x</span>.', 1, measure=("seed", "inline raw HTML span"))
acase("NEGATIVE", "(lex-seed) `a<br>b` / `<sub>2</sub>`: tags holding neither a `|` nor a declared id are no seed",
      build(), "LEX-UNSUPPORTED?", 0, prose="See a<br>b and <sub>2</sub>.")


# ------------------------------------------------ PR #510 Codex R19 controls --
# #1 (IMP): CommonMark §6.4 -- a RESOLVED image's description is plain text
# (its alt), so every bracket construct recorded inside it is the
# description's, in ONE rule at the point the image closes (`inline_pass`):
# a link there is DEMOTED to a masked tail (never a memo link, never prose),
# a nested image stays masked, a failed reference names no lost memo.  An
# UNRESOLVED image is literal `![` text and keeps the link inside it.  Until
# R19 the inner link joined the population as it closed, so the reviewer's
# `![alt [docs](absent.md)](image.png)` was a false unavailable-memo miss,
# rc 2.  Every expectation below was read off commonmark.js 0.31.2 (`node
# cm.js '["<md>"]'`) before being written.  The link-in-link mirror
# (`[a [b](x.md)](y.md)`: the inner link wins, `](y.md)` is literal) is the
# R1-3 control "(link) nested inline links: the INNER link is the link…".
rcase("NEGATIVE", "(image) the R19 reviewer's input `![alt [docs](absent-file.md)](image.png)`: the image resolves, so "
                  "its description is plain text (§6.4; commonmark.js: `<img alt=\"alt docs\">`) -- the link inside it "
                  "is no memo link, nothing is walked, rc 0",
      build(), "See ![alt [docs](%s)](image.png)." % ABSENT, 0)
case("NEGATIVE", "(image) `![alt [docs](child.md)](absent-image.md)`: neither the demoted link nor the image is a memo "
                 "link -- `child.md` is not walked (its violation is unreported) and the image's `.md` destination is "
                 "not probed",
     build(), "See ![alt [docs](child.md)](absent-image.md).", 0, files=CHILD)
case("POSITIVE-NOVEL", "(image) `![alt [docs](child.md)][missing]`: the image does NOT resolve, so `![alt` is literal text "
                       "and the link inside it IS a link (commonmark.js) -- `child.md` is walked and its violation "
                       "reported",
     build(), "See ![alt [docs](child.md)][missing].", 1, files=CHILD)
case("NEGATIVE", "(image) a nested image in a resolved image `![a ![b [c](child.md)](i.png)](j.png)`: the inner image "
                 "stays masked and the link inside it is demoted -- `child.md` is not walked",
     build(), "See ![a ![b [c](child.md)](i.png)](j.png).", 0, files=CHILD)
case("NEGATIVE", "(image) `![a ![b [c](child.md)](i.png)][missing]`: the OUTER image fails but the INNER one resolves, and "
                 "the link inside the inner description is demoted (commonmark.js: `<img alt=\"b c\">`) -- `child.md` is "
                 "not walked",
     build(), "See ![a ![b [c](child.md)](i.png)][missing].", 0, files=CHILD)
case("POSITIVE-NOVEL", "(image) `![a ![b](i.png) [c](child.md)][missing]`: the link stands OUTSIDE the inner image's "
                       "description and the outer image fails -- it is a link, `child.md` is walked",
     build(), "See ![a ![b](i.png) [c](child.md)][missing].", 1, files=CHILD)
case("NEGATIVE", "(link) `[a ![b [c](child.md)](i.png)](parent.md)`: the inner link deactivated the outer `[` as it closed, "
                 "before the image demoted it (§6.3 \"links may not contain links\"; commonmark.js: `[a <img alt=\"b "
                 "c\">](parent.md)`) -- neither `child.md` nor the absent `parent.md` is linked: 0 sites, not rc 2",
     build(), "See [a ![b [c](child.md)](i.png)](parent.md).", 0, files=CHILD)
rcase("NEGATIVE", "(image) `![alt [x][missing]](img.png)`: a failed reference inside a resolved image's description names "
                  "no lost memo (resolved, it would have been demoted; commonmark.js: `alt=\"alt [x][missing]\"`) -- rc 0",
      build(), "See ![alt [x][missing]](img.png).", 0)
case("NEGATIVE", "(image) `![alt [b](9z)](i.png)`: the demoted link's tail stays masked -- the `9z` in its destination is "
                 "not prose, 0 sites",
     build(), "See ![alt [b](9z)](i.png).", 0)

# #3 (IMP): `sibling_path` stage (c) is ONE platform-independent rule -- the
# decoded name is read under Windows path syntax (`PureWindowsPath`, the
# superset: `/` and `\` both separate, a drive / UNC / root prefix anchors)
# on every platform, and an anchored name is rejected; stage (e) joins the
# name's parts, so `\` is a separator everywhere, never a POSIX name
# character.  WHATWG URL `#path-state` step 1 reads a special-scheme path
# (`file` is special) the same way, and calls its drive-letter quirk
# "platform-independent".  Until R19 (c) rejected a leading `/` only.
rcase("NEGATIVE", "(rc) a percent-encoded Windows drive-absolute `C%3A%5Ctemp%5Cchild.md` (`C:\\temp\\child.md`) is "
                  "rejected after decoding on every platform (stage c: a drive anchors): rc 0",
      build(), "See [x](C%3A%5Ctemp%5Cchild.md).", 0)
rcase("NEGATIVE", "(rc) a percent-encoded backslash-rooted `%5Cchild.md` (`\\child.md`) is rejected (stage c: a root "
                  "anchors): rc 0",
      build(), "See [x](%5Cchild.md).", 0)
rcase("NEGATIVE", "(rc) a percent-encoded UNC `%5C%5Cserver%5Cshare%5Cx.md` is rejected (stage c: a UNC prefix anchors): "
                  "rc 0",
      build(), "See [x](%5C%5Cserver%5Cshare%5Cx.md).", 0)
rcase("NEGATIVE", "(rc) a raw `\\\\server\\share\\x.md` destination decodes (§2.4: `\\\\` is one backslash, `\\s` is "
                  "literal) to the backslash-rooted `\\server\\share\\x.md` (commonmark.js: href "
                  "`%5Cserver%5Cshare%5Cx.md`) and is rejected: rc 0",
      build(), "See [x](\\\\server\\share\\x.md).", 0)
rcase("NEGATIVE", "(rc) drive-relative `C:child.md`: raw, it is a URL of scheme `c` (stage a); percent-encoded "
                  "`C%3Achild.md` decodes to a drive-anchored name (stage c) -- both rejected, rc 0",
      build(), "See [a](C:child.md) and [b](C%3Achild.md).", 0)
rcase("NEGATIVE", "(rc) `n%3Achild.md`: a ONE-letter name before `:` is a Windows drive letter (URL `#path-state` step "
                  "1.4.1, platform-independent) -- drive-relative, rejected, rc 0; the multi-letter `notes%3Achild.md` "
                  "of R8 stays a file name",
      build(), "See [x](n%3Achild.md).", 0)
case("POSITIVE-NOVEL", "(link) `sub%5Cchild.md`: a backslash is a path separator on every platform (WHATWG URL "
                       "`#path-state` step 1: for a special scheme -- `file` is one -- `\\` ends a segment as `/` does; "
                       "`PureWindowsPath` is that syntax) -- the file `sub/child.md` is walked",
     build(), "See [x](sub%5Cchild.md).", 1, files={"sub/child.md": VIOLATION + "\n"})


# ------------------------------------------------ PR #510 Codex R20 controls --

# #1 (MIN): a file name is anything ending in the lexer's `FILE_SUFFIX` -- the
# stem may be EMPTY, exactly as `sibling_path` stage (d) reads a destination
# (the one constant, defined in the lexer, consumed by the memo).  Until R20
# the token arm required a stem of one character or more, so beside a
# declared id `md` the prose `Read .md for details` reported `md` as a site.
MD = dict(i7z="**md**", s7z="**UMBRELLA, not a terminal unit.** x")
case("NEGATIVE", "(file) `.md` alone is a file name (the suffix-only name `sibling_path` accepts): beside a declared "
                 "no-owner id `md`, `Read .md for details` reports 0 sites",
     build(**MD), "Read .md for details.", 0)
case("NEGATIVE", "(file) `notes.md` beside a declared no-owner id `md` is still one file name: 0 sites",
     build(**MD), "Read notes.md for details.", 0)
case("POSITIVE", "(file) bare `md` (no suffix) beside a declared no-owner id `md` IS a site -- the subject of the two "
                 "controls above is live",
     build(**MD), "Read md for details.", 1)
case("POSITIVE-NOVEL", "(link) `[x](.md)` links the sibling file named `.md`: `sibling_path` stage (d) and the lexer's "
                       "file token read the ONE `FILE_SUFFIX`, so the suffix-only name is a file on both sides",
     build(), "See [x](.md).", 1, files={".md": VIOLATION + "\n"})

# #2 (IMP): every composer that reads "a row id in this position" composes the
# grammar's `ROW_ID` (slug | short) -- the marker's appositive subject, the
# two-owner clause, the row-noun-anchored reading.  Until R20 the appositive
# was built on `SHORT_ID` alone: the reviewer's declaring field below did not
# match, the row was read as an umbrella, no attribution finding, a corrupted
# census, exit 0.  The R14 spelling sweep reads spellings, not kind coverage;
# the controls module's `row_kind_coverage_control` is the kind half.
SLUG_PTR = "Slice %s — **UMBRELLA, not a terminal unit** — points into §8."
acase("POSITIVE", "(a) the R20 reviewer's declaring field `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributes the marker "
                  "to a SLUG row: the row is a pointer and the UMBRELLA-MARK attribution finding is emitted",
      build(wb=SLUG_PTR % "`#11-zz-alpha`"), "UMBRELLA-MARK", 1)
acase("POSITIVE", "(a) the bold slug form `Slice **#11-zz-alpha** — **UMBRELLA, …**` attributes the marker too",
      build(wb=SLUG_PTR % "**#11-zz-alpha**"), "UMBRELLA-MARK", 1)
acase("POSITIVE", "(a) the short form `Slice **9z** — **UMBRELLA, …**` in the same position still attributes (the "
                  "alternation lost nothing)",
      build(wb=SLUG_PTR % "**9z**"), "UMBRELLA-MARK", 1)
acase("POSITIVE", "(d) `owned by `#11-zz-alpha` and **Qx**`: a SLUG owner and a short owner are a two-owner clause "
                  "(`OWNS_TWO` composes `ROW_ID`)",
      build(s9z="charter.  The drain is owned by `#11-zz-alpha` and **Qx**."), "TWO-OWNERS?", 1)
acase("POSITIVE", "(c-seed) `Lands after Slice `#11-zz-alpha`` in a Slice cell whose Deps cell names only **9z**: the "
                  "anchored reading admits the slug, so the prose names a party the cell does not carry -- "
                  "ORDER-PROSE? 1",
      build(s7z="Terminal.  Acceptance: the probe must return 3.  Lands after Slice `#11-zz-alpha`.", d7z="**9z**"),
      "ORDER-PROSE?", 1)
acase("NEGATIVE", "(c-seed) the same prose with `#11-zz-alpha` in the Deps cell too: the cell carries the party, "
                  "ORDER-PROSE? 0",
      build(s7z="Terminal.  Acceptance: the probe must return 3.  Lands after Slice `#11-zz-alpha`.",
            d7z="**9z**, `#11-zz-alpha`"),
      "ORDER-PROSE?", 0)
