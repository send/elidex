#!/usr/bin/env python3
"""CommonMark 0.31.2 §6.3's LINK GRAMMAR, and the §2.4 / §2.5 character rules it
rests on -- carved out of `plan_memo_lexer.py` at touch time (PR #510 R42-7,
when that file reached 1,008 lines and `line_bound_control` said so).

THE SEAM IS A ONE-WAY EDGE, AND IT WAS MEASURED RATHER THAN CHOSEN: an AST pass
over the module's top-level statements reported that this group references
NOTHING outside itself, while the rest of the lexer references six names in it
(`_CHAR_REF`, `_inline_tail`, `_is_escape`, `_is_image`, `_reference`,
`_reference_tail`).  So `plan_memo_lexer` imports this module and this module
imports none of it -- no cycle, and nothing to keep in step.

WHAT IS HERE: the pieces a link is made of -- its destination (§6.3's balanced
parentheses and the WHATWG normalisation), its title, its label and that label's
normalisation, the `![` test, the inline and reference tails -- plus the §2.4
backslash escape and §2.5 character reference those rest on.  WHAT IS NOT: the
one-pass scanner that USES them (`inline_pass`), the §6.1 code-span closer index
and the bracket stack, which are the lexer's own.

⚠ `plan_memo_blocks.py` takes four of its six lexer imports from here now
(`link_destination` / `link_label` / `link_title` / `_escaped` / `_skip_ws`),
which is where a reference-definition reader should have been pointing all
along: it reads the link GRAMMAR, never the inline scanner.
"""

import re
import string
import urllib.parse
from html.entities import html5

ASCII_PUNCT = frozenset(string.punctuation)

def _escaped(s, i):
    """Whether `s[i]` sits behind an ODD run of backslashes (§2.4: the pairs
    before it escape each other, the odd one escapes `s[i]`)."""
    k = i
    while k > 0 and s[k - 1] == "\\":
        k -= 1
    return (i - k) % 2 == 1

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

# The nesting limit on a BARE destination's parentheses -- CommonMark 0.31.2
# §6.3, the parenthetical the destination grammar carries: "Implementations may
# impose limits on parentheses nesting to avoid performance issues, but at least
# three levels of nesting should be supported."  So the spec REQUIRES no limit,
# permits one, and the two reference implementations differ: commonmark.js
# 0.31.2 counts `openparens` without a bound, cmark 0.31.1 `manual_scan_link_url`
# has `if (++nb_p > 32) return -1`.  32 is cmark's number, taken here for the
# reason the spec's own parenthetical gives -- the SCAN, not conformance.
#
# ⚠ WHAT THIS IS NOT (PR #510 R26-1).  It is not a conformance fix, and the
# report that asked for it was wrong about the spec: 33 levels are not "literal
# Markdown", they are a destination every conforming implementation MAY read
# and commonmark.js DOES read.  The vendored corpus cannot decide it either --
# the deepest destination in all 630 examples is Example 496's
# `[link](foo(and(bar)))`, depth 2, measured -- so this is a choice made
# knowingly, in a region the spec leaves open.
#
# ⚠ BUT IT IS NOT A DIVERGENCE FROM THE ORACLE THAT GOVERNS THESE DOCUMENTS,
# and that is the reason to prefer 32 over any other number.  A plan memo is
# read on GitHub, which renders with cmark-gfm -- the same implementation this
# program already treats as the GFM oracle (`gh api -X POST /markdown -f
# mode=gfm`).  Measured through that oracle, not argued: a 32-deep destination
# comes back `<a href="a((…)).md">x</a>` and a 33-deep one comes back as the
# literal text `[x](a(((…))).md)`.  So above 32 the DOCUMENT does not hold a
# link where it is published, and reading one would be the checker inventing a
# memo its own reader never sees -- the same class as every finding this PR has
# closed.  commonmark.js is the oracle for the CommonMark core; where the two
# reference implementations are both conforming and disagree, the one that
# renders the artefact wins.  Re-run:
#   n=33; python3 -c "print('[x](a'+'('*$n+'z'+')'*$n+'.md)')" > /tmp/d.md
#   gh api -X POST /markdown -f mode=gfm -f text="$(cat /tmp/d.md)"
#
# WHY IT IS TAKEN ANYWAY: `inline_pass` states a linearity contract, which
# commonmark.js does not, and this scan is what breaks it.  A `]` followed by
# `(` runs the destination scan and, on failure, the pass advances ONE
# character, so `[`xN + `](`xN made every `]` scan nearly the whole remaining
# suffix.  Every such shape needs the depth to keep RISING -- a `)` that would
# take the run below its start ends the scan, and any group with a net close
# resolves the link instead of failing it -- so bounding the depth bounds the
# scan.  Measured over six adversarial shapes (`[`xN + `](`xN, that with a
# trailing `)`, with `\)` groups, with `](a` groups, fully nested, and with a
# long tail): all six cost 3.95x-3.97x per doubling uncapped and 2.01x-2.06x
# capped.
DESTINATION_NESTING_LIMIT = 32

def link_destination(s, i):
    """Link destination at `i` -> (destination, end) or (None, i).  §6.3:
    `<...>`: no line ending, no unescaped `<` or `>`.  Bare: nonempty, no ASCII
    control character (§2.1: U+0000-1F or U+007F) or space, does not start
    with `<`, parentheses only backslash-escaped or in balanced unescaped
    pairs, nested no deeper than `DESTINATION_NESTING_LIMIT` (the comment
    above: the spec permits the limit, cmark takes it, and here it is what
    bounds the scan).  Both forms return the text through
    `normalize_destination` (§2.4 escapes and §2.5 character references, one
    pass) -- the ONE site, so the reference definition
    (`plan_memo_blocks.reference_definitions` calls this) decodes by the same
    rule.
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
            if depth > DESTINATION_NESTING_LIMIT:
                return None, i          # §6.3's permitted limit; the scan's bound
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
