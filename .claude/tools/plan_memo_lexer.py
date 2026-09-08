#!/usr/bin/env python3
"""Phase 2 of CommonMark 0.31.2 "Appendix: A parsing strategy" -- INLINE
structure -- for `plan-memo-umbrella-check.py`: a subset of CommonMark
0.31.2 and GFM 0.29, lexed by construction from the clauses
`docs/plans/2026-08-plan-memo-umbrella-checker.md` §3 lists.  Block
structure (Phase 1: fences, block starts, the one `block_end` predicate,
table rows, reference definitions) is `plan_memo_blocks.py`, which imports
this module's inline grammar; the order is plan §2 "Lexing order".

Per block inline content (a paragraph or a cell): code spans (CommonMark
§6.1, backtick strings of equal length), raw HTML (§6.6: an open tag, a
closing tag, an HTML comment, a processing instruction, a declaration or a
CDATA section -- ONE tag grammar, `_HTML_TAG`, whose open / closing tag
bodies `OPEN_TAG` / `CLOSING_TAG` are also §4.6 start condition 7's, read by
`plan_memo_blocks.py`) and links / images (§6.3 / §6.4) are lexed by ONE
left-to-right pass (`inline_pass`): a code span or a raw HTML span is
skipped as met (neither is inline-parsed: a bracket inside an attribute
value or a comment is not a link delimiter, PR #510 R17), an inline-link
tail is parsed by lookahead on the raw text -- there is no pre-mask of any
kind.  An image's bracket structure is parsed so that it is not a link and
a link may wrap it; its destination never joins the population, its alt
text is prose, its tail is masked, and when it RESOLVES its description is
plain text (§6.4): a link recorded inside it is demoted to a masked tail,
never a memo link (PR #510 R19).  A §6.5 autolink is ONE masked token, tried
at a `<` before the tag grammar (R21): its contents are not inline syntax and
its text is its own URL, so nothing inside it is a link, a naming site or a
sibling.  The inline constructs outside the lexed clauses -- §6.2 emphasis
beyond the decoration the id grammar reads, §6.7 hard and §6.8 soft line
breaks, §6.9 textual content, §2.5 character references in PROSE -- are read
as written, each with its cost stated in the plan's §3.0b CLOSED list; a
character reference in a link DESTINATION is decoded (§2.5 / §6.3,
`normalize_destination` -- the one place a destination's text is read, PR
#510 R16); the block types not modelled are the plan's §3.0 table.

`Lexed` is the one Phase-2 value per block: code spans, raw HTML spans,
links, images, and
the two bare tokens the scanners must not read an id out of (`[C19]`-style
citation ids, `.md` file names).  Nothing in this module knows what a ROW
id is: the citation shape and the ASCII boundary it composes are the id
grammar's (`plan_memo_ids.py`, below this module), and the disposition
exception (an id-only code span is the document spelling an id, not code)
is applied over a `Lexed` by `plan_memo_tables.py`.
"""

import bisect
import re
import string
from html.entities import html5

from plan_memo_ids import ALNUM, CITE_ID

ASCII_PUNCT = frozenset(string.punctuation)


# --------------------------------------------------------------------------
# CommonMark §6.1 code spans: a backtick string (a run of one or more
# backticks) opens a span closed by the NEXT backtick string of equal length;
# a string with no equal-length partner is literal, and scanning resumes after
# it.  A span may contain line endings, so the unit is the block's inline
# content, never a line.  Code spans and brackets are recognised by ONE
# left-to-right pass (`inline_pass`, below the link grammar).
# --------------------------------------------------------------------------

_BACKTICKS = re.compile(r"`+")


def _escaped(s, i):
    """Whether `s[i]` sits behind an ODD run of backslashes (§2.4: the pairs
    before it escape each other, the odd one escapes `s[i]`)."""
    k = i
    while k > 0 and s[k - 1] == "\\":
        k -= 1
    return (i - k) % 2 == 1


def blank_spans(s, spans):
    """`s` with every span replaced by spaces (line endings kept), so offsets
    survive and a later grammar cannot see inside a masked construct."""
    if not spans:
        return s
    buf = list(s)
    for a, b in spans:
        for k in range(a, b):
            if buf[k] != "\n":
                buf[k] = " "
    return "".join(buf)


# --------------------------------------------------------------------------
# CommonMark §6.3 links and §4.7 link reference definitions
# --------------------------------------------------------------------------


def _skip_ws(s, i, newlines=1):
    """Spaces, tabs and up to `newlines` line endings -- §6.3: the inline
    link's components "may be separated by spaces, tabs, and up to one line
    ending"; §4.7 allows the same separator between a definition's colon, destination and title."""
    seen = 0
    while i < len(s):
        if s[i] in " \t":
            i += 1
        elif s[i] == "\n" and seen < newlines:
            seen += 1
            i += 1
        else:
            break
    return i


# §2.5, the reference grammar: "Entity references consist of `&` + any of the
# valid HTML5 entity names + `;`" -- "Decimal numeric character references
# consist of `&#` + a string of 1–7 arabic digits + `;`" -- "Hexadecimal
# numeric character references consist of `&#` + either `X` or `x` + a string
# of 1-6 hexadecimal digits + `;`".  The name arm is a SHAPE (a letter, then
# letters and digits; the longest HTML5 name is 31 characters); whether the
# shape names an entity is the HTML5 list's to say (`html5`, `_reference`).
_CHAR_REF = re.compile(r"&(#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{1,31});")

# "The document https://html.spec.whatwg.org/entities.json is used as an
# authoritative source for the valid entity references and their
# corresponding code points" -- the stdlib ships that list as
# `html.entities.html5` (imported above), keyed WITH the `;` for the names
# CommonMark recognises and without it for HTML's legacy semicolon-less
# forms (`copy`), which §2.5 excludes: "Although HTML5 does accept some entity
# references without a trailing semicolon (such as `&copy`), these are not
# recognized here, because it makes the grammar too ambiguous" (Example
# 29).  Looked up with the `;`, so the legacy forms are never found.  NOT
# `html.unescape`: it decodes the legacy forms, and it is a second pass over
# text this module has already read once.


def _codepoint(n):
    """§2.5: "A numeric character reference is parsed as the corresponding
    Unicode character.  Invalid Unicode code points will be replaced by the
    REPLACEMENT CHARACTER (U+FFFD).  For security reasons, the code point
    U+0000 will also be replaced by U+FFFD."  Invalid = above U+10FFFF or a
    surrogate (commonmark.js 0.31.2: `&#xD800;` renders U+FFFD, measured)."""
    if n == 0 or n > 0x10FFFF or 0xD800 <= n <= 0xDFFF:
        return "\ufffd"
    return chr(n)


def _reference(m):
    """The character a `_CHAR_REF` match stands for, or None when its name is
    not an HTML5 entity (§2.5 Example 30: `&MadeUpEntity;` "not recognized as
    entity references either" -- literal text)."""
    body = m.group(1)
    if body[0] != "#":
        return html5.get(body + ";")
    return _codepoint(int(body[2:], 16) if body[1] in "xX" else int(body[1:]))


def normalize_destination(s):
    """The ONE normalisation of a link destination's raw text -- the inline
    link's (§6.3) and the reference definition's (§4.7), bare or in angle
    brackets -- in ONE left-to-right pass: a backslash escape (§2.4: a
    backslash before an ASCII punctuation character is removed; any other
    backslash is literal) yields its character, a character reference (§2.5)
    yields the character it stands for, anything else is read as written.

    §2.5, verbatim: "Valid HTML entity references and numeric character
    references can be used in place of the corresponding Unicode character,
    with the following exceptions: Entity and character references are not
    recognized in code blocks and code spans.  Entity and character
    references cannot stand in place of special characters that define
    structural elements in CommonMark." -- and, on where they ARE read:
    "Entity and numeric character references are recognized in any context
    besides code spans or code blocks, including URLs, link titles, and
    fenced code block info strings" (Examples 31-34).  §6.3 on the
    destination: "Entity and numerical character references in the
    destination will be parsed into the corresponding Unicode code points,
    as usual."  So `[child](child&#46;md)` and `[sib]: child&#46;md` both
    name `child.md` (commonmark.js 0.31.2, measured), and until PR #510 R16
    the destination was backslash-unescaped ONLY: `sibling_path` saw the
    literal `child&#46;md`, no `.md` suffix, and the sibling was silently
    outside the population (rc 0).

    ONE pass, so the two grammars meet at a character exactly once: a
    backslash-escaped `&` (`\\&#46;`) is a literal `&` and opens no
    reference; a decoded `&` (`&#x26;#46;`) is a character, never re-read as
    the start of a second reference -- commonmark.js renders `a\\&#46;b.md`
    and `a&#x26;#46;b.md` both as `a&#46;b.md` (measured).  What this does
    NOT touch: a link LABEL (§6.3 label matching normalises case and
    whitespace only -- `[foo&auml;]` and `[fooä]` are different labels in
    commonmark.js, measured -- `normalize_label` reads the raw label); a
    link TITLE (§2.5 decodes titles too, but `link_title` reads a title for
    its SHAPE only -- the end offset -- and never its text, so there is
    nothing to decode); a code span (§2.5's first exception; `inline_pass`
    jumps past a span and this function never sees one).  The decoded
    destination is then a URL for `sibling_path`, whose stages (scheme,
    percent-decoding) run over the CHARACTERS this pass produced: `&#37;20`
    is `%20` here and a space there, in spec order."""
    out, i = [], 0
    while i < len(s):
        if _is_escape(s, i):
            out.append(s[i + 1])
            i += 2
            continue
        m = _CHAR_REF.match(s, i)
        ch = _reference(m) if m else None
        if ch is not None:
            out.append(ch)
            i = m.end()
            continue
        out.append(s[i])
        i += 1
    return "".join(out)


def _is_escape(s, j):
    return s[j] == "\\" and j + 1 < len(s) and s[j + 1] in ASCII_PUNCT


def link_destination(s, i):
    """Link destination at `i` -> (destination, end) or (None, i).  §6.3:
    `<...>`: no line ending, no unescaped `<` or `>`.  Bare: nonempty, no ASCII
    control character (§2.1: U+0000-1F or U+007F) or space, does not start
    with `<`, parentheses only backslash-escaped or in balanced unescaped
    pairs.  Both forms return the text through `normalize_destination` (§2.4
    escapes and §2.5 character references, one pass) -- the ONE site, so the
    reference definition (`plan_memo_blocks.reference_definitions` calls
    this) decodes by the same rule.
    """
    if i < len(s) and s[i] == "<":
        j = i + 1
        while j < len(s):
            if _is_escape(s, j):
                j += 2
            elif s[j] in "<>\n":
                break
            else:
                j += 1
        if j < len(s) and s[j] == ">":
            return normalize_destination(s[i + 1:j]), j + 1
        return None, i
    j, depth = i, 0
    while j < len(s):
        c = s[j]
        if c == " " or ord(c) <= 31 or ord(c) == 127:
            break
        if _is_escape(s, j):
            j += 2
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            if depth == 0:
                break
            depth -= 1
        j += 1
    if j > i and depth == 0:
        return normalize_destination(s[i:j]), j
    return None, i


def link_title(s, i):
    """Link title at `i` -> end offset, or None if `s[i]` opens no valid title.
    `"…"` (no unescaped `"`), `'…'` (no unescaped `'`), `(…)` (no unescaped
    `(` or `)`)."""
    if i >= len(s) or s[i] not in "\"'(":
        return None
    opener = s[i]
    close = {"\"": "\"", "'": "'", "(": ")"}[opener]
    j = i + 1
    while j < len(s):
        if _is_escape(s, j):
            j += 2
        elif s[j] == close:
            return j + 1
        elif opener == "(" and s[j] == "(":
            return None
        else:
            j += 1
    return None


# §6.3 / §2.1: the characters label matching strips and collapses are spaces,
# tabs and line endings -- NOT Unicode whitespace (`str.split()` would fold a
# no-break space into a space and match two labels the spec keeps apart).
_LABEL_WS = re.compile(r"[ \t\r\n]+")


def normalize_label(label):
    """§6.3 label matching: "perform the Unicode case fold, strip leading and
    trailing spaces, tabs, and line endings, and collapse consecutive internal
    spaces, tabs, and line endings to a single space"."""
    return _LABEL_WS.sub(" ", label.strip(" \t\r\n")).casefold()


def _has_label_content(raw):
    """§6.3: "at least one character that is not a space, tab, or line ending"."""
    return bool(raw.strip(" \t\r\n"))


def link_label(s, i):
    """A link label opening at `s[i] == '['` -> (raw_label, end) or (None, i).
    §6.3: it "ends with the first right bracket (]) that is not
    backslash-escaped"; no unescaped `[` or `]` inside; at most 999
    characters between the brackets; at least one character that is not a
    space, tab, or line ending."""
    if i >= len(s) or s[i] != "[":
        return None, i
    j = i + 1
    while j < len(s):
        if _is_escape(s, j):
            j += 2
        elif s[j] in "[]":
            break
        else:
            j += 1
    if j >= len(s) or s[j] != "]":
        return None, i
    raw = s[i + 1:j]
    if len(raw) > 999 or not _has_label_content(raw):
        return None, i
    return raw, j + 1


def _is_image(s, i):
    """Whether the `[` at `s[i]` opens an image (§6.4): an unescaped `!`
    stands right before it.  Images are not links -- their destination never
    joins the population, their text is read as written -- and a link may
    wrap one (`[![alt](img.png)](sib.md)` links `sib.md`)."""
    return i > 0 and s[i - 1] == "!" and not _escaped(s, i - 1)


def _inline_tail(s, k):
    """After `](` at `k` -> (destination, end after `)`) or None.  §6.3 inline
    link: optional spaces/tabs/one line ending, an optional destination, then
    (separated the same way) an optional title, then `)`."""
    i = _skip_ws(s, k)
    dest, j = link_destination(s, i)
    if dest is None:
        dest, j = "", i
    k2 = _skip_ws(s, j)
    if k2 > j:
        t = link_title(s, k2)
        if t is not None:
            k2 = _skip_ws(s, t)
    if k2 < len(s) and s[k2] == ")":
        return dest, k2 + 1
    return None


def _reference_tail(s, opener, close, defs):
    """The reference forms at the `]` of `s[close]`, whose `[` is `s[opener]`
    (§6.3 precedence after the inline form): full `[text][label]`, collapsed
    `[text][]`, shortcut `[text]` -> (end, dest, form, label).  `dest` is
    None when no definition answers (the memo reports it as unresolved);
    `form` is None when the text is not a label at all (literal brackets,
    nothing to report).  ONE label grammar: the text of a collapsed /
    shortcut reference is a label iff `link_label` reads `[text]` from the
    opener (it stops at the first unescaped `[` or `]`, and the stack pairs
    `close` with the opener, so when it reads a label it closes at `close`)."""
    nxt = close + 1
    if nxt < len(s) and s[nxt] == "[":
        if nxt + 1 < len(s) and s[nxt + 1] == "]":
            form, end = "collapsed", nxt + 2
        else:
            raw, end = link_label(s, nxt)
            if raw is not None:
                # a link label follows, so `[text]` is not a shortcut either
                return end, defs.get(normalize_label(raw)), "full", raw
            form, end = "shortcut", close + 1
    else:
        form, end = "shortcut", close + 1
    raw, _ = link_label(s, opener)
    if raw is None:
        return end, None, None, None
    return end, defs.get(normalize_label(raw)), form, raw


# --------------------------------------------------------------------------
# CommonMark §6.5 autolinks -- "Autolinks are absolute URIs and email
# addresses inside < and >.  They are parsed as links, with the URL or email
# address as the link label."  The disposition is the plan's §3.1: MASKED, the
# whole span -- an autolink is ONE token whose contents are not inline syntax
# (`<https://example.com/[child](absent.md)>` is one autolink, commonmark.js
# 0.31.2 measured: `<a href="https://example.com/%5Bchild%5D(absent.md)">`),
# and its text IS its destination, so an id inside it is no more a naming site
# than an id in a link's destination is.  Tried at a `<` BEFORE the §6.6 tag
# grammar, the spec's order; the two are disjoint by construction (an autolink
# needs a `:` or an `@` where a tag name may hold neither, and a tag's `<!` /
# `</` / `<?` opener is not an ASCII letter), so the order is stated, not
# load-bearing -- the §6.5 and §6.6 example lists are the falsifier.
# --------------------------------------------------------------------------

# "A URI autolink consists of <, followed by an absolute URI followed by >" --
# "An absolute URI, for these purposes, consists of a scheme followed by a
# colon (:) followed by zero or more characters other than ASCII control
# characters, space, <, and >" (so U+0000-1F, U+0020 and U+007F end it:
# `< https://foo.bar >` is not an autolink, Example 608) -- "a scheme is any
# sequence of 2-32 characters beginning with an ASCII letter and followed by
# any combination of ASCII letters, digits, or the symbols plus ("+"), period
# ("."), or hyphen ("-")" (`<m:abc>` is a 1-character scheme and no autolink,
# Example 609; `<a+b+c:d>`, `<made-up-scheme://foo,bar>`, `<MAILTO:FOO@BAR.BAZ>`
# are).  "Backslash-escapes do not work inside autolinks" (Example 603,
# `<https://example.com/\[\>`), so the span is matched raw and never
# unescaped.
_URI_AUTOLINK = r"[A-Za-z][A-Za-z0-9+.-]{1,31}:[^\x00-\x20\x7f<>]*>"
# "An email autolink consists of <, followed by an email address, followed by
# >" -- "An email address, for these purposes, is anything that matches the
# non-normative regex from the HTML5 spec:
#   /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?
#    (?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/"
# -- transcribed verbatim, with the spec's `$` replaced by the closing `>`
# (`<foo.bar.baz>` has no `@` and is no autolink, Example 610; `<foo\+@bar.
# example.com>` is none either, Example 606, since the class holds no `\`).
_EMAIL_AUTOLINK = (r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?"
                   r"(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*>")
_AUTOLINK = re.compile("<(?:%s|%s)" % (_URI_AUTOLINK, _EMAIL_AUTOLINK))


# --------------------------------------------------------------------------
# CommonMark §6.6 raw HTML -- THE one tag grammar, spelled once.  "Text
# between < and > that looks like an HTML tag is parsed as a raw HTML tag and
# will be rendered in HTML without escaping."  Each production below is the
# spec's, verbatim in its docstring-comment; every arm was checked against
# commonmark.js 0.31.2 (`node cm.js`, PR #510 R17) at an INLINE position
# (`a <…>`: at a line start most of these shapes are §4.6 HTML BLOCKS, a
# Phase-1 matter).  §4.6 start condition 7 ("a complete open tag ... or a
# complete closing tag") reads `OPEN_TAG` / `CLOSING_TAG` from here, so the
# tag grammar has no second spelling in `plan_memo_blocks.py`.
# --------------------------------------------------------------------------

# "spaces, tabs, and up to one line ending" -- the separator BEFORE an
# attribute (at least one character: `<a href='bar'title=title>` is not a
# tag, Example 622 "Missing whitespace") ...
_WS = r"(?:[ \t]*\n[ \t]*|[ \t]+)"
# ... and its optional form, around an attribute's `=` and before the `/?>`
# (`<a b = "c">`, `<a b= c>`, `<b c="d" />` are tags; two line endings never
# meet the grammar, since a blank line ends the paragraph first).
_WS0 = r"(?:[ \t]*(?:\n[ \t]*)?)"
# "A tag name consists of an ASCII letter followed by zero or more ASCII
# letters, digits, or hyphens (-)": `<a-1>`, `<a1>` are tags; `<3 …>`,
# `<a:b>`, `<a_ b>`, `< span>` are not (Example 618 `<33> <__>`).
TAG_NAME = r"[A-Za-z][A-Za-z0-9-]*"
# "An attribute name consists of an ASCII letter, _, or :, followed by zero
# or more ASCII letters, digits, _, ., :, or -": `<a b:c=d>`, `<a b.c=d>`,
# `<a _boolean>` are tags; `<a 1b=d>`, `<a h*#ref="hi">` (Example 619) are not.
_ATTR_NAME = r"[A-Za-z_:][A-Za-z0-9_.:-]*"
# "An unquoted attribute value is a nonempty string of characters not
# including spaces, tabs, line endings, ", ', =, <, >, or `" -- so brackets
# and parentheses ARE value characters: `<span title=[x](y.md)>` is a tag
# (measured; ⚠ the R17 brief presumed otherwise) -- "A single-quoted
# attribute value consists of ', zero or more characters not including ',
# and a final '" -- "A double-quoted attribute value consists of ", zero or
# more characters not including ", and a final "" (`<a href="hi'>` is no
# tag, Example 620; `<a href="\"">` is no tag, Example 632: the `\` does not
# escape inside a tag, and `"` may not follow the value).
_ATTR_VALUE = r"(?:[^ \t\n\"'=<>`]+|'[^']*'|\"[^\"]*\")"
# "An attribute consists of spaces, tabs, and up to one line ending, an
# attribute name, and an optional attribute value specification" -- "An
# attribute value specification consists of optional spaces, tabs, and up
# to one line ending, a = character, optional spaces, tabs, and up to one
# line ending, and an attribute value."
_ATTRIBUTE = _WS + _ATTR_NAME + "(?:" + _WS0 + "=" + _WS0 + _ATTR_VALUE + ")?"
# "An open tag consists of a < character, a tag name, zero or more
# attributes, optional spaces, tabs, and up to one line ending, an optional
# / character, and a > character" -- the body after the `<`.  `<a / >`
# (Example 621) and `<a b=c<d>` are not tags; `<a b=c>d>` is the tag `<a
# b=c>` and the text `d>`.
OPEN_TAG = TAG_NAME + "(?:" + _ATTRIBUTE + ")*" + _WS0 + "/?>"
# "A closing tag consists of the string </, a tag name, optional spaces,
# tabs, and up to one line ending, and the character >" -- the body after
# the `<`.  `</a >` is one; `</ span>` and `</a b>` (Example 624) are not.
CLOSING_TAG = "/" + TAG_NAME + _WS0 + ">"
# "An HTML comment consists of <!-->, <!--->, or <!--, a string of
# characters not including the string -->, and -->" -- the 0.31 grammar
# (0.30 forbade `--` inside and a `-` at the end): `<!-- a -- b -->`, `<!--
# x --->`, `<!---->` are comments, `<!-->` / `<!--->` are the two short
# ones (Example 626: `foo <!--> foo -->` is the comment `<!-->` and text),
# `<!-- x --` is not closed.  Bodies after the `<`.
_COMMENT = r"!-->|!--->|!--.*?-->"
# "A processing instruction consists of the string <?, a string of
# characters not including the string ?>, and the string ?>" (Example 627;
# `<? ?>` is one).
_PI = r"\?.*?\?>"
# "A declaration consists of the string <!, an ASCII letter, zero or more
# characters not including the character >, and the character >" -- either
# case since 0.30 (`<!x>`, `<!ELEMENT br EMPTY>` Example 628); `<!>` and
# `<!ǅ x>` (not an ASCII letter) are not.
_DECLARATION = r"![A-Za-z][^>]*>"
# "A CDATA section consists of the string <![CDATA[, a string of characters
# not including the string ]]>, and the string ]]>" -- exact case, as §4.6
# condition 5 is (`<![cdata[ x ]]>` is text); Example 629.
_CDATA = r"!\[CDATA\[.*?\]\]>"
# "An HTML tag consists of an open tag, a closing tag, an HTML comment, a
# processing instruction, a declaration, or a CDATA section."  DOTALL: a
# comment, an instruction or a CDATA section may span the paragraph's line
# endings (Example 625, `foo <!-- this is a --\ncomment - with hyphens -->`).
_HTML_TAG = re.compile("<(?:%s|%s|%s|%s|%s|%s)" % (OPEN_TAG, CLOSING_TAG, _COMMENT, _PI, _DECLARATION, _CDATA),
                       re.DOTALL)


def _code_closer(runs, a1, k):
    """The end offset of the first backtick string of length `k` starting at
    or after `a1` (§6.1: a code span "ends with a backtick string of equal
    length"), or None.  `runs` = every backtick string of `s`, in order."""
    j = bisect.bisect_left(runs, (a1, 0))
    while j < len(runs):
        ra, rb = runs[j]
        if rb - ra == k:
            return rb
        j += 1
    return None


def inline_pass(s, defs):
    """ONE left-to-right pass over a block's inline content -- CommonMark
    0.31.2 "Appendix: A parsing strategy", Phase 2 "inline structure" --
    recognising backtick strings (§6.1), autolinks (§6.5), raw HTML (§6.6)
    and brackets (§6.3 / §6.4) together, and resolving references through
    `defs` (normalised label -> destination).  Returns (code, links, images,
    unresolved, html, autolinks).

    Autolinks (§6.5, PR #510 R21): at a `<` the autolink grammar is tried
    first (the spec's order; `_AUTOLINK`), and a match is ONE token the scan
    jumps past -- its contents are not inline syntax, so
    `<https://example.com/[child](absent.md)>` is one autolink and `absent.md`
    is no memo link (commonmark.js 0.31.2 measured; until R21 the brackets
    were scanned and the missing sibling was a false rc-2 miss).  Backslash
    escapes do not work inside one, so the span is matched raw.

    Backtick strings: a run opens a code span closed by the next run of
    equal length; the scan jumps past the span (brackets inside it are never
    delimiters: `` `[a](x.md)` `` is code); an unmatched run is literal and
    the scan resumes after it.  Inside a span backslashes are literal (§6.1:
    "backslash escapes do not work in code spans"), so a closer is read raw.
    An escaped backtick (`\\` + `` ` ``, §2.4) is a literal character and
    opens nothing.

    Raw HTML (§6.6, PR #510 R17): at a `<` the ONE tag grammar `_HTML_TAG`
    is tried; a match is a span the scan jumps past -- its text is never
    inline-parsed, so a bracket inside an attribute value or a comment is
    not a link delimiter (`<span title="[x](y.md)">` links nothing and a
    `]` inside it closes nothing; the tail `[x](absent.md)` there once made
    a false unavailable-memo miss, rc 2) and its content is not prose (an
    id inside an attribute is no naming site; the memo records the span
    for the LEX-UNSUPPORTED? seed, the disposition the plan's §3.0 gives
    every raw line); a `<` the grammar refuses is literal text and the
    brackets after it are read (`<3 [x](y.md)` and `<a href="x"
    [x](y.md)>` link `y.md`; `\\<span …>` is an escaped `<`).  The three
    delimiters are read left to right as met, commonmark.js's order:
    `<a href="`">b` c` is a tag and then a literal backtick, `` `x <span
    title="`">b `` a code span and then text; `[<span>](y.md)` is a link
    wrapping a tag (each measured).

    Brackets, per the Appendix's "look for link or image": a stack of `[` /
    `![` openers, each "active"; on `]` the nearest opener is popped -- "if
    we do find one, but it's not active, we remove the inactive delimiter
    from the stack, and return a literal text node ]"; if active, "we parse
    ahead to see if we have an inline link/image, reference link/image,
    collapsed reference link/image, or shortcut reference link/image" -- the
    inline tail is parsed by LOOKAHEAD ON THE RAW TEXT and the scan jumps
    past it, so a backtick inside a destination (`[sib](slice`x`.md)`) is
    consumed by the link, while a backtick BEFORE the `]` (`[not a
    `link](/foo`)`) opens a span that swallows the `]` and no link forms.
    "If we don't, then we remove the opening delimiter from the delimiter
    stack and return a literal text node ]"; if we do, the link or image is
    emitted and "if we have a link (and not an image), we also set all [
    delimiters before the opening delimiter to inactive.  (This will prevent
    us from getting links within links.)"

    A RESOLVED image's description is plain text (§6.4: "the image
    description" is rendered as the `alt` attribute's "plain string
    content"), so in ONE rule at the point the image closes every bracket
    construct recorded inside its description -- the entries whose offset
    lies past the image's `[`; they are the trailing ones, since brackets
    nest and each list is appended in closing order -- is the description's:
    a link there is DEMOTED to a masked tail in `images` (not a link -- its
    destination never joins the population -- and not prose either: the
    alt text is the link's TEXT, `![alt [docs](x.md)](i.png)` renders `<img
    alt="alt docs">`), a nested image stays masked, a failed reference there
    names no lost memo (resolved, it would have been demoted).  An
    UNRESOLVED image is the literal text `![…]` and the link inside it IS a
    link: `![alt [docs](x.md)][missing]` renders `![alt <a href="x.md">docs
    </a>][missing]` (commonmark.js 0.31.2, each shape measured; PR #510 R19
    -- until then the inner link joined the population as it closed, and
    `absent.md` inside a resolved image's description was a false rc-2
    miss).  The mirror case needs no rule: a link inside a LINK deactivates
    the outer opener as it closes (above), so `[a [b](x.md)](y.md)` links
    `x.md` and leaves `](y.md)` literal, as commonmark.js does.

    Linear in the bracket structure: no substring is re-parsed.  `code` =
    [(start, end)] backticks included; `html` = [(start, end)] of every raw
    HTML span, `<` and `>` included; `links` = [(tail_start, end,
    destination)] with `tail_start` the `]` closing the link text, so a
    caller masking the tail leaves the visible text -- prose -- in the
    scanned stream; `images` = [(tail_start, end)], every tail that is NOT a
    link's: a resolved image's (§6.4: its destination never joins the
    population, its alt text is prose, its tail is masked) and a demoted
    link's inside one; `unresolved` = [(offset, label, form, is_image)], every
    reference whose label `defs` does not define, with its FORM (`"full"` /
    `"collapsed"` / `"shortcut"`) decided by this one escape-honouring parse
    -- a caller never re-walks the raw text -- and whether the opener was an
    image (literal image syntax under §6.4, never a memo the author meant to
    link).  Such a LINK site is prose under §6.3, and a population the
    author meant to link is silently lost unless the caller reports it; the
    memo exempts a shortcut (every `[C19]` citation is one) unless a
    definition of its label exists somewhere the grammar cannot read it.

    Each failed reference is recorded ONCE.  After a failed FULL reference
    `[text][label]` (an image's too) the scan resumes after the literal
    `]`, so `[label]` is re-scanned -- it must be: §6.3 Example 571,
    `[foo][bar][baz]` with only `baz` defined, links `[bar][baz]`, and
    commonmark.js renders `![alt][missing][baz]` as `![alt]` plus that
    link.  When that re-scan closes as a SHORTCUT it fails for the very
    reason the full form did (same label, same `defs`) and is the same
    site, not a second one: it is not recorded, so `![alt][missing]` never
    leaves a bare `[missing]` behind for the orphan rule to read (§6.4: an
    undefined image reference is literal text, never a memo the author
    meant to link).
    """
    runs = [(m.start(), m.end()) for m in _BACKTICKS.finditer(s)]
    code, out, images, unresolved, html, auto = [], [], [], [], [], []
    stack, i, n = [], 0, len(s)
    relabel = -1        # the `[` of the label of the last failed full reference
    while i < n:
        c = s[i]
        if _is_escape(s, i):
            i += 2                      # §2.4: `\[` / `\]` / `\`` are literal
            continue
        if c == "<":
            m = _AUTOLINK.match(s, i)   # §6.5 before §6.6, the spec's order
            if m is not None:
                auto.append((i, m.end()))
                i = m.end()             # an autolink is one token, never inline-parsed
                continue
            m = _HTML_TAG.match(s, i)
            if m is None:
                i += 1                  # neither §6.5 nor §6.6, so a literal `<`
            else:
                html.append((i, m.end()))
                i = m.end()             # a raw HTML span is never inline-parsed
            continue
        if c == "`":
            a1 = i
            while a1 < n and s[a1] == "`":
                a1 += 1
            close = _code_closer(runs, a1, a1 - i)
            if close is None:
                i = a1                  # an unmatched backtick string is literal
            else:
                code.append((i, close))
                i = close
            continue
        if c == "[":
            stack.append([i, _is_image(s, i), True])
            i += 1
            continue
        if c != "]" or not stack:
            i += 1
            continue
        pos, is_img, active = stack.pop()
        if not active:
            i += 1                      # literal `]`; the opener is gone
            continue
        dest, end, form = None, None, None
        if i + 1 < n and s[i + 1] == "(":
            r = _inline_tail(s, i + 2)
            if r is not None:
                dest, end = r
        if end is None:
            end, dest, form, label = _reference_tail(s, pos, i, defs)
            if dest is None:
                if form is not None and not (form == "shortcut" and pos == relabel):
                    unresolved.append((pos, label, form, is_img))
                if form == "full":
                    relabel = i + 1     # `[label]` is re-scanned next (Example 571), not re-recorded
                i += 1                  # literal `]`; the opener is gone; the tail is NOT consumed
                continue
        if is_img:
            # §6.4: the description of a RESOLVED image is plain text, so
            # every bracket construct recorded inside it (offsets past this
            # image's `[`, the trailing entries) is the description's -- ONE
            # rule, here, where the image closes: a link is demoted to a
            # masked tail (never a memo link, never prose), a nested image
            # is demoted with it (it renders no `<img>` of its own -- its
            # alt text is folded into this one's), a failed reference names
            # no lost memo
            for j in range(len(images) - 1, -1, -1):
                if images[j][0] <= pos:
                    break
                images[j] = images[j][:2] + ("demoted",)
            while out and out[-1][0] > pos:
                images.append(out.pop()[:2] + ("demoted",))
            while unresolved and unresolved[-1][0] > pos:
                unresolved.pop()
            images.append((i, end, "image"))
        else:
            out.append((i, end, dest))
            for opener in stack:        # links may not contain links
                if not opener[1]:
                    opener[2] = False
        i = end
    return code, out, images, unresolved, html, auto



# --------------------------------------------------------------------------
# Bare tokens (no CommonMark construct, but a tokenisation fact of these
# documents): a `[C19]`-style citation id and a bare `.md` file name are read
# as one token, never as a run of row ids.  Found over the raw text, so a
# file name inside a code span is a token too.
# --------------------------------------------------------------------------

# WHAT A FILE NAME IS, decided here, once: a name is anything that ENDS IN
# `FILE_SUFFIX` -- the stem is unconstrained, so the suffix alone (`.md`) is
# a file name.  `plan_memo_memo.Memo.sibling_path` stage (d) CONSUMES this
# constant for the same test on a link destination (a link to `.md` names
# the sibling file `.md`); the lexer defines it because the lexer sits below
# the memo and reads it first.  Until PR #510 R20 the token arm required a
# stem of one character or more while `sibling_path` accepted the bare
# suffix, so beside a declared id `md` the prose `Read .md for details`
# reported `md` as a naming site.
FILE_SUFFIX = ".md"

# A bare `.md` file name is read by PATH SYNTAX, not a character class: the
# maximal run (possibly EMPTY -- the rule above) of non-whitespace characters
# ending in `FILE_SUFFIX`, bounded by spaces / tabs / line ends or the cell
# edge (`9z+notes.md`, `9z@notes.md`, `計画.md`, `.md` are file names --
# what `sibling_path` accepts), with the inline delimiters `[` `]` `<` `>`
# `` ` `` `|` excluded so a link's visible text (`[Slice 9z](slice-9z-sib.md)`)
# and a code span are not swallowed, and parentheses admitted only as a
# balanced pair (`(9z).md`).  Trailing closing punctuation (`)` `,` `.` `;`)
# needs no autolink-style stripping rule: the token ENDS at the suffix, so
# anything after it is outside by construction (the GFM §6.9
# extended-autolink trailing-punctuation rule is moot here, and is why none
# is picked).  The end boundary is the grammar's ASCII class (`ALNUM`), so
# `x.mdの` still ends the token; the citation shape is the grammar's
# `CITE_ID` (either case -- `[c1]` is `[C1]` under §6.3 label matching, and
# the unresolved-reference walk exempts it by the same predicate).
_TOKEN = re.compile(r"(?P<cite>%s)|(?P<file>(?:[^\s\[\]()<>`|]|\([^\s()]*\))*%s(?!%s))"
                    % (CITE_ID, re.escape(FILE_SUFFIX), ALNUM))


class Lexed:
    """The lexical facts of one block's INLINE content (a paragraph or a
    cell) -- Phase 2 of "Appendix: A parsing strategy"; block structure
    (fences, reference definitions, tables, paragraphs) is Phase 1, decided
    over raw lines by `plan_memo_memo.py::Memo`, and a reference
    definition is never inline content.  `tokens` = [(start, end, "cite" |
    "file")] over the raw text.  `resolve(defs)` runs `inline_pass` and sets
    `code` = code spans, `html` = raw HTML spans (§6.6), `autolinks` =
    [(start, end)] of every §6.5 autolink span (`<` and `>` included; masked
    whole, its destination never joining the population -- an autolink's URL
    carries a scheme or is a `mailto:`, so it is never a sibling on disk),
    `links` =
    [(tail_start, end, destination)], `images` = [(tail_start, end, kind)]
    (every tail that is not a link's, `kind` = "image" for a resolved
    image's own tail and "demoted" for a link or a nested image demoted
    inside a resolved image's description, §6.4) and
    `unresolved` = [(offset, label, form, is_image)] of the references no
    definition answers; `mask` is set by the disposition step in
    `plan_memo_tables.py` once the row ids are known."""

    __slots__ = ("text", "code", "html", "autolinks", "tokens", "links", "images", "unresolved", "mask")

    def __init__(self, text):
        self.text = text
        self.tokens = [(m.start(), m.end(), m.lastgroup) for m in _TOKEN.finditer(text)]
        self.code, self.html, self.autolinks = [], [], []
        self.links, self.images, self.unresolved = [], [], []
        self.mask = None

    def resolve(self, defs):
        """The inline pass over the RAW text (code spans, autolinks, raw HTML
        and brackets together; no pre-mask), with `defs` = normalised label ->
        destination."""
        (self.code, self.links, self.images, self.unresolved,
         self.html, self.autolinks) = inline_pass(self.text, defs)
