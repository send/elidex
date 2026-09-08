#!/usr/bin/env python3
"""PR #510 Codex R17-on mutants for `plan-memo-umbrella-check.py --self-test --mutants`.

The third mutant module, split from `plan_memo_selftest_mutants_pr510.py` at the
review-round seam the CONTROL registry is split at (`plan_memo_selftest_cases_inline.py`
holds the same rounds' controls): that module holds R1-R16 and the design
re-gates -- the lexical substrate and the block grammar -- and this one every
round from R17 on, which closed the Phase-2 INLINE construct family (R17 §6.6
raw HTML, R19 §6.4 images, R21 §6.5 autolinks and the §3.0b closed list) with
what landed beside them (R19's display name and path syntax, R20's row-kind
grammar and file token, R21's out-of-field marker, KIND-SPELLING and schema id
kinds, design re-gate 4's rendered-text stream).  It appends to the SAME
`MUTANTS` list -- one registry, one import site (the runner).  The rules are
`plan_memo_selftest_mutants.py`'s: the substring must occur EXACTLY ONCE in its
file, every named control must go red, a crash is a FAIL.  A mutant's control
lives in `plan_memo_selftest_cases_inline.py` under the same round label.
"""

from plan_memo_selftest_cases_sibling import R25_PER_PART, R25_RESERVED_NAMES
from plan_memo_selftest_mutants import (
    CHECK, EMPHASIS, IDS, INLINE_EXAMPLES, LEXER, MEMO, MUTANTS, POPULATION, ROLES, SEQUENCE,
    SIBLING, STAGE_C, TABLES,
)

# The R25-1 control names are COMPOSED by the cases module (one per member of
# `_RESERVED_CHARS`, from one format string), so they are read from there
# rather than transcribed: a transcription would fail loudly (`unknown control`
# is a FAIL) but would still be the grammar of a name spelled twice.

# -- PR #510 Codex R21 control names, spelled once (the §6.5 autolink family,
# the double marker, the spelling gate, the schema id kinds)
R21_REVIEWER = ("(autolink) the R21 reviewer's input `<https://example.com/[child](absent.md)>` is ONE §6.5 "
                "autolink: the brackets inside it are not link syntax, `absent.md` is no memo, rc 0")
R21_URI_ID = ("(autolink) a declared id inside a URI autolink is no naming site: the span is masked whole, "
              "exactly as a link's destination is -- 0 sites")
R21_EMAIL_ID = (
    '(autolink) a declared id inside an EMAIL autolink is no naming site either (the second §6.5 arm,'
    ' `mailto:` at the renderer) -- 0 sites.  The id is the whole local part: dropping the arm leaves'
    ' `9z` bounded by `<` and `@`, so the mutant HAS a site to report (an id glued to a domain label '
    '-- `ops@9z.example.com` -- is no site either way and proves nothing)')
R21_BACKTICK = ("(autolink) a backtick inside an autolink is consumed by it, not by a code span (the three "
                "delimiters are read left to right as met; commonmark.js agrees) -- 0 sites")
R21_SCHEME = (
    '(autolink) `<m:abc…>` is a ONE-character scheme and no autolink (§6.5 Example 609, 2-32 '
    'characters): the `<` is literal and the link INSIDE the would-be span is read (commonmark.js: '
    '`&lt;m:abc<a href="child.md">x</a>&gt;`).  The link sits inside the span so that widening the '
    'scheme to one character masks it and the control goes red')
R21_SPACE = (
    '(autolink) a SPACE inside the absolute URI ends it (§6.5 Example 608), so `<https://foo.bar/ …>`'
    ' is no autolink and no tag either -- the `<` is literal and the link INSIDE the would-be span is'
    ' read (commonmark.js: `&lt;https://foo.bar/ <a href="child.md">x</a>&gt;`).  The space sits in '
    'the URI TAIL, which is the class the grammar excludes it from: a space right after `<` fails at '
    'the SCHEME instead, so that shape leaves the tail-widening mutant alive and proves nothing')
R21_DOUBLE_MARK = ("(a) a row carrying the marker in its declaring field AND again in another cell is the "
                   "double marker the assertion forbids: UMBRELLA-MARK 1")
R21_SPELLING = ("(rc) two MARKED rows spelling the undetermined kind two ways is KIND-SPELLING: the "
                "undetermined set is empty and the divergence is still real")
R21_CITE_IN_SLICE = ("(schema) a citation-shaped id in the §5 SLICE table keys no slice row (the schema keys "
                     "short + slug): the row is unkeyed, so its cells would go unasserted -- rc 2")
R21_CITE_IN_SLOT = "(schema) a citation-shaped id in the §8 SLOT table is the same miss -- rc 2"
R21_SHORT_IN_CITE = ("(schema) a SHORT id in the citation table keys no citation row (the schema keys cite): "
                     "rc 2 -- the inverse direction, which would otherwise pollute the keep-set")
R21_SLUG_IN_CITE = "(schema) a `#11-` SLUG in the citation table is the same miss -- rc 2"

R17_LAZY = "a lazy schema header after a definition in a linked memo's quote is a table: id declared, kind umbrella, census +1"
R17_REVIEWER = ("(quote) the R17 reviewer's shape: `> [a]: /u` then an UNQUOTED schema header over a QUOTED delimiter row "
                "is a table in the quote (cmark-gfm: a definition is paragraph text until the paragraph ends, and the "
                "header is read out of its last line) -- the row declares its id")
R17_SPAN = ("(html) the R17 reviewer's shape `<span title=\"[child](absent.md)\">text</span>`: a bracket inside a "
            "double-quoted attribute value is no link -- nothing is walked, rc 0")
R17_ATTR_ID = ("(html) `<span title=\"Slice 9z owns it\">x</span>`: an id inside an attribute value is no naming site -- "
               "the span is masked whole (kind `html`), as a raw HTML-block line is raw")

MUTANTS += [
    # -- PR #510 Codex R17
    ("R17 #1 lazy header: a definition keeps the run open (re-inject the paragraph-only rule -- `cur` alone -- so "
     "the quote ends before the header is asked)", MEMO,
     'not ((cur or i == def_end) and table_header_at(lines, i, lazy))',
     'not (cur and table_header_at(lines, i, lazy))',
     # ⚠ not the item control: under this mutant the item ends before the header and the header opens a
     # table at the DOCUMENT level (its indented delimiter row is a delimiter there), so the id is declared
     # either way -- the item shape is discriminated by the block-sequence control alone
     [R17_LAZY, R17_REVIEWER, SEQUENCE]),
    ("R17 #2 §6.6: raw HTML is a span of the one inline pass (drop the `<` arm: a tag is text and its brackets "
     "are delimiters)", LEXER,
     '            m = _HTML_TAG.match(s, i)\n            if m is None:',
     '            m = None\n            if m is None:',
     [R17_SPAN, R17_ATTR_ID, INLINE_EXAMPLES,
      "(lex-seed) `<span title=\"Slice 9z owns it\">`: an inline span holding a declared id is seeded under the one "
      "raw-line rule"]),
    ("R17 #2 §6.6: an attribute value may be single- or double-quoted (drop the quoted arms: unquoted only)", LEXER,
     '_ATTR_VALUE = r"(?:[^ \\t\\n\\"\'=<>`]+|\'[^\']*\'|\\"[^\\"]*\\")"',
     '_ATTR_VALUE = r"(?:[^ \\t\\n\\"\'=<>`]+)"',
     [R17_SPAN, R17_ATTR_ID, INLINE_EXAMPLES,
      "(html) a single-quoted attribute value `<span title='[x](absent.md)'>` is a tag: rc 0"]),
    ("R17 #2 §6.6: an HTML comment is a raw span (drop the comment arm)", LEXER,
     '_COMMENT = r"!-->|!--->|!--.*?-->"', '_COMMENT = r"(?!)"',
     ["(html) an HTML comment `<!-- [x](absent.md) -->` is raw: rc 0",
      "(html) `<!-- Slice 9z -- owns it -->`: `--` inside a comment is allowed since 0.31 (§6.6: \"a string of "
      "characters not including the string -->\") -- the whole comment is masked",
      INLINE_EXAMPLES]),
    ("R17 #2 §6.6: the 0.31 comment grammar admits `--` inside (re-inject 0.30's exclusion)", LEXER,
     '!--.*?-->', '!--(?:(?!--).)*-->',
     ["(html) `<!-- Slice 9z -- owns it -->`: `--` inside a comment is allowed since 0.31 (§6.6: \"a string of "
      "characters not including the string -->\") -- the whole comment is masked",
      INLINE_EXAMPLES]),
    ("R17 #2 §6.6: a processing instruction is a raw span (drop the arm)", LEXER,
     '_PI = r"\\?.*?\\?>"', '_PI = r"(?!)"',
     ["(html) a processing instruction `<? [x](absent.md) ?>` is raw: rc 0",
      "(html) `<? Slice 9z owns it ?>`: a processing instruction is masked", INLINE_EXAMPLES]),
    ("R17 #2 §6.6: a declaration needs an ASCII letter after `<!` (re-inject `<!` + anything)", LEXER,
     '_DECLARATION = r"![A-Za-z][^>]*>"', '_DECLARATION = r"![^>]*>"',
     ["(html) `<! Slice 9z owns it>`: no ASCII letter after `<!`, no declaration -- the site is reported",
      "(html) `<! [x](absent.md)>`: a declaration needs an ASCII letter right after `<!` -- literal, rc 2"]),
    ("R17 #2 §6.6: a CDATA section is a raw span (drop the arm)", LEXER,
     '_CDATA = r"!\\[CDATA\\[.*?\\]\\]>"', '_CDATA = r"(?!)"',
     ["(html) a CDATA section `<![CDATA[ [x](absent.md) ]]>` is raw: rc 0",
      "(html) `<![CDATA[ Slice 9z owns it ]]>`: a CDATA section is masked", INLINE_EXAMPLES]),
    ("R17 #2 §6.6: an attribute is preceded by at least one space, tab or line ending (re-inject optional whitespace: "
     "Example 622's `<a href='bar'title=title>` becomes a tag)", LEXER,
     '_WS = r"(?:[ \\t]*\\n[ \\t]*|[ \\t]+)"', '_WS = r"(?:[ \\t]*\\n[ \\t]*|[ \\t]*)"',
     ["(html) `<a b=\"c\"d=9z>x</a>`: no whitespace before `d` (Example 622), no tag -- `9z` is prose",
      "(html) `<a b=\"c\"d=\"[x](absent.md)\">`: an attribute needs whitespace before it (Example 622) -- literal, "
      "the link is read, rc 2", INLINE_EXAMPLES]),
    ("R17 #2 seed: an inline raw HTML span is recorded for the LEX-UNSUPPORTED? seed (drop the record)", MEMO,
     '        self.raw.extend((lineno, text, "inline") for lineno, text in self._inline_raw())',
     '        pass',
     ["(lex-seed) `<span title=\"Slice 9z owns it\">`: an inline span holding a declared id is seeded under the one "
      "raw-line rule",
      "(lex-seed) … and that seed carries the `inline` reading"]),
    ("R17 #2 disposition: the raw HTML span is masked (drop the `html` kind from the disposition)", TABLES,
     '    base += [(a, b, "html") for a, b in lx.html]\n', '',
     [R17_ATTR_ID,
      "(html) a cell's `<span title=\"Slice 9z owns it\">` is masked by the same inline pass: no site"]),
]

R19_REVIEWER = ("(image) the R19 reviewer's input `![alt [docs](absent-file.md)](image.png)`: the image resolves, so its "
                "description is plain text (§6.4; commonmark.js: `<img alt=\"alt docs\">`) -- the link inside it is no "
                "memo link, nothing is walked, rc 0")
R19_NOT_WALKED = ("(image) `![alt [docs](child.md)](absent-image.md)`: neither the demoted link nor the image is a memo "
                  "link -- `child.md` is not walked (its violation is unreported) and the image's `.md` destination is "
                  "not probed")
R19_UNRESOLVED_KEEPS = ("(image) `![alt [docs](child.md)][missing]`: the image does NOT resolve, so `![alt` is literal "
                        "text and the link inside it IS a link (commonmark.js) -- `child.md` is walked and its "
                        "violation reported")
R19_NESTED = ("(image) a nested image in a resolved image `![a ![b [c](child.md)](i.png)](j.png)`: the inner image stays "
              "masked and the link inside it is demoted -- `child.md` is not walked")
R19_INNER_RESOLVES = ("(image) `![a ![b [c](child.md)](i.png)][missing]`: the OUTER image fails but the INNER one "
                      "resolves, and the link inside the inner description is demoted (commonmark.js: `<img alt=\"b "
                      "c\">`) -- `child.md` is not walked")
R19_OUTSIDE = ("(image) `![a ![b](i.png) [c](child.md)][missing]`: the link stands OUTSIDE the inner image's description "
               "and the outer image fails -- it is a link, `child.md` is walked")
R19_DEACTIVATED = ("(link) `[a ![b [c](child.md)](i.png)](parent.md)`: the inner link deactivated the outer `[` as it "
                   "closed, before the image demoted it (§6.3 \"links may not contain links\"; commonmark.js: `[a <img "
                   "alt=\"b c\">](parent.md)`) -- neither `child.md` nor the absent `parent.md` is linked: 0 sites, not "
                   "rc 2")
R19_DISPLAY = ("diagnostics name a memo relative to the root memo's directory: `a/child.md` and `b/child.md` are two "
               "files, and a memo outside that directory is named by its absolute path")

MUTANTS += [
    # -- PR #510 Codex R19
    ("R19 #1 §6.4: a resolved image's description is plain text -- a link recorded inside it is demoted (re-inject the "
     "immediate add: the link stays a link)", LEXER,
     '            while out and out[-1][0] > pos:\n                images.append(out.pop()[:2] + ("demoted",))',
     '            while False:\n                images.append(out.pop()[:2] + ("demoted",))',
     [R19_REVIEWER, R19_NOT_WALKED, R19_NESTED, R19_INNER_RESOLVES, R19_DEACTIVATED]),
    ("R19 #1 §6.4: the demoted link's tail stays masked (re-inject a plain drop: the tail is prose)", LEXER,
     '                images.append(out.pop()[:2] + ("demoted",))', '                out.pop()',
     ["(image) `![alt [b](9z)](i.png)`: the demoted link's tail stays masked -- the `9z` in its destination is not "
      "prose, 0 sites"]),
    ("R19 #1 §6.4: a failed reference inside a resolved image's description names no lost memo (drop the rule)", LEXER,
     '            while unresolved and unresolved[-1][0] > pos:\n                unresolved.pop()',
     '            while False:\n                unresolved.pop()',
     ["(image) `![alt [x][missing]](img.png)`: a failed reference inside a resolved image's description names no lost "
      "memo (resolved, it would have been demoted; commonmark.js: `alt=\"alt [x][missing]\"`) -- rc 0"]),
    ("R19 #1 §6.4: demotion is the RESOLVED image's (re-inject it on the failed image too: `![alt` literal, yet the "
     "link inside is stripped)", LEXER,
     '                i += 1                  # literal `]`; the opener is gone; the tail is NOT consumed',
     '                while out and out[-1][0] > pos:\n                    images.append(out.pop()[:2] + ("demoted",))\n                i += 1',
     [R19_UNRESOLVED_KEEPS, R19_OUTSIDE]),
    ("R19 #2 display: a memo is named by its path relative to the root memo's directory (re-inject the basename)", POPULATION,
     '            return str(path.relative_to(self.root))', '            return path.name',
     [R19_DISPLAY]),
    ("R19 #3 sibling: the decoded name must be relative under Windows path syntax on every platform (re-inject the "
     "`/`-only test)", SIBLING,
     # PR #510 R22 added the reserved-component clause to the same `if`; the
     # MUTATION is untouched -- stage (c) back to a leading-`/` test, which
     # also drops the reserved clause, so both this mutant's controls and
     # R22's device controls go red on it.
     STAGE_C,
     '    if _CONTROL.search(name) or name.startswith("/"):            # (c)',
     ["(rc) a percent-encoded Windows drive-absolute `C%3A%5Ctemp%5Cchild.md` (`C:\\temp\\child.md`) is rejected after "
      "decoding on every platform (stage c: a drive anchors): rc 0",
      "(rc) a percent-encoded backslash-rooted `%5Cchild.md` (`\\child.md`) is rejected (stage c: a root anchors): rc 0",
      "(rc) a percent-encoded UNC `%5C%5Cserver%5Cshare%5Cx.md` is rejected (stage c: a UNC prefix anchors): rc 0",
      "(rc) a raw `\\\\server\\share\\x.md` destination decodes (§2.4: `\\\\` is one backslash, `\\s` is literal) to "
      "the backslash-rooted `\\server\\share\\x.md` (commonmark.js: href `%5Cserver%5Cshare%5Cx.md`) and is rejected: "
      "rc 0",
      "(rc) drive-relative `C:child.md`: raw, it is a URL of scheme `c` (stage a); percent-encoded `C%3Achild.md` "
      "decodes to a drive-anchored name (stage c) -- both rejected, rc 0",
      "(rc) `n%3Achild.md`: a ONE-letter name before `:` is a Windows drive letter (URL `#path-state` step 1.4.1, "
      "platform-independent) -- drive-relative, rejected at stage (c) by the ANCHOR, rc 0.  ⚠ Since R25-1 the "
      "multi-letter `notes%3Achild.md` is refused at the same stage by the CHARACTER rule, so this no longer "
      "discriminates one-letter from multi-letter; what it still says is that the drive reading is applied on "
      "every platform"]),
    ("R19 #3 sibling: a backslash separates on every platform (re-inject the POSIX reading: `\\` a name character)",
     SIBLING,
     '    return _resolve(directory.joinpath(*p.parts))                # (e)',
     '    return _resolve(directory / name)                            # (e)',
     ["(link) `sub%5Cchild.md`: a backslash is a path separator on every platform (WHATWG URL `#path-state` step 1: for "
      "a special scheme -- `file` is one -- `\\` ends a segment as `/` does; `PureWindowsPath` is that syntax) -- the "
      "file `sub/child.md` is walked"]),
    # -- PR #510 Codex R20: row-id composers cover every row kind; the suffix-only file name
    ("R20 #1 file: the stem of a bare `.md` file name may be EMPTY (re-inject the >=1-character stem)", LEXER,
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))*%s(?!%s))',
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))+%s(?!%s))',
     ["(file) `.md` alone is a file name (the suffix-only name `sibling_path` accepts): beside a declared no-owner id "
      "`md`, `Read .md for details` reports 0 sites"]),
    ("R20 #1 sibling: stage (d) is the lexer's FILE_SUFFIX test alone (re-inject a stem requirement on the resolver "
     "side)",
     SIBLING,
     '    if not name.endswith(FILE_SUFFIX):                           # (d)',
     '    if not name.endswith(FILE_SUFFIX) or name == FILE_SUFFIX:    # (d)',
     ["(link) `[x](.md)` links the sibling file named `.md`: `sibling_path` stage (d) and the lexer's file token read "
      "the ONE `FILE_SUFFIX`, so the suffix-only name is a file on both sides"]),
    ("R20 #2 grammar: ROW_ID is every row kind (re-inject SHORT_ID only -- the appositive, OWNS_TWO and the anchored "
     "reading all lose the slug at once, since they compose the one alternation)", IDS,
     'ROW_ID = "(?:%s)" % "|".join(dict(KINDS)[k] for k in ROW_KINDS)',
     'ROW_ID = SHORT_ID',
     ["(a) the R20 reviewer's declaring field `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributes the marker to a "
      "SLUG row: the row is a pointer and the UMBRELLA-MARK attribution finding is emitted",
      "(a) the bold slug form `Slice **#11-zz-alpha** — **UMBRELLA, …**` attributes the marker too",
      "(d) `owned by `#11-zz-alpha` and **Qx**`: a SLUG owner and a short owner are a two-owner clause (`OWNS_TWO` "
      "composes `ROW_ID`)",
      "a marker naming another row does not enter the count",
      "PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the "
      "spelling sweep)"]),
    ("R20 #2 appositive: ROW_NOUN_ID composes ROW_ID (re-inject decorated_id(SHORT_ID) at the one composer)", TABLES,
     'ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(ROW_ID)',
     'ROW_NOUN_ID = ROW_NOUN_SEP + decorated_id(SHORT_ID)',
     ["(a) the R20 reviewer's declaring field `Slice `#11-zz-alpha` — **UMBRELLA, …**` attributes the marker to a "
      "SLUG row: the row is a pointer and the UMBRELLA-MARK attribution finding is emitted",
      "PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the "
      "spelling sweep)"]),
    ("R20 #2 anchored: the anchored reading admits every row kind (re-inject the short-only test)", CHECK,
     'if t is None or t.kind not in ROW_KINDS or t.id not in keep or t.id == b.self_id:',
     'if t is None or t.kind != "short" or t.id not in keep or t.id == b.self_id:',
     ["(c-seed) `Lands after Slice `#11-zz-alpha`` in a Slice cell whose Deps cell names only **9z**: the anchored "
      "reading admits the slug, so the prose names a party the cell does not carry -- ORDER-PROSE? 1",
      "PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half of the "
      "spelling sweep)"]),
    ("R20 #3 naming: a row with no id is named by its declaring locator (re-inject `%r` of the id: `row None`)",
     TABLES,
     '        if self.self_id is not None:\n            return repr(self.self_id)',
     '        if True:\n            return repr(self.self_id)',
     ["a row whose id cell declares no id is named by its declaring locator (`row <no id> at :LINE (token)`), never "
      "`row None`"]),
    ("R21 #1 §6.5: an autolink is one masked token of the inline pass (drop the arm: its brackets are "
     "delimiters again)", LEXER,
     '            m = _AUTOLINK.match(s, i)   # §6.5 before §6.6, the spec\'s order',
     '            m = None',
     [R21_REVIEWER, R21_URI_ID, R21_EMAIL_ID, R21_BACKTICK, INLINE_EXAMPLES]),
    ("R21 #1 §6.5: the email arm is an autolink too (drop it: the URI arm alone)", LEXER,
     '_AUTOLINK = re.compile("<(?:%s|%s)" % (_URI_AUTOLINK, _EMAIL_AUTOLINK))',
     '_AUTOLINK = re.compile("<(?:%s)" % (_URI_AUTOLINK,))',
     [R21_EMAIL_ID, INLINE_EXAMPLES]),
    ("R21 #1 §6.5: a scheme is 2-32 characters (widen to 1: `<m:abc>` becomes an autolink)", LEXER,
     '_URI_AUTOLINK = r"[A-Za-z][A-Za-z0-9+.-]{1,31}:[^\\x00-\\x20\\x7f<>]*>"',
     '_URI_AUTOLINK = r"[A-Za-z][A-Za-z0-9+.-]{0,31}:[^\\x00-\\x20\\x7f<>]*>"',
     [R21_SCHEME, INLINE_EXAMPLES]),
    ("R21 #1 §6.5: a space ends the absolute URI (admit it: `< https://foo.bar >` becomes an autolink)", LEXER,
     '[^\\x00-\\x20\\x7f<>]*>"',
     '[^\\x00-\\x1f\\x7f<>]*>"',
     [R21_SPACE, INLINE_EXAMPLES]),
    ("R21 #1 §6.5: the autolink span is MASKED (drop the disposition: its text is prose again)", TABLES,
     '    base += [(a, b, "autolink") for a, b in lx.autolinks]\n', '',
     [R21_URI_ID, R21_EMAIL_ID, R21_BACKTICK]),
    ("R21 #1 §6.4: a construct demoted into a resolved image's description renders no tag of its own "
     "(re-tag it `image`: the resolved-image count over-claims)", LEXER,
     '                images.append(out.pop()[:2] + ("demoted",))',
     '                images.append(out.pop()[:2] + ("image",))',
     [INLINE_EXAMPLES]),
    ("R21 #2 (a): the marker in the declaring field short-circuits the SEED only (re-inject the `continue` "
     "that skipped the out-of-field scan)", ROLES,
     # the marker match goes through `MARKER_RE` since PR #510 R22 (bounded);
     # the MUTATION is untouched -- one `continue` gating both halves.
     '        if not MARKER_RE.search(row.field) and DECLARES.search(row.field):',
     '        if MARKER_RE.search(row.field):\n            continue\n        if DECLARES.search(row.field):',
     [R21_DOUBLE_MARK]),
    ("R21 #3: KIND-SPELLING is gated on the spellings, not on the winning kind (re-nest it under a "
     "non-empty undetermined set)", CHECK,
     '    if len(pop.spellings) > 1:', '    if undet and len(pop.spellings) > 1:',
     [R21_SPELLING, "(rc) that divergence is a mechanical finding: rc 1"]),
    ("R21 #4: an id cell declares an id of a kind ITS schema keys (drop the kind test)", TABLES,
     'return t.id if t is not None and t.start == 0 and t.kind in kinds else None',
     'return t.id if t is not None and t.start == 0 else None',
     [R21_CITE_IN_SLICE, R21_CITE_IN_SLOT, R21_SHORT_IN_CITE, R21_SLUG_IN_CITE]),
    ("R21 #4: the §5 slice table keys short + slug (widen it to every kind)", TABLES,
     'decl="Slice", idc="#", kinds=ROW_KINDS)',
     'decl="Slice", idc="#", kinds=ROW_KINDS + ("cite",))',
     [R21_CITE_IN_SLICE]),
    ("R21 #4: the citation table keys citation ids (widen it to every kind)", TABLES,
     'idc="ID", kinds=("cite",))', 'idc="ID", kinds=("cite", "short", "slug"))',
     [R21_SHORT_IN_CITE, R21_SLUG_IN_CITE]),
]
# -- PR #510 design re-gate 4: the stream IS the rendered text.  The control
# names are spelled once here; each mutant's named control has its SUBJECT
# inside the span the mutation changes (the R21 class of survivor).
RG4_STRONG = ("(render) §6.2 strong emphasis `W**z**` renders `Wz` and names the UMBRELLA row `Wz`: a "
              "construct that contributes no character does not split a token")
RG4_EM = ("(render) §6.2 emphasis `W*z*` renders `Wz` and names the UMBRELLA row `Wz`: a construct that "
          "contributes no character does not split a token")
RG4_STRIKE = ("(render) GFM strikethrough `W~~z~~` renders `Wz` and names the UMBRELLA row `Wz`: a "
              "construct that contributes no character does not split a token")
RG4_TAG = ("(render) §6.6 a raw HTML tag pair `W<b>z</b>` renders `Wz` and names the UMBRELLA row `Wz`: a "
           "construct that contributes no character does not split a token")
RG4_COMMENT = ("(render) §6.6 an HTML comment `W<!-- c -->z` renders `Wz` and names the UMBRELLA row `Wz`: "
               "a construct that contributes no character does not split a token")
RG4_LINK = ("(render) §6.3 a link's brackets `W[z](slice-9z-sib.md)` renders `Wz` and names the UMBRELLA "
            "row `Wz`: a construct that contributes no character does not split a token")
RG4_REF = ("(render) §2.5 a character reference `W&#122;` renders `Wz` and names the UMBRELLA row `Wz`: a "
           "construct that contributes no character does not split a token")
RG4_MIRROR = ("(render) §6.2 strong emphasis `W**z**` names no row on the mirror table, where `W` is the "
              "umbrella and `Wz` terminal: the old reading fabricated a site on `W`")
RG4_DECOR = ("(render) `**9z**7z` still names `9z`: a `**` pair whose content is only declared ids is the "
             "document DECORATING an id, so the delimiters stand and bound it -- dropping them would leave "
             "the single token `9z7z`, which no row declares")
RG4_SINGLE = ("(render) `*9z*7z` names nothing: a SINGLE `*` is not this document's decoration "
              "(`plan_memo_ids.DECOR` is `**` and a backtick), so the pair renders away and `9z7z` is one "
              "token -- the exception is the decoration's, not emphasis's")
RG4_UNMATCHED = ("(render) `Slice W*z owns it`: an UNMATCHED `*` run is literal text (§6.2 -- commonmark.js "
                 "renders `9*z` verbatim), so it BOUNDS the token and `Wz` is not named -- dropping a run "
                 "that pairs with nothing would fabricate the row")
RG4_INTRAWORD = ("(render) `Slice W_z_ owns it`: an intraword `_` run can neither open nor close (§6.2 "
                 "rules 5-6, the `snake_case` rule), so it is literal and bounds the token")
RG4_TILDE3 = ("(render) `Slice W~~~z~~~ owns it`: three tildes are no strikethrough (GFM: a matching pair "
              "of ONE OR TWO), so the run is literal text and bounds the token")
RG4_DROPWINS = ("(render) `Slice [W](slice-9z-sib.md)z owns it` names `Wz`: the link's tail renders "
                "nothing, and the `.md` file token INSIDE it is dropped with it -- where a drop and a blank "
                "overlap the drop wins, or the tail's two sides stay apart")
RG4_ESCAPED_DECOR = ("(render) `\\*\\*C\\*\\* is how the row is written`: an ESCAPED decoration character "
                     "stands as written -- substituting it would spell a `**C**` bold the document does not "
                     "have, and the undecorated single letter is the declared miss")
RG4_REF_DECOR = ("(render) `&#42;&#42;C&#42;&#42;` is the same rule for §2.5: a reference that would spell "
                 "a decoration stands as written")
RG4_SEED = ("(render) that same code span IS the `[LEX-SPLIT?]` residue seed: the reader reads `Wz` across "
            "a span the disposition blanks, and the seed says so")
RG4_QUIET = ("(render) `Slice W**z** owns it` seeds NOTHING: the construct renders no character, so the two "
             "readings agree and there is no residue to report")
RG4_LOCATOR = ("a site read across a construct that renders nothing is reported at its raw column (the "
               "stream map)")
RG4_LINEAR = ("emphasis matching is linear: N unmatched delimiter runs cost O(N) work (the Appendix's "
              "openers_bottom)")
RG4_MARK_EM = ("(kind) a declaring field spelling the marker with §6.2 emphasis inside it (`not a "
               "*terminal* unit`) DECLARES the umbrella: the row is in the census, so a prose mention of it "
               "is a site")
RG4_MARK_COMMENT = ("(kind) a declaring field spelling the marker with §6.6 a comment inside it DECLARES "
                    "the umbrella: the row is in the census, so a prose mention of it is a site")
RG4_MARK_REF = ("(kind) a declaring field spelling the marker with §2.5 a character reference for the comma "
                "DECLARES the umbrella: the row is in the census, so a prose mention of it is a site")
RG4_MARK_ESC = ("(kind) a declaring field spelling the marker with §2.4 an escaped comma DECLARES the "
                "umbrella: the row is in the census, so a prose mention of it is a site")
RG4_GATE = ("(kind) a declaring field spelling the marker ACROSS a code span is the schema miss, rc 2: a "
            "reader reads the marker, the disposed stream does not, and §1 forbids a clean exit for a "
            "could-not-scan over the census")
RG4_NOT_GATE = ("(kind) a declaring field holding a code span and NO marker is no miss: rc 0 -- the "
                "residue is a marker the reader reads ACROSS a blanked span, never the presence of one")
RG4_QUOTED = ("(kind) the marker QUOTED WHOLE in a code span declares nothing (I-A: a quoted marker is not "
              "a declaration) -- the row is terminal and its mention is no site")
RG4_DEPS = ("(cell) a `Deps` cell holding only an HTML comment is EMPTY: it renders nothing, so the "
            "umbrella row carries no Deps edge (`is_empty` reads the cell's stream, not its raw text)")


# -- PR #510 Codex R23 control names, spelled once (the rendered-decoration
# disposition, the kind-phrase gate in both directions, the BOM, the linear
# demotion).  They are used by THIS registry only, so they are declared here
# and not in the base registry the two derived ones share.
R23_AST_NO = ('(R23 render) `The &ast; marks it` names NO row `ast`: the reference renders `*`, '
              'so the letters `a` `s` `t` are nowhere in the document a reader reads -- leaving '
              "the entity's SOURCE spelling in the stream fabricated the site")
R23_AST_YES = ('(R23 render) `The ast marks it` IS the site -- the discriminating half: the row '
               'is declared, the scan reaches that prose, and only the SPELLING differs')
R23_QX_NO = ('(R23 render) `Slice Q&ast;x owns it` names NO row `Qx`: the reference renders a '
             'character, so the span is a BLANK and bounds the two sides -- dropping it (the '
             'disposition for a construct that renders nothing) would join them into an id the '
             'document does not spell')
R23_UNDET = ('(R23 kind) `KIND UNDETER`MINED`` in a declaring field is the schema miss: a reader '
             'reads the undetermined kind (§6.1 -- the code span contributes `MINED` as plain '
             'text), the disposed stream reads none, and the row silently left the census as an '
             'active terminal at rc 0 -- the marker was gated, every OTHER kind phrase was not')
R23_STREAM = ('(R23 kind) `KIND `x` UNDETERMINED` is the SAME miss from the other side: the blank '
              'stands as spaces, so the STREAM reads the undetermined kind and a reader (`KIND x '
              'UNDETERMINED`) reads none -- a gate stated over one direction leaves the other '
              'authoritative')
R23_DECOR_KIND = ('(R23 kind) `KIND&ast;UNDETERMINED` is that same stream-side miss through a '
                  'BLANKED decoration spelling: the reader sees `KIND*UNDETERMINED` and no kind '
                  'at all.  It is the one control over what a reader SEES where such a span is '
                  'blanked -- render it as spaces and the two readings agree on a kind neither '
                  'should read')
R23_POINTER = ('(R23 kind) the POINTER phrase straddling a code span is the miss too -- the third '
               'member, gated by arriving in `KIND_PHRASES` and not by being named here')
R23_QUOTED = ('(R23 kind) a kind phrase QUOTED WHOLE is no miss: the two readings differ about '
              "it, and that difference is I-A's disposition (a quoted phrase declares nothing), "
              'not a could-not-scan -- the STRADDLE is the conjunct that tells them apart')
R23_CLEAN = ('(R23 kind) a field that spells the phrase CLEANLY and straddles a blank elsewhere '
             'is no miss: both readings say undetermined, so the census is not in doubt -- the '
             'DISAGREEMENT is the other conjunct')
R23_SEED = ('(R23 seed) the `[LEX-SPLIT?]` residue seed reads every kind phrase too, not only the '
            'marker: `KIND UNDETER`MINED`` in PROSE (no declaring field, so no census to gate) is '
            'the reported disagreement')
R23_BOM = ('(R23 memo) a linked memo whose FIRST block is a slice table still declares its rows '
           'when the file opens with a BOM: the signature is dropped where the text becomes '
           'lines, or the header row is not a header row and the census silently shrinks at rc 0')
R23_TWO_BOMS = ('(R23 memo) exactly ONE U+FEFF is dropped: a second is an ordinary character of '
                'the document and the table is lost to it again -- a strip that is a `lstrip` '
                'rewrites the document instead of reading its encoding')
R23_GATE_PROPERTY = ('PROPERTY: Population._kind reads every kind phrase from '
                     'plan_memo_tables.KIND_PHRASES, the tuple the residue gate iterates (a '
                     'fourth phrase cannot decide a kind without being gated)')
R23_LINEAR = ("a resolved image's demotion is linear: N nested images demote their descendants "
              'once, not once per enclosing image')

MUTANTS += [
    # -- PR #510 design re-gate 4
    ("RG4 stream: a matched §6.2 / GFM delimiter run renders NOTHING (drop the marks: the delimiters stand "
     "in the stream and split the token again -- the defect this round fixed)", TABLES,
     '                      for a, b in (op, cl)]', '                      for a, b in ()]',
     [RG4_STRONG, RG4_EM, RG4_STRIKE, RG4_MIRROR, RG4_MARK_EM]),
    ("RG4 stream: the DECORATION exception -- a `**` pair whose content is only declared ids stands "
     "(drop it: `**9z**` renders away and `**9z**7z` is one token)", TABLES,
     'if not (ch == "*" and use == 2 and id_only(_reading(rd, ob, ca), keep))', 'if True',
     [RG4_DECOR]),
    ("RG4 stream: the exception is the DECORATION's, not emphasis's (widen it to a single `*`: `*9z*7z` "
     "names `9z`)", TABLES,
     'ch == "*" and use == 2 and id_only(_reading', 'ch == "*" and use >= 1 and id_only(_reading',
     [RG4_SINGLE]),
    ("RG4 stream: the exception is `id_only`'s (widen it to every `**` pair: `W**z**` is decorated again "
     "and the split id comes back)", TABLES,
     'and use == 2 and id_only(_reading(rd, ob, ca), keep)', 'and use == 2 and True',
     [RG4_STRONG, RG4_MIRROR]),
    ("RG4 stream: a raw HTML span renders no character (re-inject it as text the checker refuses to read: "
     "it blanks and splits the token again)", TABLES,
     '"html": False, "link": False, "mark": False}', '"html": True, "link": False, "mark": False}',
     # ⚠ NOT the `Deps` control: a blanked comment is spaces, and a cell of spaces is as empty as
     # a cell of nothing -- the subject must be a TOKEN the blank would split
     [RG4_TAG, RG4_COMMENT, RG4_MARK_COMMENT]),
    ("RG4 stream: a link's TAIL renders no character (re-inject it as blanks)", TABLES,
     '"html": False, "link": False,', '"html": False, "link": True,',
     # ⚠ the tail must sit BETWEEN the token's halves: in `W[z](sib.md)` it follows them both and
     # blanking it splits nothing (measured -- that shape leaves this mutant alive)
     [RG4_DROPWINS]),
    # ⚠ RETIRED AT R24, and NOT because it was weakened: the mutant here was
    # "re-inject blank-wins" against `stream`'s `if v > disp[k]`, and its
    # control was RG4_DROPWINS -- `Slice [W](slice-9z-sib.md)z owns it` names
    # `Wz`, because the `.md` token INSIDE the link's tail was dropped with the
    # tail rather than blanking it apart.  R24 moved the file/cite reading onto
    # the block's RENDERING (`dispose` stage 2), where a link's tail is already
    # gone, so no token is ever found inside one and that overlap cannot arise.
    # Measured before retiring it, and RE-RUNNABLE rather than a count that
    # goes stale: re-inject blank-wins (`if v > disp[k]` ->
    # `if v > disp[k] or (v == 1 and disp[k] == 2)`) and run `--self-test`
    # plus the #506 `--worklist`.  At R24 NO control turned red and the census
    # was byte-identical -- the ordering has become a statement with no
    # witness.
    # The LINE stays (it is the spec's own statement, and the next mask kind
    # will need it); the mutant does not, because a mutant its control cannot
    # kill reports coverage this suite does not have.  The control stays too:
    # it still asserts a true and reachable thing, now for the structural
    # reason rather than the ordering one.
    ("RG4 stream: a §2.4 escape and a §2.5 reference SUBSTITUTE their character (drop both: the reference "
     "and the backslash are read as written again)", TABLES,
     '        (decor if ch in DECOR_CHARS else subst)[a] = (b, ch)', '        pass',
     [RG4_REF, RG4_MARK_REF, RG4_MARK_ESC]),
    ("RG4 stream: a substitution that would spell a DECORATION does not substitute (drop the carve: "
     "`\\*\\*C\\*\\*` becomes the bold `**C**` no reader sees)", TABLES,
     '        (decor if ch in DECOR_CHARS else subst)[a] = (b, ch)', '        subst[a] = (b, ch)',
     [RG4_ESCAPED_DECOR, RG4_REF_DECOR]),
    ("RG4 lexer: the §2.4 escape is a substitution (re-inject the bare skip: the backslash stands in the "
     "stream)", LEXER,
     '            subst.append((i, i + 2, s[i + 1]))  # §2.4: `\\[` renders the character alone',
     '            pass',
     [RG4_MARK_ESC]),
    ("RG4 lexer: a §2.5 reference in PROSE is a substitution (drop the record: only a destination decodes, "
     "as before R16's sibling rule was generalised)", LEXER,
     '                subst.append((i, m.end(), r))   # §2.5: the reference renders its character',
     '                pass',
     [RG4_REF, RG4_MARK_REF]),
    ("RG4 lexer: a link's `[` renders nothing (drop the mark: the opener splits the token)", LEXER,
     "            opens.append((pos, pos + 1))    # a link's `[` renders as nothing", '            pass',
     # the `[` is what sits between `W` and `z` in `W[z](sib.md)`; in `[W](sib.md)z` it precedes both
     [RG4_LINK]),
    ("RG4 §6.2: a `~` run of three or more is no delimiter (drop the GFM bound: `W~~~z~~~` strikes)",
     EMPHASIS, '_MAX_RUN = {"~": 2}', '_MAX_RUN = {}',
     [RG4_TILDE3]),
    ("RG4 §6.2: `_` opens only where §6.2 rules 5-6 allow (drop the arm: intraword `_` becomes emphasis "
     "and `snake_case` breaks)", EMPHASIS,
     '''    if ch == "_":
        return Delimiter(i, j, ch, left and (not right or _is_punct(prev)),
                         right and (not left or _is_punct(nxt)))''',
     '    if ch == "_":\n        pass',
     [RG4_INTRAWORD, INLINE_EXAMPLES]),
    ("RG4 §6.2: the rule of three (drop it: a `can_open` closer takes an opener the spec forbids it)",
     EMPHASIS,
     '    if (closer.can_open or opener.can_close) and closer.orig % 3 and (opener.orig + closer.orig) % 3 == 0:',
     '    if False:',
     [INLINE_EXAMPLES]),
    ("RG4 §6.2: the flanking rules read the character BEFORE the run (drop it: every run reads a line "
     "start)", EMPHASIS, '    prev = s[i - 1] if i else None', '    prev = None',
     # ⚠ not the UNMATCHED probe: a `*` that opens and never closes is unmatched under both readings
     # (measured) -- the subject must be a run whose CLOSING depends on what precedes it
     [RG4_EM, INLINE_EXAMPLES]),
    ("RG4 §6.2: `openers_bottom` memoises a failed search (drop it: a paragraph of unmatched runs is "
     "quadratic)", EMPHASIS, '            openers_bottom[key] = closer', '            pass',
     [RG4_LINEAR]),
    ("RG4 §6.4: emphasis inside a RESOLVED image's description is demoted -- plain string content, no "
     "`<em>` (drop the demotion: the tag count over-claims)", LEXER,
     '            dem_pair.append((pair_bottom, len(pairs)))', '            pass',
     [INLINE_EXAMPLES]),
    ("RG4 seed: the `[LEX-SPLIT?]` residue is reported (drop the loop: a unit read across a blanked span "
     "is silent again)", CHECK,
     '        for kind, text, off in split_units(b.lexed, keep):',
     '        for kind, text, off in []:',
     [RG4_SEED]),
    ("RG4 gate: a kind phrase straddling a blanked span in a DECLARING field is a schema miss (drop it: "
     "the row leaves the census at rc 0 -- §1's clean exit for a could-not-scan)", POPULATION,
     '        for name in kind_disagreements(row.cells[row.schema.decl].lexed):', '        for name in ():',
     [RG4_GATE, R23_UNDET, R23_STREAM]),
    ("RG4 residue: a unit WHOLLY inside a blanked span is no straddle (widen the predicate: a quoted "
     "marker becomes the schema miss I-A exists to prevent)", TABLES,
     '    return 0 < inside < b - a', '    return inside > 0',
     [RG4_QUOTED]),
    ("RG4 gate: the miss is the STRADDLE, not the presence of a span (re-inject the coarse test: any "
     "declaring field holding a code span becomes a schema miss)", POPULATION,
     '        for name in kind_disagreements(row.cells[row.schema.decl].lexed):',
     '        for name in ["marker"] if row.cells[row.schema.decl].lexed.code else []:',
     [RG4_NOT_GATE]),
    ("RG4 cell: `is_empty` reads the cell's disposed stream (re-inject the raw text: an HTML comment fills "
     "the cell)", ROLES,
     '        if not is_empty(_stream(row, "Deps")):', '        if not is_empty(row.col("Deps").text):',
     [RG4_DEPS]),
    ("RG4 report: a reporting coordinate goes through the stream map (drop it: the column is off by the "
     "characters the stream dropped)", CHECK,
     '        return self.at_raw(self.stream.at(i))\n\n    def at_raw(self, i):\n        return self.para.locate(i)',
     '        return self.at_raw(i)\n\n    def at_raw(self, i):\n        return self.para.locate(i)',
     [RG4_LOCATOR]),
]
# -- PR #510 Codex R22 control names, spelled once (the raw-line seed's file /
# citation disposition, the Windows device components, the phrase boundaries)
R22_SEED_FILE = ("(seed) a raw HTML-block line whose only declared id sits INSIDE a `.md` file name seeds "
                 "nothing: the same name in prose is masked and reports nothing, and a seed that fires "
                 "where prose would not is reporting a reading the document has no way to hold")
R22_SEED_INDENT = "(seed) the same for an INDENTED-CODE line: one disposition, both readings"
R22_SEED_CITE = ("(seed) a raw line whose only declared id is a CITATION seeds nothing -- the shared spans "
                 "SUBSUME the seed's former hand-written `kind != cite` filter, so the citation half has "
                 "one spelling and not two")
R22_NUL = ("(sibling) `NUL.md` is a DOS device, not a memo beside this one: the destination is no "
           "sibling even though a file of that name is there to read")
R22_DIR_NUL = ("(sibling) `NUL/child.md`: the device is read per PART of the parsed path -- a device "
               "DIRECTORY rejects the destination too.  The device sits in a NON-FINAL component on "
               "purpose: with it in the last one, a final-component-only reading passes this control "
               "and the mutant survives (measured -- the probe would have had another subject)")
R22_COM1 = ("(sibling) `com1.md`: the device names fold case (`ntpath._isreservedname` upper-cases "
            "the stem), so the lower-case spelling is the same device")
R22_TRAILING_SPACE = ("(sibling) `dir%20/child.md`: a component ending in a SPACE is reserved too -- Windows "
                      "strips the trailing run, so that component names a different directory there than here")
R22_PRN = ("(sibling) `prn%20.md`: the stem's trailing spaces are stripped before the device "
           "lookup, so `prn .md` is `PRN` (Microsoft, \"Naming Files, Paths, and Namespaces\")")
R22_UNDET_NESS = ("(kind) `The KIND UNDETERMINEDNESS metric must be recorded` declares no unsettled kind: "
                  "with a nonempty `Deps` cell the unbounded reading made the row no-owner and forced "
                  "`UMBRELLA-CELL`, rc 1")
R22_MANKIND = ("(kind) nor does `MANKIND UNDETERMINED by the probe` -- the boundary is on BOTH sides, "
               "and the left one is the half a right-only fix would leave authoritative")
R22_SUBUMBRELLA = ("(kind) `SUBUMBRELLA, not a terminal unit.` carries no marker: the row is terminal, out "
                   "of the no-owner census, and a prose mention of it is no site")
R22_UNITARY = "(kind) nor does `UMBRELLA, not a terminal unitary claim.` -- the right-hand edge"
R22_SLICER = ("(kind) `is a pointer rather than a slicer of work` is no pointer row: it is an active "
              "terminal, and a terminal row stating no acceptance condition owes the ACCEPT-VOCAB? "
              "seed -- which the unbounded reading suppressed")
R22_DECLARES = ("(kind) `not a terminal unitary claim` is not the kind vocabulary either: the seed half "
                "of assertion (a) reads the same bounded phrases the marker does")
R22_GRANDCHILD = ("(licence) `The grandchild of 9z` is not the licensing phrase `child of`: a licence "
                  "SUPPRESSES a report, so an unbounded edge there is the dangerous polarity -- the "
                  "mention is reported")
R22_MEMORANDUM = ("(licence) `9z's memorandum` is not the licensed possession `memo`: the right-hand edge "
                  "of the forward look, reported")

# The R22 seed's span mask, spelled once: two mutants patch the same clause.
R22_SEED_MASK = ("ids = sorted({t.id for t in tokens(line) if t.id in keep\n"
                 "                          and not any(a < t.idend and t.idstart < b for a, b, _ in spans)})")

MUTANTS += [
    # -- PR #510 Codex R22: one reading, one spelling -- the raw-line seed's
    # disposition, the reserved Windows component, the phrase boundaries
    ("R22 #1 seed: the raw line is masked by the file / citation spans the DISPOSITION reads (drop the "
     "mask: the seed reads the raw line again)", CHECK,
     R22_SEED_MASK, "ids = sorted({t.id for t in tokens(line) if t.id in keep})",
     [R22_SEED_FILE, R22_SEED_INDENT, R22_SEED_CITE]),
    ("R22 #1 seed: the mask is BOTH kinds (drop the citation half -- the hand-written filter the shared "
     "spans subsume)", CHECK,
     "            spans = file_and_cite_spans(line)",
     '            spans = [s for s in file_and_cite_spans(line) if s[2] == "file"]',
     [R22_SEED_CITE]),
    ("R22 #2 sibling: stage (c) rejects a reserved COMPONENT (drop the clause -- an empty anchor was the "
     "whole test until R22)", SIBLING,
     STAGE_C, "    if _CONTROL.search(name) or p.anchor:                        # (c)",
     [R22_NUL, R22_DIR_NUL, R22_COM1, R22_TRAILING_SPACE, R22_PRN]),
    ("R22 #2 sibling: EVERY part, not just the last (re-inject a final-component-only reading)", SIBLING,
     "            or any(_is_reserved_component(s) for s in p.parts)):",
     "            or any(_is_reserved_component(s) for s in p.parts[-1:])):",
     # R25-1 put a second clause under the same per-part reading, so its
     # non-final probe belongs to this mutant too: the CHARACTER half must
     # be read per part exactly as the device half is
     [R22_DIR_NUL, R22_TRAILING_SPACE, R25_PER_PART]),
    ("R22 #2 sibling: the device is the STEM before the first dot, case-folded, trailing spaces stripped "
     "(re-inject the whole component)", SIBLING,
     '    return part.partition(".")[0].rstrip(" ").upper() in _DEVICE_NAMES',
     '    return part.upper() in _DEVICE_NAMES',
     # ⚠ not R22_DIR_NUL: its device component carries no `.md`, so the whole
     # component IS the stem there and the mutant leaves that control green
     [R22_NUL, R22_COM1, R22_PRN]),
    ("R22 #2 sibling: the stem's case is folded (drop the fold)", SIBLING,
     '    return part.partition(".")[0].rstrip(" ").upper() in _DEVICE_NAMES',
     '    return part.partition(".")[0].rstrip(" ") in _DEVICE_NAMES',
     [R22_COM1, R22_PRN]),
    ("R22 #2 sibling: a component ending in a dot or a space is reserved as well (drop that half)", SIBLING,
     '    if part[-1:] in (".", " "):\n        return part not in (".", "..")',
     '    if False:\n        return part not in (".", "..")',
     [R22_TRAILING_SPACE]),
    ("R22 #3 phrase: the shared boundary composer is load-bearing (drop both lookarounds from `bounded`)",
     IDS,
     '    return "%s(?:%s)%s" % (BEFORE, phrase, AFTER)', '    return "(?:%s)" % phrase',
     [R22_UNDET_NESS, R22_MANKIND, R22_SUBUMBRELLA, R22_UNITARY, R22_SLICER, R22_DECLARES]),
    ("R22 #3 phrase: the MARKER is bounded (re-inject the bare phrase)", TABLES,
     "MARKER_RE = re.compile(bounded(re.escape(MARKER)))", "MARKER_RE = re.compile(re.escape(MARKER))",
     [R22_SUBUMBRELLA, R22_UNITARY]),
    ("R22 #3 phrase: UNDETERMINED is bounded (re-inject the bare phrase)", TABLES,
     'UNDETERMINED = re.compile(bounded(r"KIND\\s*[\u2014-]?\\s*UNDETERMINED"), re.IGNORECASE | re.ASCII)',
     'UNDETERMINED = re.compile(r"KIND\\s*[\u2014-]?\\s*UNDETERMINED", re.IGNORECASE | re.ASCII)',
     [R22_UNDET_NESS, R22_MANKIND]),
    ("R22 #3 phrase: POINTER is bounded (re-inject the bare phrase)", TABLES,
     'POINTER = re.compile(bounded(r"is a pointer rather than a slice"))',
     'POINTER = re.compile(r"is a pointer rather than a slice")',
     [R22_SLICER]),
    ("R22 #3 phrase: the DECLARES vocabulary is bounded (re-inject the bare alternation)", ROLES,
     '    bounded(r"is an umbrella|not a terminal unit|\u22653 intersecting|three intersecting|"\n'
     '            r"no canonical algorithm|edge-dense"),',
     '    (r"is an umbrella|not a terminal unit|\u22653 intersecting|three intersecting|"\n'
     '     r"no canonical algorithm|edge-dense"),',
     [R22_DECLARES]),
    ("R22 #3 phrase: the BACKWARD licensing look is bounded on its left (drop the edge)", ROLES,
     "    BEFORE +                                  # PR #510 R22, see LICENSE_AFTER",
     '    "" +                                      # PR #510 R22, see LICENSE_AFTER',
     [R22_GRANDCHILD]),
    ("R22 #3 phrase: the FORWARD licensing look is bounded on its right (drop the edge)", ROLES,
     '    r")" + AFTER,', '    r")",',
     [R22_MEMORANDUM]),
]

MUTANTS += [
    # -- PR #510 Codex R23
    ("R23 #1 stream: a §2.5 / §2.4 spelling of a DECORATION character is BLANKED (leave it standing "
     "as written: the entity's SOURCE letters are back where the id scanner reads them)", TABLES,
     '        if disp[a] == 0:                # inside a dropped or blanked span, that span wins\n'
     '            for k in range(a, b):\n                disp[k] = 1',
     '        pass',
     # ⚠ NOT the `Q&ast;x` control: standing as written keeps the two sides apart just as a blank
     # does, so only the FABRICATED-id half of the pair can see this mutation
     [R23_AST_NO]),
    ("R23 #1 stream: it is a BLANK and not a DROP (the character IS rendered, so its two sides are "
     "not one word: dropping it joins them into an id no reader reads)", TABLES,
     '                disp[k] = 1', '                disp[k] = 2',
     # ⚠ NOT the `&ast;` control: a drop removes the source letters too, so only the JOINING half
     # of the pair can see this mutation -- the two mutants partition the two ways to be wrong
     [R23_QX_NO]),
    ("R23 #1 stream: what a READER sees where a decoration spelling is blanked is the character it "
     "renders (leave it as spaces: the two readings agree on a kind neither of them should read)",
     TABLES,
     '            blank_at[a] = (b, ch)', '            pass',
     [R23_DECOR_KIND]),
    ("R23 #2 gate: the residue gate iterates EVERY member of KIND_PHRASES (truncate it to the "
     "first: the marker stays gated and every other phrase decides a kind unwatched again -- the "
     "R23 defect exactly)", TABLES,
     '    for name, rx in KIND_PHRASES:\n        hit = [(rd.blanks, m) for m in rx.finditer(rd)]',
     '    for name, rx in KIND_PHRASES[:1]:\n        hit = [(rd.blanks, m) for m in rx.finditer(rd)]',
     [R23_UNDET, R23_STREAM, R23_POINTER]),
    ("R23 #2 seed: the residue SEED reads every member too (truncate it to the first: a kind phrase "
     "read across a blank outside a declaring field is silent again)", TABLES,
     '    for name, rx in KIND_PHRASES:\n        out += [(name, m.group(0), st.at(m.start()))',
     '    for name, rx in KIND_PHRASES[:1]:\n        out += [(name, m.group(0), st.at(m.start()))',
     [R23_SEED]),
    ("R23 #2 gate: the two readings must DISAGREE about the phrase (drop the conjunct: a field that "
     "spells the phrase cleanly and straddles a blank elsewhere becomes a miss)", TABLES,
     '        if bool(hit) == bool(other):\n            continue', '        if False:\n            continue',
     [R23_CLEAN]),
    ("R23 #2 gate: the disagreement must come from a STRADDLE (drop the conjunct: a phrase quoted "
     "WHOLE becomes the miss I-A exists to prevent)", TABLES,
     '        if any(_straddles(blanks, m.start(), m.end()) for blanks, m in hit):', '        if True:',
     [R23_QUOTED]),
    ("R23 #2 gate: the READER's rendering is consulted (drop it: a phrase the reader reads across a "
     "blank is no longer a miss)", TABLES,
     '        hit = [(rd.blanks, m) for m in rx.finditer(rd)]', '        hit = []',
     # ⚠ the stream-side controls stay GREEN under this one, and must: they are the other direction
     [RG4_GATE, R23_UNDET]),
    ("R23 #2 gate: the STREAM's rendering is consulted too (drop it: a phrase only the stream reads "
     "-- a blank standing as spaces -- is no longer a miss)", TABLES,
     '        hit = hit or [(st.blanks, m) for m in other]', '        hit = hit or []',
     # ⚠ the reader-side controls stay GREEN under this one, and must
     [R23_STREAM, R23_DECOR_KIND]),
    ("R23 #2 _kind: every phrase it reads comes from KIND_PHRASES (re-inject a direct read: a "
     "phrase decides a kind without the tuple -- and so without the gate -- ever seeing it)",
     POPULATION,
     '        hit = {name: rx.search(row.field) for name, rx in KIND_PHRASES}',
     '        import plan_memo_tables\n'
     '        hit = {name: rx.search(row.field) for name, rx in KIND_PHRASES}\n'
     '        hit["pointer"] = hit["pointer"] or plan_memo_tables._APPOSITIVE.search(row.field)',
     [R23_GATE_PROPERTY]),
    ("R23 #3 memo: a leading U+FEFF is dropped where the text becomes lines (keep it: the first "
     "block's header row is not a header row, and the table nobody saw is no schema miss)", MEMO,
     '    if text[:1] == "\\ufeff":        # spelled as an escape: it is invisible\n'
     '        text = text[1:]',
     '    pass',
     [R23_BOM]),
    ("R23 #3 memo: exactly ONE is dropped (strip every leading one: a second U+FEFF is an ordinary "
     "character of the document and stripping it rewrites the document)", MEMO,
     '        text = text[1:]', '        text = text.lstrip("\\ufeff")',
     [R23_TWO_BOMS]),
    ("R23 #4 lexer: the demotion ranges are applied as a UNION, once (re-inject the per-close walk: "
     "N nested images re-tag one descendant N times -- the same OUTPUT, quadratic work, which is "
     "why only a work witness can see it)", LEXER,
     '    if not ranges:\n        return entries\n    edge = [0] * (len(entries) + 1)',
     '    for a, b in ranges:\n        for j in range(a, b):\n'
     '            entries[j] = entries[j][:width] + ("demoted",)\n    return entries\n'
     '    edge = [0] * (len(entries) + 1)',
     [R23_LINEAR]),
    ("R23 #4 lexer: the image range is recorded at the close (drop it: a nested image is a resolved "
     "image of its own and the tag count over-claims)", LEXER,
     '            dem_img.append((img_bottom, len(images)))', '            pass',
     [INLINE_EXAMPLES]),
]

# -- PR #510 Codex R24 control names, spelled once.
R24_RENDER_SWEEP = ("PROPERTY: the verdict is invariant under a §2.5 re-spelling of any prose character "
                    "the document renders the same (the rendered-text rule, swept position by position)")
R24_FILE_RENDERED = ("(R24 render) `9z&#32;notes.md owns it` names `9z`: §2.5 renders the reference as a "
                     "SPACE, so the file name begins at `notes` and the id stands beside it -- the raw "
                     "reading saw one unbroken run ending in `.md`, masked the whole of it, and the "
                     "ownership claim left the census at rc 0")
R24_DECOR_RENDERED = ("(R24 render) `**&#57;z**7z owns it` names `9z`: the pair's content RENDERS `9z`, so "
                      "the `**` decorate an id and stand as the boundary they are.  Asked of the source, "
                      "`id_only` read `&#57;z`, dropped the delimiters, and joined the two sides into the "
                      "token `9z7z` a reader never sees -- the mirror of the fabrication design re-gate 4 "
                      "closed, one spelling further out")
R24_DECOR_READING = ("(R24 render) ``**`x` 9z**7z owns it`` names nobody: a READER sees the document "
                     "bolding `x 9z`, which is prose and no decorated id, so the delimiters drop and the "
                     "sides join.  This is the control over WHICH rendering the exception is asked of -- "
                     "the checker's own disposed stream would show `9z` beside blanks, whitespace-separate "
                     "into an id-only run, and keep a decoration the document does not have")

MUTANTS += [
    # -- PR #510 Codex R24, FAMILY 1: the disposition's two stage-2 questions
    ("R24 F1 tables: the file/cite reading is taken over the block's RENDERING (re-inject the raw "
     "reading: a §2.5 reference that renders whitespace no longer bounds the file name)", TABLES,
     'lx.tokens = [(rd.at(a), rd.at(b), kind) for a, b, kind in file_and_cite_spans(rd)]',
     'lx.tokens = file_and_cite_spans(lx.text)',
     [R24_FILE_RENDERED, R24_RENDER_SWEEP]),
    ("R24 F1 tables: the decoration exception is asked of the RENDERED content (re-inject the raw "
     "content: `**&#57;z**` decorates an id the source does not spell)", TABLES,
     'id_only(_reading(rd, ob, ca), keep)', 'id_only(lx.text[ob:ca], keep)',
     [R24_DECOR_RENDERED, R24_RENDER_SWEEP]),
    ("R24 F1 tables: the exception reads what a READER sees, not the disposed stream (hand it the "
     "stream: a code span's blanks whitespace-separate into an id-only run)", TABLES,
     '    rd = stream(lx, reader=True)', '    rd = stream(lx)',
     [R24_DECOR_READING]),
]

# -- PR #510 Codex R24 FAMILY 2 control names, spelled once.
R24_NUL = ("(R24 §2.1) a link destination holding a literal U+0000 names the file the document "
           "renders: §2 replaces the NUL with U+FFFD before parsing, so `[x](child\0.md)` links "
           "`child<U+FFFD>.md`, that memo is walked and its rows are declared.  Left in, the NUL "
           "is an ASCII control, `link_destination` (§6.3) refuses the destination, and the memo "
           "-- with every violation in it -- left the census while the run could still exit 0")
R24_CR = ("(R24 §2.1) a memo whose lines end with a bare CARRIAGE RETURN is one document of many "
          "lines: §2.1's line ending is `\\n`, `\\r\\n`, or `\\r` not followed by `\\n`, so the "
          "tables parse and the rows are declared.  Read without that rule the whole file is one "
          "line, no table is admitted, and no schema miss says so")
R24_ENDINGS = ("PROPERTY: the census is the same under each of the three line endings CommonMark §2.1 "
               "recognises (LF, CRLF, a bare CR), written as bytes")
R24_BOM = ("(R23 memo) a linked memo whose FIRST block is a slice table still declares its rows "
           "when the file opens with a BOM: the signature is dropped where the text becomes "
           "lines, or the header row is not a header row and the census silently shrinks at rc 0")
R24_C0 = "a decoded destination with a C0 control character is rejected, never resolved"

MUTANTS += [
    # -- PR #510 Codex R24, FAMILY 2: §2's input preprocessing, as a unit
    ("R24 F2 memo: §2 replaces U+0000 with U+FFFD before parsing (drop it: the NUL is an ASCII "
     "control, the link destination is refused, and the memo it named leaves the census)", MEMO,
     'text.replace("\\0", "\\ufffd")', 'text',
     [R24_NUL]),
    ("R24 F2 memo: §2.1's OTHER two line endings become U+000A (drop the normalisation: with the "
     "reader's own translation switched off, a CR-ended memo is one line and holds no table)", MEMO,
     '    return _LINE_ENDING.sub("\\n", text.replace("\\0", "\\ufffd"))',
     '    return text.replace("\\0", "\\ufffd")',
     [R24_CR, R24_ENDINGS]),
    ("R24 F2 memo: a CR NOT followed by an LF is its own ending (drop that arm: `\\r\\n` folds and a "
     "bare `\\r` does not)", MEMO,
     '_LINE_ENDING = re.compile(r"\\r\\n|\\r")', '_LINE_ENDING = re.compile(r"\\r\\n")',
     [R24_CR, R24_ENDINGS]),
    # ⚠ NO MUTANT FOR `newline=""` ITSELF, and the reason is a measurement, not
    # an omission: restoring the reader's universal-newline translation while
    # KEEPING `_preprocess`'s normalisation changes nothing a control can see
    # (written and run at R24: the mutant survived both R24_CR and
    # R24_ENDINGS, because the two mechanisms do the same job).  `newline=""`
    # is not a behaviour of its own -- it is what makes the §2.1 rule THIS
    # unit's to state, and the proof of that is the pair above: with the
    # reader translating, "drop the normalisation" would survive too, and the
    # requirement would again be one nothing proves.
]

# -- PR #510 Codex R24 FAMILY 3 control names, spelled once.
R24_LONG_SLUG = ("(R24 width) a pointer row whose appositive names a 76-character slug attributes the "
                 "marker to that row: the appositive ends where the marker begins, which is a GRAMMAR "
                 "fact and not a character count.  Read through a 70-character window the appositive "
                 "fell outside it, the field was taken as the row's OWN declaration, no UMBRELLA-MARK "
                 "was emitted, and the census carried a pointer row as an umbrella at rc 0")
R24_MENTION_ONLY = ("(R24 width) a field that merely MENTIONS a sibling and then declares itself "
                    "attributes nothing: the discrimination is the DASH between the id and the marker, "
                    "never the distance -- so removing the window does not widen the attribution")
R24_VACUOUS = ("(R24 width) `xderivation … that the 9z` is REPORTED: the licensing phrase is 40 "
               "characters, exactly the width of the slice the backward look was given, so its "
               "lookbehind fell off the start of that slice and succeeded against nothing -- the "
               "document says `xderivation`, which is not the licensed phrase")
R24_TRAILING_NOUN = ("(R24 width) a row noun standing between the licensing phrase and the id does not "
                     "hide the phrase (`the child of Slice 9z`), and NOT because the rule has a clause for "
                     "one: the ANCHORED reading of that site starts AT the noun and wins the dedup, so the "
                     "text before the mention is `the child of` either way.  This is the control the "
                     "deleted `_TRAILING_NOUN` substitution was believed to be needed for")
R24_WIDTH_PROPERTY = ("PROPERTY: no ANCHORED pattern in the module set is handed a subject truncated by "
                      "a number (a width window is a second statement of what the anchor already says)")

MUTANTS += [
    # -- PR #510 Codex R24, FAMILY 3: a matcher's context boundary is the grammar's
    ("R24 F3 tables: the appositive is bounded by where the MARKER begins, not by a width (re-inject "
     "the 70-character slice: a longer declared slug pushes the appositive out of the window)", TABLES,
     '_APPOSITIVE.search(field, 0, m.start())', '_APPOSITIVE.search(field[max(0, m.start() - 70): m.start()])',
     [R24_LONG_SLUG, R24_WIDTH_PROPERTY]),
    ("R24 F3 tables: the appositive still requires the DASH (drop it: a field that merely mentions a "
     "sibling attributes to it, which is what the window was believed to prevent)", TABLES,
     r'_APPOSITIVE = re.compile(ROW_NOUN_ID + r"\s*[—–-]\s*" + DECOR + r"\s*$", re.ASCII)',
     r'_APPOSITIVE = re.compile(ROW_NOUN_ID + r"[^a-zA-Z]*" + DECOR + r"\s*$", re.ASCII)',
     [R24_MENTION_ONLY]),
    ("R24 F3 roles: the backward look reads the whole preceding text, bounded by `endpos` where `$` "
     "matches (re-inject the 40-character slice: a phrase that long starts at index 0 and the "
     "lookbehind succeeds against nothing)", ROLES,
     'LICENSE_BEFORE.search(m.text, 0, m.start)', 'LICENSE_BEFORE.search(m.text[max(0, m.start - 40):m.start])',
     [R24_VACUOUS, R24_WIDTH_PROPERTY]),
    # ⚠ THE SUBJECT OF THIS CONTROL IS THE DEDUP, not a clause of the licensing
    # pattern.  A mutant that dropped a trailing-row-noun clause SURVIVED it --
    # which is what said the clause was unreachable and got it deleted rather
    # than ported (see `LICENSE_BEFORE`).  What the control actually rests on
    # is that the ANCHORED reading wins, so the mutation is here: let the BARE
    # reading win, and `the child of Slice 9z` is read from `9z` with the row
    # noun in front of the phrase.
    ("R24 F3 check: the ANCHORED reading wins the mention dedup (let the bare one win: a licensing "
     "phrase separated from the id by a row noun no longer stands immediately before the mention)", CHECK,
     '        if prev is None or m.start < prev.start:', '        if prev is None or m.start > prev.start:',
     [R24_TRAILING_NOUN]),
    # The BARE pass reads the grammar's closed set, exactly as the anchored one
    # does (R24 collapsed the complement spelling `t.kind == "cite"` into it).
    # The mutation narrows the bare pass to ONE kind and leaves `_anchored`
    # untouched, which is precisely the divergence the complement made possible
    # and nothing could see: measured, only the `bare/slug` probes go red, so
    # the coverage control's new half has its own subject rather than riding on
    # the anchored one.
    ("R24 check: the BARE pass admits every ROW_KINDS kind, not one of them (narrow it to `short`: a "
     "slug named with no row noun before it stops being a naming site, while the anchored reading "
     "still reports it)", CHECK,
     'if t.kind not in ROW_KINDS or tid not in keep or tid == b.self_id:',
     'if t.kind != "short" or tid not in keep or tid == b.self_id:',
     ["PROPERTY: every row-id composer admits every row kind of plan_memo_ids.ROW_KINDS (the kind half "
      "of the spelling sweep)"]),
]

# -- PR #510 Codex R24-3, the one finding of the round that is in no family.
R24_SPAN_LOCATOR = ("a §6.6 span crossing a line ending seeds each of its lines at ITS line, not all of "
                    "them at the opener's")

MUTANTS += [
    ("R24 memo: a multiline §6.6 span's pieces are located at their OWN offsets (key them on the "
     "span's start instead: the seed sends the reader to the line the comment opened on, where the "
     "id it names is not)", MEMO,
     '                for off, piece in pieces(p.lexed, a, b):\n                    yield p.locate(off)[0], piece',
     '                for _off, piece in pieces(p.lexed, a, b):\n                    yield p.locate(a)[0], piece',
     [R24_SPAN_LOCATOR]),
]

# -- PR #510 Codex R25 control names, spelled once (this registry's only
# readers), and the two R25 mutants.
R25_HARD = ("(R25 §6.7) a HARD line break spelled with a BACKSLASH names the row across it: `Slice\\` + "
            "a line ending + `C` reads `Slice C` (§2.4, vendored Example 16: `foo\\` + an ending renders "
            "`<p>foo<br />` + `bar</p>`), so the umbrella row `C` is a naming site.  Until R25 the "
            "backslash stood in the stream, `NOUN_ANCHOR` could not reach the id, and the run exited 0 "
            "with no site at all")
R25_SOFT = ("(R25 §6.8) the SOFT-break partner, green before the fix: the same two words across a plain "
            "line ending name the same row -- this is the control that says the backslash case is about "
            "the BACKSLASH and not about the adjacency")
R25_BREAK_PROPERTY = ("PROPERTY: the verdict is invariant under re-spelling any ONE line break as each of "
                      "CommonMark's three (the §6.8 soft break, §6.7's two-space and backslash hard "
                      "breaks) -- the render-equivalence family's second guard, for the class its first "
                      "one excludes by construction")

R25_REVERSED = ("(link) `notes%3Achild.md` is NOT a sibling.  It has no scheme -- WHATWG URL §4.4 "
                "#scheme-start-state / #scheme-state read the input as written and `%` is in neither "
                "class, #string-percent-decode being a later, separate operation -- so stage (a) admits "
                "it; stage (c) then refuses it, because the decoded `notes:child.md` is an NTFS "
                "alternate data stream on Windows and a plain file name on POSIX, which is two readings "
                "where that stage allows one.  ⚠ THIS EXPECTATION IS A REVERSAL of PR #510 R8, decided "
                "at R25-1, and it costs a POSIX memo genuinely named `notes:child.md` -- dropped "
                "without a report, as `sub\\child.md` and `NUL.md` already are")

MUTANTS += [
    ("R25-1 sibling: stage (c) refuses a component holding a character Windows does not read as a "
     "letter of a name (drop the clause -- the reading that let an NTFS alternate data stream through)",
     SIBLING,
     "    if _RESERVED_CHARS.intersection(part):\n        return True",
     "    if False:\n        return True",
     # every member of the class, not just the one R25 reported: an enumerated
     # fix leaves the next member authoritative, and so would an enumerated
     # PROOF of one
     list(R25_RESERVED_NAMES) + [R25_REVERSED]),
    ("R25-2 lexer: a §6.7 hard line break's backslash renders NOTHING (drop the clause -- the literal "
     "backslash that stood in the stream until R25)", LEXER,
     '    return s[j] == "\\\\" and j + 1 < len(s) and s[j + 1] == "\\n"', "    return False",
     [R25_HARD, R25_BREAK_PROPERTY]),
    ("R25-2 lexer: the hard break's mark is the BACKSLASH alone, not the backslash AND the line ending "
     "(swallow the ending too: the break stops separating and the two lines become one word)", LEXER,
     "            marks.append((i, i + 1))", "            marks.append((i, i + 2))",
     [R25_HARD, R25_BREAK_PROPERTY]),
]
