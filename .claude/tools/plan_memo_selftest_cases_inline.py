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


# ------------------------------------------------ PR #510 Codex R21 controls --

# #1 (IMP): CommonMark §6.5 autolinks.  `<https://example.com/[child](absent.md)>`
# is ONE autolink -- its contents are not link syntax (commonmark.js 0.31.2:
# `<a href="https://example.com/%5Bchild%5D(absent.md)">`) -- and until R21 the
# brackets inside it were scanned, `absent.md` entered the population, and the
# unavailable-memo miss was a false rc 2.  The disposition (plan §3.0b) is
# MASKED: the span is one token, its text IS its destination, so nothing
# inside it is a link, a naming site or a sibling, and -- unlike a §6.6 raw
# HTML span -- it is NOT seeded, because the construct is fully lexed.
rcase("NEGATIVE", "(autolink) the R21 reviewer's input `<https://example.com/[child](absent.md)>` is ONE §6.5 "
                  "autolink: the brackets inside it are not link syntax, `absent.md` is no memo, rc 0",
      build(), "See <https://example.com/[child](absent.md)> here.", 0)
rcase("POSITIVE", "(autolink) the same brackets OUTSIDE an autolink are a link to a memo that is not there: "
                  "rc 2 -- the control above has a live subject",
      build(), "See [child](absent-file.md) here.", 2)
rcase("NEGATIVE", "(autolink) an autolink's URL is never a sibling on disk (§6.5 admits an absolute URI or an "
                  "email address only, and `sibling_path` stage (a) rejects a scheme): rc 0",
      build(), "See <https://example.com/absent-file.md> here.", 0)
case("NEGATIVE", "(autolink) a declared id inside a URI autolink is no naming site: the span is masked whole, "
                 "exactly as a link's destination is -- 0 sites",
     build(), "See <https://example.com/9z> here.", 0)
case("POSITIVE", "(autolink) the same URL WITHOUT the angle brackets is prose (§6.5 Example 611: a bare URL is "
                 "no autolink), so the id in it IS a site -- the control above has a live subject",
     build(), "See https://example.com/9z here.", 1)
case("NEGATIVE", "(autolink) a declared id inside an EMAIL autolink is no naming site either (the second §6.5 "
                 "arm, `mailto:` at the renderer) -- 0 sites.  The id is the whole local part: dropping the arm "
                 "leaves `9z` bounded by `<` and `@`, so the mutant HAS a site to report (an id glued to a "
                 "domain label -- `ops@9z.example.com` -- is no site either way and proves nothing)",
     build(), "See <9z@example.com> here.", 0)
case("NEGATIVE", "(autolink) backslash escapes do not work inside an autolink (§6.5 Example 603, "
                 "`<https://example.com/\\[\\>`): the span is matched raw, so the `\\[` opens nothing and the id "
                 "in it is masked -- 0 sites",
     build(), "See <https://example.com/\\[9z\\> here.", 0)
case("NEGATIVE", "(autolink) a backtick inside an autolink is consumed by it, not by a code span (the three "
                 "delimiters are read left to right as met; commonmark.js agrees) -- 0 sites",
     build(), "See `a` <https://e.example/9z-`b`> `c` here.", 0)
case("POSITIVE-NOVEL", "(autolink) `<m:abc…>` is a ONE-character scheme and no autolink (§6.5 Example 609, "
                       "2-32 characters): the `<` is literal and the link INSIDE the would-be span is read "
                       "(commonmark.js: `&lt;m:abc<a href=\"child.md\">x</a>&gt;`).  The link sits inside the "
                       "span so that widening the scheme to one character masks it and the control goes red",
     build(), "See <m:abc[x](child.md)> here.", 1, files=CHILD)
case("POSITIVE-NOVEL", "(autolink) a SPACE inside the absolute URI ends it (§6.5 Example 608), so "
                       "`<https://foo.bar/ …>` is no autolink and no tag either -- the `<` is literal and the "
                       "link INSIDE the would-be span is read (commonmark.js: `&lt;https://foo.bar/ "
                       "<a href=\"child.md\">x</a>&gt;`).  The space sits in the URI TAIL, which is the class "
                       "the grammar excludes it from: a space right after `<` fails at the SCHEME instead, so "
                       "that shape leaves the tail-widening mutant alive and proves nothing",
     build(), "See <https://foo.bar/ [x](child.md)> here.", 1, files=CHILD)
case("POSITIVE-NOVEL", "(autolink) `<foo.bar.baz>` has neither a scheme nor an `@` (§6.5 Example 610) and is no "
                       "tag: the `<` is literal and the link after it is read",
     build(), "See <foo.bar.baz> [x](child.md) here.", 1, files=CHILD)
case("POSITIVE-NOVEL", "(autolink) an autolink inside a LINK's text: the autolink is masked, the link is still "
                       "the memo link (commonmark.js nests the two `<a>`s)",
     build(), "See [<https://a.example/b>](child.md) here.", 1, files=CHILD)
acase("NEGATIVE", "(autolink) an autolink holding a declared id and a `|` is NOT a `[LEX-UNSUPPORTED?]` seed: "
                  "the construct is fully lexed and hides nothing, unlike a raw HTML span",
      build(), "LEX-UNSUPPORTED?", 0, prose="See <https://e.example/9z|x> here.")
acase("POSITIVE", "(autolink) the same id in a raw HTML SPAN is a seed -- the two dispositions differ, and the "
                  "control above has a live subject",
      build(), "LEX-UNSUPPORTED?", 1, prose="See <span title=\"9z\">x</span> here.")

# #2 (IMP): the marker outside the declaring field certifies nothing, and that
# is true of a row that ALSO carries it where it belongs.  The seed for the
# kind said in words is the only thing a marker in the declaring field
# short-circuits; until R21 one `continue` gated the out-of-field scan too, so
# a row marked twice was reported only when the second marker was its only one.
MARK = "**UMBRELLA, not a terminal unit.**"
acase("POSITIVE", "(a) a row carrying the marker in its declaring field AND again in another cell is the "
                  "double marker the assertion forbids: UMBRELLA-MARK 1",
      build(d9z=MARK), "UMBRELLA-MARK", 1)
acase("NEGATIVE", "(a) the same row with the marker in its declaring field ONLY: UMBRELLA-MARK 0 -- the scan "
                  "reads the other cells, it does not report them",
      build(), "UMBRELLA-MARK", 0)
acase("POSITIVE", "(a) a row carrying the marker in another cell and NOT in its declaring field is the same "
                  "finding (the arm that already worked)",
      build(sqx="Terminal.  Acceptance: the probe must return 4.", duz=MARK), "UMBRELLA-MARK", 1)

# #3 (IMP): KIND-SPELLING is gated on the SPELLINGS, never on the winning
# kind.  `Population._kind` collects a spelling whether or not the row also
# carries the marker; nesting the finding under a non-empty `undetermined` set
# suppressed it exactly when every spelling-carrying row is also marked.
acase("POSITIVE", "(rc) two MARKED rows spelling the undetermined kind two ways is KIND-SPELLING: the "
                  "undetermined set is empty and the divergence is still real",
      build(s9z="charter.  KIND UNDETERMINED for now.", wa="why.  KIND — UNDETERMINED for now."),
      "KIND-SPELLING", 1)
rcase("POSITIVE", "(rc) that divergence is a mechanical finding: rc 1",
      build(s9z="charter.  KIND UNDETERMINED for now.", wa="why.  KIND — UNDETERMINED for now."), "", 1)
acase("NEGATIVE", "(rc) two MARKED rows spelling it the SAME way: KIND-SPELLING 0",
      build(s9z="charter.  KIND UNDETERMINED for now.", wa="why.  KIND UNDETERMINED for now."),
      "KIND-SPELLING", 0)

# #4 (IMP): a table's id column is keyed by the kinds its schema declares
# (`Schema.kinds`, applied in the ONE place `bare_id`).  A citation-shaped id
# in a slice or slot table entered `ids` as an umbrella while both mention
# passes ignored it (a citation id is masked in prose and exempt from the
# reference walk), so its ownership text could never be checked and the run
# exited 0; a short id in the citation table polluted the keep-set.
CITE_TABLE = ("## §0.6 more citations\n\n| ID | Citation | Anchor | Used by |\n|---|---|---|---|\n"
              "| %s | ECMA-262 §2 Y | `#b` | — |\n")
rcase("POSITIVE", "(schema) a citation-shaped id in the §5 SLICE table keys no slice row (the schema keys "
                  "short + slug): the row is unkeyed, so its cells would go unasserted -- rc 2",
      build(i7z="**[C99]**"), "", 2)
rcase("POSITIVE", "(schema) a citation-shaped id in the §8 SLOT table is the same miss -- rc 2",
      build(extra="## §8 more slots\n\n| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
                  "| **[C98]** | Terminal.  why. | now | 2026-12-31 |\n"), "", 2)
rcase("POSITIVE", "(schema) a SHORT id in the citation table keys no citation row (the schema keys cite): "
                  "rc 2 -- the inverse direction, which would otherwise pollute the keep-set",
      build(extra=CITE_TABLE % "**9y**"), "", 2)
rcase("POSITIVE", "(schema) a `#11-` SLUG in the citation table is the same miss -- rc 2",
      build(extra=CITE_TABLE % "`#11-zz-gamma`"), "", 2)
rcase("NEGATIVE", "(schema) a citation id in the citation table is admitted: rc 0 -- the three controls above "
                  "have a live subject",
      build(extra=CITE_TABLE % "[C9]"), "", 0)

# -- the §3.0b CLOSED list's PROSE-AS-WRITTEN rows: each one's COST, measured.
# A construct the inline pass does not lex is not thereby unexamined -- its text
# is read by the scanners exactly as written, and these say what that buys and
# what it costs.  Without them the table's "read as written" column would be an
# assertion; with them a change of disposition turns a control red.
case("POSITIVE", "(§6.2) emphasis is LEXED (design re-gate 4): a declared id inside `*...*` IS a naming "
                 "site, because the delimiters render no character at all -- the `**9z**` DECORATION a row "
                 "id may carry is the exception `dispose` makes for it, not this pass's",
     build(), "See *Slice 9z* here.", 1)
case("POSITIVE", "(§6.2) a link WRAPPED in emphasis is still the memo link: emphasis-as-prose costs the "
                 "population nothing (commonmark.js: `<em><a href=\"child.md\">x</a></em>`)",
     build(), "See *[x](child.md)* here.", 1, files=CHILD)
case("POSITIVE", "(§6.7) a HARD line break inside a link's text does not break the link: the inline pass reads "
                 "the block's joined text, so a construct may span the lines of its paragraph (§6.8 soft breaks "
                 "likewise)",
     build(), "See [x  \ny](child.md) here.", 1, files=CHILD)
case("POSITIVE", "(§2.5) a character reference in PROSE renders its character, so an id SPELLED as one "
                 "(`Slice &#57;z` renders `Slice 9z`) IS a naming site: the reference and the §2.4 escape "
                 "are the two spellings of one substitution, at the one site the stream builds (design "
                 "re-gate 4; until then §2.5 was decoded in a link DESTINATION only and this measured 0)",
     build(), "See Slice &#57;z here.", 1)

# ------------------------------- PR #510 design re-gate 4: the RENDERED text --
# The stream IS what the document renders (`plan_memo_tables.stream`), so an id
# or the kind marker split by a construct that renders NOTHING is read as the
# one unit a reader reads.  Two probe rows make the reading falsifiable: an
# UMBRELLA `Wz` and a TERMINAL `W`, so the wrong reading of `W**z**` reports
# nothing where the right one reports a site (the LOST-SITE axis), and the
# mirror table -- terminal `Wz`, umbrella `W` -- reports a site where the right
# reading reports none (the FABRICATED-SITE axis).  Each shape below is a
# construct of the §3.0b list that contributes no character, measured against
# commonmark.js 0.31.2 (`~~` against GitHub's GFM pipeline).
SPLIT = ("## §5 split probe\n\n| # | Slice | Primary module(s) | Slot | Tier | Deps |\n"
         "|---|---|---|---|---|---|\n"
         "| **Wz** | **%s** charter. | `w.rs` | — | T1 | — |\n"
         "| **W** | **%s** Acceptance: must. | `x.rs` | — | T1 | — |")
UMB = "UMBRELLA, not a terminal unit."
LOST = SPLIT % (UMB, "Terminal.")        # `Wz` is the umbrella: a split id LOSES the site
FAB = SPLIT % ("Terminal.", UMB)         # `W` is the umbrella: a split id FABRICATES one

for name, prose in [
        ("§6.2 strong emphasis `W**z**`", "Slice W**z** owns it."),
        ("§6.2 emphasis `W*z*`", "Slice W*z* owns it."),
        ("GFM strikethrough `W~~z~~`", "Slice W~~z~~ owns it."),
        ("§6.6 a raw HTML tag pair `W<b>z</b>`", "Slice W<b>z</b> owns it."),
        ("§6.6 an HTML comment `W<!-- c -->z`", "Slice W<!-- c -->z owns it."),
        ("§6.3 a link's brackets `W[z](slice-9z-sib.md)`", "Slice W[z](slice-9z-sib.md) owns it."),
        ("§2.5 a character reference `W&#122;`", "Slice W&#122; owns it."),
]:
    case("POSITIVE", "(render) %s renders `Wz` and names the UMBRELLA row `Wz`: a construct that "
                     "contributes no character does not split a token" % name,
         build(extra=LOST), prose, 1)
    case("NEGATIVE", "(render) %s names no row on the mirror table, where `W` is the umbrella and `Wz` "
                     "terminal: the old reading fabricated a site on `W`" % name,
         build(extra=FAB), prose, 0)

case("NEGATIVE", "(render) `Slice W*z owns it`: an UNMATCHED `*` run is literal text (§6.2 -- commonmark.js "
                 "renders `9*z` verbatim), so it BOUNDS the token and `Wz` is not named -- dropping a run "
                 "that pairs with nothing would fabricate the row",
     build(extra=LOST), "Slice W*z owns it.", 0)
case("NEGATIVE", "(render) `Slice W_z_ owns it`: an intraword `_` run can neither open nor close (§6.2 "
                 "rules 5-6, the `snake_case` rule), so it is literal and bounds the token",
     build(extra=LOST), "Slice W_z_ owns it.", 0)
case("NEGATIVE", "(render) `Slice W~~~z~~~ owns it`: three tildes are no strikethrough (GFM: a matching "
                 "pair of ONE OR TWO), so the run is literal text and bounds the token",
     build(extra=LOST), "Slice W~~~z~~~ owns it.", 0)
case("POSITIVE", "(render) `Slice [W](slice-9z-sib.md)z owns it` names `Wz`: the link's tail renders "
                 "nothing, and the `.md` file token INSIDE it is dropped with it -- where a drop and a "
                 "blank overlap the drop wins, or the tail's two sides stay apart",
     build(extra=LOST), "Slice [W](slice-9z-sib.md)z owns it.", 1)
case("NEGATIVE", "(render) `Slice W`z` owns it`: a code span RENDERS TEXT this checker refuses to read "
                 "(I-A), so it blanks and bounds the token -- the residue the `[LEX-SPLIT?]` seed reports "
                 "rather than decides",
     build(extra=LOST), "Slice W`z` owns it.", 0)
acase("POSITIVE", "(render) that same code span IS the `[LEX-SPLIT?]` residue seed: the reader reads `Wz` "
                  "across a span the disposition blanks, and the seed says so",
      build(extra=LOST), "LEX-SPLIT?", 1, prose="Slice W`z` owns it.")
acase("NEGATIVE", "(render) `Slice W**z** owns it` seeds NOTHING: the construct renders no character, so "
                  "the two readings agree and there is no residue to report",
      build(extra=LOST), "LEX-SPLIT?", 0, prose="Slice W**z** owns it.")

# The DECORATION exception: `**id**` where the id is declared is the document
# decorating it (the same `id_only` predicate the code-span disposition uses),
# so its delimiters STAND and bound the token.
case("POSITIVE", "(render) `**9z**7z` still names `9z`: a `**` pair whose content is only declared ids is "
                 "the document DECORATING an id, so the delimiters stand and bound it -- dropping them "
                 "would leave the single token `9z7z`, which no row declares",
     build(), "**9z**7z owns it.", 1)
case("NEGATIVE", "(render) `*9z*7z` names nothing: a SINGLE `*` is not this document's decoration "
                 "(`plan_memo_ids.DECOR` is `**` and a backtick), so the pair renders away and `9z7z` is "
                 "one token -- the exception is the decoration's, not emphasis's",
     build(), "*9z*7z owns it.", 0)
case("POSITIVE", "(render) an id-only code span is still the document SPELLING an id: `` `9z` `` is a "
                 "naming site, decoration and all",
     build(), "See `9z` here.", 1)
case("NEGATIVE", "(render) `\\*\\*C\\*\\* is how the row is written`: an ESCAPED decoration character "
                 "stands as written -- substituting it would spell a `**C**` bold the document does not "
                 "have, and the undecorated single letter is the declared miss",
     build(), "\\*\\*C\\*\\* is how the row is written.", 0)
case("NEGATIVE", "(render) `&#42;&#42;C&#42;&#42;` is the same rule for §2.5: a reference that would "
                 "spell a decoration stands as written",
     build(), "&#42;&#42;C&#42;&#42; is how the row is written.", 0)

# The licensing rule, the seeds' vocabularies and the kind reader all read that
# one stream, so a phrase split by emphasis is the phrase (three sites of the
# #506 memo moved from REPORTED to LICENSED for exactly this reason).
case("NEGATIVE", "(render) `each of *9z's* children`: the licensing phrase is read as the document "
                 "renders it, so the emphasis no longer hides a licensed construction",
     build(), "That is what makes each of *9z's* children a permissible PR.", 0)
case("NEGATIVE", "(render) `9z's **charter**`: the licensed noun carries the decoration, and the phrase "
                 "is licensed all the same",
     build(), "The obligation is stated against 9z's **charter**.", 0)
case("NEGATIVE", "(render) `9z is an *umbrella* row`: same, for the kind-statement arm of the licence",
     build(), "§5 row 9z is an *umbrella* row whose derivation mints its children.", 0)

# The KIND readers read the same stream: the marker, the `DECLARES` near-miss,
# the undetermined and pointer spellings, the acceptance and ownership
# vocabularies.  A row whose declaring field spells the marker across emphasis,
# a comment, a character reference or an escape DECLARES it -- the rendered
# document says umbrella -- and a prose mention of that row is a site exactly
# while the row is in the census, which is what these measure.
SPLIT_SLOT = ("## §8 split probe\n\n| Slot | Why deferred | Trigger | Re-eval |\n|---|---|---|---|\n"
              "| `#11-zz-gamma` | **%s** why. | now | 2026-12-31 |")
GAMMA = "Slot #11-zz-gamma lands before Slice 7z."

for name, phrase in [
        ("plain", UMB),
        ("§6.2 emphasis inside it (`not a *terminal* unit`)", "UMBRELLA, not a *terminal* unit."),
        ("GFM strikethrough inside it", "UMBRELLA, not a ~~terminal~~ unit."),
        ("§6.6 a tag pair inside it", "UMBRELLA, not a <b>terminal</b> unit."),
        ("§6.6 a comment inside it", "UMBRELLA, not a<!-- sic --> terminal unit."),
        ("§2.5 a character reference for the comma", "UMBRELLA&#44; not a terminal unit."),
        ("§2.4 an escaped comma", "UMBRELLA\\, not a terminal unit."),
]:
    case("POSITIVE", "(kind) a declaring field spelling the marker with %s DECLARES the umbrella: the row "
                     "is in the census, so a prose mention of it is a site" % name,
         build(extra=SPLIT_SLOT % phrase), GAMMA, 1)
case("NEGATIVE", "(kind) the marker QUOTED WHOLE in a code span declares nothing (I-A: a quoted marker is "
                 "not a declaration) -- the row is terminal and its mention is no site",
     build(extra=SPLIT_SLOT % ("Terminal.  Not `%s` at all." % UMB)), GAMMA, 0)
rcase("POSITIVE", "(kind) a declaring field spelling the marker ACROSS a code span is the schema miss, rc 2: "
                  "a reader reads the marker, the disposed stream does not, and §1 forbids a clean exit for "
                  "a could-not-scan over the census",
      build(extra=SPLIT_SLOT % "UMBRELLA, not a `terminal` unit."), GAMMA, 2)
rcase("NEGATIVE", "(kind) a declaring field holding a code span and NO marker is no miss: rc 0 -- the "
                  "residue is a marker the reader reads ACROSS a blanked span, never the presence of one",
      build(extra=SPLIT_SLOT % "Terminal.  See `terminal` above."), GAMMA, 0)
rcase("NEGATIVE", "(kind) nor is a field carrying the marker WHOLE beside a code span: rc 0",
      build(extra=SPLIT_SLOT % ("%s  See `terminal` above." % UMB)), GAMMA, 0)

acase("POSITIVE", "(kind) the appositive attribution reads the rendered field too: `Slice **9z** — "
                  "**UMBRELLA, not a *terminal* unit.**` is the UMBRELLA-MARK finding, so the pointer row "
                  "stays out of the census",
      build(sqx="Slice **9z** — **UMBRELLA, not a *terminal* unit.**"), "UMBRELLA-MARK", 1)
acase("POSITIVE", "(kind) the `DECLARES` near-miss backstop is the same reading: `is an *umbrella*` in a "
                  "declaring field without the marker is the seed",
      build(sqx="Terminal-ish.  This row is an *umbrella* by the derivation.  Acceptance: must."),
      "UMBRELLA-MARK?", 1)
case("POSITIVE", "(kind) the UNDETERMINED spelling read across emphasis: `KIND — UNDE*TER*MINED` puts the "
                 "row in the no-owner census, so its mention is a site",
     build(sqx="KIND — UNDE*TER*MINED until the probe runs."), "Slice Qx lands before Slice 7z.", 1)
acase("NEGATIVE", "(kind) the POINTER spelling read across a comment: the row is a POINTER, and a pointer "
                  "row owes no acceptance condition -- so the ACCEPT-VOCAB? seed stays silent where reading "
                  "the comment as text would have made the row an active terminal owing one",
      build(sqx="This row is a pointer<!-- sic --> rather than a slice."), "ACCEPT-VOCAB?", 0)
acase("NEGATIVE", "(vocab) acceptance vocabulary split by emphasis is acceptance vocabulary: a terminal row "
                  "writing `m*us*t` owes no ACCEPT-VOCAB? seed",
      build(s7z="Terminal.  The probe m*us*t return 3."), "ACCEPT-VOCAB?", 0)
acase("POSITIVE", "(vocab) the ownership connective is read the same way: `own*ed by* **9z** and **Qx**` is "
                  "the TWO-OWNERS? clause",
      build(s9z="charter.  The surface is own*ed by* **9z** and **Qx**."), "TWO-OWNERS?", 1)
acase("NEGATIVE", "(cell) a `Deps` cell holding only an HTML comment is EMPTY: it renders nothing, so the "
                  "umbrella row carries no Deps edge (`is_empty` reads the cell's stream, not its raw text)",
      build(d9z="<!-- none yet -->"), "UMBRELLA-CELL", 0)
acase("POSITIVE", "(cell) a `Deps` cell naming a row is still an edge: the control above has a live subject",
      build(d9z="**7z**"), "UMBRELLA-CELL", 1)

# The two line-break rows of §3.0b, whose cost is measured against the same probe
# table: a break renders a line ending, and a line ending bounds every unit.
case("POSITIVE", "(§6.7) a hard line break's own markup stands in the stream -- `\\` before a line ending "
                 "escapes nothing (§2.4 is ASCII punctuation, and a line ending is none) -- and costs "
                 "nothing: the break renders a line ending, which bounds every unit the scanners read, so "
                 "`Slice W\\` + `z` names no `Wz` on the probe table where `Wz` is the umbrella",
     build(extra=LOST), "Slice W\\\nz owns it.", 0)
case("POSITIVE", "(§6.8) a SOFT line break is the same measurement: `Slice W` then `z` on the next line "
                 "renders two words and names no `Wz`",
     build(extra=LOST), "Slice W\nz owns it.", 0)
case("NEGATIVE", "(render) `**9z**'s children`: the licensing rule needs no decoration spelling of its "
                 "own -- the mention's token consumes the `**` it carries, so the licensed noun is what "
                 "follows it",
     build(), "That is what **9z**'s children are for.", 0)
