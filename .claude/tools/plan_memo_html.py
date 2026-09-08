#!/usr/bin/env python3
"""The two CommonMark grammars that open at a `<` -- §6.5 autolinks and §6.6
raw HTML -- for `plan-memo-umbrella-check.py`.

ONE SUBJECT, and it is not "part of Phase 2": these grammars are read by BOTH
phases, which is why they are their own module rather than the lexer's private
matter.  Phase 1 reads the open / closing tag bodies (`OPEN_TAG` /
`CLOSING_TAG`) as §4.6 HTML-block start condition 7
(`plan_memo_blocks.py`); Phase 2 tries `_AUTOLINK` and then `_HTML_TAG` at
every `<` of a block's inline content (`plan_memo_lexer.inline_pass`).  Until
PR #510 R26 they lived inside the lexer and Phase 1 imported them back out of
it, so a Phase-1 fact was spelled in a Phase-2 module.

Pure grammar: every name here is a pattern or a fragment of one, this module
imports nothing of this checker, and nothing here decides what a span MEANS --
the disposition of a matched span (masked whole, never inline-parsed, never
prose) is stated where it is applied, in `inline_pass`.

The scanning ORDER (§6.5 before §6.6) is the lexer's, not this module's: it is
a fact about what a `<` may open, and the two grammars are disjoint by
construction, so the order is stated there beside the code that depends on it.
"""

import re


# --------------------------------------------------------------------------
# CommonMark §6.5 autolinks -- "Autolinks are absolute URIs and email
# addresses inside < and >.  They are parsed as links, with the URL or email
# address as the link label."  The disposition is the plan's §3.0b: MASKED, the
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
