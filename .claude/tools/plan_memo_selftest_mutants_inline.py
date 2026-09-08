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

from plan_memo_selftest_mutants import (
    CHECK, IDS, INLINE_EXAMPLES, LEXER, MEMO, MUTANTS, ROLES, SEQUENCE, TABLES,
)

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
     '    out += [(a, b, "html") for a, b in lx.html]\n', '',
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
    ("R19 #2 display: a memo is named by its path relative to the root memo's directory (re-inject the basename)", MEMO,
     '            return str(path.relative_to(self.root))', '            return path.name',
     [R19_DISPLAY]),
    ("R19 #3 sibling: the decoded name must be relative under Windows path syntax on every platform (re-inject the "
     "`/`-only test)", MEMO,
     '        if _CONTROL.search(name) or p.anchor:                        # (c)',
     '        if _CONTROL.search(name) or name.startswith("/"):            # (c)',
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
      "platform-independent) -- drive-relative, rejected, rc 0; the multi-letter `notes%3Achild.md` of R8 stays a "
      "file name"]),
    ("R19 #3 sibling: a backslash separates on every platform (re-inject the POSIX reading: `\\` a name character)", MEMO,
     '        return _resolve(self.path.parent.joinpath(*p.parts))         # (e)',
     '        return _resolve(self.path.parent / name)                     # (e)',
     ["(link) `sub%5Cchild.md`: a backslash is a path separator on every platform (WHATWG URL `#path-state` step 1: for "
      "a special scheme -- `file` is one -- `\\` ends a segment as `/` does; `PureWindowsPath` is that syntax) -- the "
      "file `sub/child.md` is walked"]),
    # -- PR #510 Codex R20: row-id composers cover every row kind; the suffix-only file name
    ("R20 #1 file: the stem of a bare `.md` file name may be EMPTY (re-inject the >=1-character stem)", LEXER,
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))*%s(?!%s))',
     '|(?P<file>(?:[^\\s\\[\\]()<>`|]|\\([^\\s()]*\\))+%s(?!%s))',
     ["(file) `.md` alone is a file name (the suffix-only name `sibling_path` accepts): beside a declared no-owner id "
      "`md`, `Read .md for details` reports 0 sites"]),
    ("R20 #1 sibling: stage (d) is the lexer's FILE_SUFFIX test alone (re-inject a stem requirement on the memo side)",
     MEMO,
     '        if not name.endswith(FILE_SUFFIX):                           # (d)',
     '        if not name.endswith(FILE_SUFFIX) or name == FILE_SUFFIX:    # (d)',
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
     '    out += [(a, b, "autolink") for a, b in lx.autolinks]\n', '',
     [R21_URI_ID, R21_EMAIL_ID, R21_BACKTICK]),
    ("R21 #1 §6.4: a construct demoted into a resolved image's description renders no tag of its own "
     "(re-tag it `image`: the resolved-image count over-claims)", LEXER,
     '                images.append(out.pop()[:2] + ("demoted",))',
     '                images.append(out.pop()[:2] + ("image",))',
     [INLINE_EXAMPLES]),
    ("R21 #2 (a): the marker in the declaring field short-circuits the SEED only (re-inject the `continue` "
     "that skipped the out-of-field scan)", ROLES,
     '        if MARKER not in row.field and DECLARES.search(row.field):',
     '        if MARKER in row.field:\n            continue\n        if DECLARES.search(row.field):',
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