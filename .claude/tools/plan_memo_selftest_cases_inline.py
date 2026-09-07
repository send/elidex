#!/usr/bin/env python3
"""The PR #510 Phase-2 INLINE-construct rounds of the control registry.

The third module of the one `CASES` list, split from
`plan_memo_selftest_cases_pr510.py` at the same review-round seam that module
was split from `plan_memo_selftest_cases.py` at (touch-time 1000-line rule:
`_cases_pr510.py` had reached 949 lines before the R21 controls).  The seam is
where the converge turned from Phase 1 to Phase 2: `_cases_pr510.py` holds
Codex R1-R16 and the design re-gates -- the lexical substrate, the block
grammar and the one pipeline -- and this module holds every round from R17 on,
the rounds that closed the INLINE construct family (R17 CommonMark 6.6 raw
HTML, R19 6.4 images, R21 6.5 autolinks and the closed 6.1-6.9 list of plan
3.1) together with what landed beside them (R19's display name and the
sibling resolver's path syntax, R20's row-kind grammar and file token, R21's
out-of-field marker / KIND-SPELLING / schema id kinds).

Same spellings (`case` / `acase` / `rcase`), same `CASES` list, one import
site: the controls module imports this module for its side effect.  A
control's mutant lives in `plan_memo_selftest_mutants_pr510.py` under the same
round label.
"""

from plan_memo_selftest_cases import SIB, VIOLATION, acase, build, case, rcase
from plan_memo_selftest_cases_pr510 import CHILD, SLOT4

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
