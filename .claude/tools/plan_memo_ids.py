#!/usr/bin/env python3
"""The id-token grammar of `plan-memo-umbrella-check.py`, spelled ONCE.

An id is a short alphanumeric token (`9z`, `10a`, `C`), a `#11-` slug, or a
`[C19]`-style citation id -- nothing else -- and may be decorated with bold,
backticks, or both, in either order.  This module answers "where do the
id-shaped tokens of this text start and end, and of which kind" for EVERY
reader: the bare and the row-noun-anchored naming scans, the raw-line seed,
the id cell, the kept-slug exception inside a code span, the lexer's citation
mask, and the citation exemption of the unresolved-reference walk.  It sits
below the lexer (which needs the citation shape) and imports nothing of the
checker's, because the grammar is a fact of these documents, not of
CommonMark.

THE BOUNDARY, decided here and nowhere else.  A token is read at every
position by `_TOKEN` (decoration, the longest kind first, decoration) and
kept iff each side is BOUNDED:
  * a side that carries decoration (`**9z**`, `` `9a` ``) is bounded by the
    decoration itself: the mark closes the token, so `` `9a`-`9d` `` is two
    ids and `**9z**7z` names `9z`;
  * an undecorated side is bounded by the COMPLEMENT of the kind's
    continuation class -- any other character, or the text edge -- never by
    a list of punctuation marks (a list once left `9z?` / `9z!` / `"9z"` /
    `“9z”` unreported while the report claimed "everything else is
    reported"):
      - a SHORT id is continued by an ASCII alphanumeric (`ALNUM`), or by a
        `.` with an ASCII alphanumeric on its far side (a dotted number:
        `§6.2a` names no row `2a`, `9z.次の` bounds `9z` -- the far side is
        tested with the same ASCII class, never `str.isalnum`).  A HYPHEN
        BOUNDS a short id on both sides: `after 1b-5` names `1b`,
        `0b-family` names `0b`, `9z-owner` / `owner-9z` name `9z`;
      - a SLUG is continued by an ASCII alphanumeric, `_` or `-` on BOTH
        sides (`次は#11-zz-alpha` is a site; `x#11-zz-alpha`, `#11-zz-alphaZZ`
        and `#11-zz-alpha_extra` are not the id `#11-zz-alpha` -- the slug's
        own class keeps its internal hyphens, so a hyphen after a slug is
        inside it, never after it);
      - a CITATION id is delimited by its own brackets.
  A bare `.md` file name (`slice-9z-sib.md`) is the lexer's `file` token,
  masked before any scan reads the text, so it never reaches this boundary.
Every class here is ASCII by spelling (`\\b` / `\\w` / `\\d` are Unicode in
a str pattern): `次のSlice C` has no Unicode word boundary before `Slice`,
and both naming sites went unreported under `\\b` (PR #510 R9).

The self-test's spelling sweep (`plan_memo_selftest_properties.py::
id_spelling_sweep_control`) reads every other module's string constants
for a second spelling of these classes; a reader that needs an ASCII
alphanumeric boundary composes `ALNUM` rather than writing it.
"""

import re

ALNUM_CHARS = "0-9A-Za-z"
ALNUM = "[%s]" % ALNUM_CHARS
"""The ASCII alphanumeric class: what continues a short id, and the ASCII
word boundary every anchor in these modules reads (the row-noun anchor, the
end of a bare `.md` file name)."""

BEFORE = "(?<!%s)" % ALNUM
AFTER = "(?!%s)" % ALNUM
"""The ASCII word boundary, as the two lookarounds that spell it -- the ONE
composition every reader outside this module uses, so no other module writes
the class (`plan_memo_selftest_properties.id_spelling_sweep_control` sweeps for
exactly that).  `\\b` is NOT it: it is Unicode in a str pattern (`次のSlice C`
has no boundary before `Slice`), and under `re.ASCII` it still counts `_` as a
word character where this grammar does not."""


def bounded(phrase):
    """`phrase`, bounded by `BEFORE` / `AFTER` on BOTH sides: an ASCII
    alphanumeric may not abut it on either end.

    The marker phrases the census reads (`plan_memo_tables.MARKER_RE`,
    `UNDETERMINED`, `POINTER`, `plan_memo_roles.DECLARES`,
    `LICENSE_BEFORE` / `LICENSE_AFTER`) all compose this rather than each
    growing its own edge: a phrase matcher that is a bare substring test or
    an unanchored regex matches INSIDE a longer word, and the class of
    "next unbounded phrase" is closed only by having one spelling of the
    boundary to compose.  Measured at PR #510 R22: `KIND UNDETERMINEDNESS`
    and `MANKIND UNDETERMINED` both classified a row kind-undetermined and
    forced exit 1, and `SUBUMBRELLA, not a terminal unit` /
    `UMBRELLA, not a terminal unitary claim` both read as the kind marker.
    A hyphen is not alphanumeric and so does not abut -- the same boundary
    a short id takes (`9z-owner` names `9z`), spelled once."""
    return "%s(?:%s)%s" % (BEFORE, phrase, AFTER)


SHORT_ID = ALNUM + "{1,4}"
SLUG_ID = r"#11-[a-z0-9-]+"
CITE_LABEL = r"[A-Za-z][0-9]+"
"""A citation label, either case: `[C19]` in the citation table, and `[c19]`
in prose is the SAME label under CommonMark §6.3 case-fold matching, so the
lexer's mask and the unresolved-reference exemption read one predicate
(`is_cite_label`) -- an uppercase-only mask once left `[c1]` visible to the
bare scan as a naming site of a declared short id `c1` (PR #510 R14)."""
CITE_ID = r"\[" + CITE_LABEL + r"\]"
DECOR_MARKS = ("**", "`")
"""How this document DECORATES an id, spelled once: bold, backticks, or
both, in either order.  `DECOR_CHARS` is the same fact as a character set,
which the stream builder reads to keep itself honest -- an escaped `\\*` or a
`&#42;` renders an asterisk that is TEXT, and substituting it would spell a
decoration the document does not have, so those two substitutions are
BLANKED instead of substituted (`plan_memo_tables.stream`).  Blanked, not
left standing as written: the source spelling holds letters and digits, and
a `&ast;` standing in the stream named a row `ast` no reader can see (PR
#510 R23)."""
DECOR_CHARS = frozenset("".join(DECOR_MARKS))
DECOR = r"(?:%s)*" % "|".join(re.escape(m) for m in DECOR_MARKS)

KINDS = (("slug", SLUG_ID), ("cite", CITE_ID), ("short", SHORT_ID))
"""Longest alternative first: a `#11-` slug is atomic (its internal hyphens
are not separators) and `[C1]` is one token, never `C1` between brackets."""

ROW_KINDS = ("slug", "short")
"""The kinds a row WITH A DECLARING FIELD is keyed by -- a §5 slice (`9z`,
`2ab`, `C`) or a slot (`#11-vm-foo`) -- and so the kinds that can stand in
a ROW position: after a row noun (`Slice #11-zz-alpha`), as the marker's
appositive subject (`Slice `#11-zz-alpha` — **UMBRELLA, …**`), as an owner
(`owned by **9z** and `#11-zz-alpha``).  A citation id (`[C19]`) keys a row
of the citation table -- declared, in the keep-set, so a code span may
spell it -- but declares no kind, is never named `Slice [C1]`, never owns
and never carries the marker: it is NOT a row in this sense and is not in
this alternation.  Every composer that reads "a row id here" composes
`ROW_ID`; a composer built on `SHORT_ID` alone read the slug form of the
same position as prose -- the marker attributed to a slug row made that
row an umbrella, no attribution finding, a wrong census, exit 0 (PR #510
R20; the R14 spelling sweep reads spellings, not kind coverage -- the
self-test's `row_kind_coverage_control` is the kind half)."""

ROW_ID = "(?:%s)" % "|".join(dict(KINDS)[k] for k in ROW_KINDS)
"""The ONE "a row id" alternation, in `KINDS` order (slug before short: the
longest kind first, as `_TOKEN` reads it)."""


def decorated_id(core, tag=""):
    """The ONE decorated-id spelling: groups `<tag>l` / `<tag>id` / `<tag>r`."""
    return r"(?P<%sl>%s)(?P<%sid>%s)(?P<%sr>%s)" % (tag, DECOR, tag, core, tag, DECOR)


_DECOR_TOKENS = re.compile("|".join(re.escape(m) for m in DECOR_MARKS))


def balanced(m, tag=""):
    """The decoration around `<tag>id` closes, in reverse order, exactly what
    it opened -- `**x**`, `` `x` ``, `` **`x`** `` -- and is not empty."""
    left = _DECOR_TOKENS.findall(m.group(tag + "l"))
    return bool(left) and _DECOR_TOKENS.findall(m.group(tag + "r")) == left[::-1]


_TOKEN = re.compile(decorated_id("|".join("(?P<%s>%s)" % kv for kv in KINDS)))
_KIND = {kind: re.compile(core) for kind, core in KINDS}

# The continuation class per kind (the docstring's rule); a citation id is
# delimited by its own brackets and has none.
_CONTINUES = {
    "short": re.compile(ALNUM),
    "slug": re.compile("[%s_-]" % ALNUM_CHARS),
    "cite": None,
}


class Token:
    """One bounded id token: its `kind` ("short" / "slug" / "cite"), the id
    text, the extent of the whole decorated token (`start` / `end`) and of
    the id itself (`idstart` / `idend`), and its decoration (`l` / `r`)."""

    __slots__ = ("kind", "id", "start", "end", "idstart", "idend", "l", "r")

    def __init__(self, m):
        self.kind = next(k for k, _ in KINDS if m.group(k) is not None)
        self.id, self.start, self.end = m.group("id"), m.start(), m.end()
        self.idstart, self.idend = m.start("id"), m.end("id")
        self.l, self.r = m.group("l"), m.group("r")

    @property
    def balanced(self):
        left = _DECOR_TOKENS.findall(self.l)
        return bool(left) and _DECOR_TOKENS.findall(self.r) == left[::-1]


def _glued(text, i, step, kind, lo, hi):
    """Whether `text[i]` (inside `[lo, hi)`) continues a token of `kind`
    ending just before it (`step` = +1) or starting just after it (`step` =
    -1) -- the docstring's rule."""
    cont = _CONTINUES[kind]
    if cont is None or not (lo <= i < hi):
        return False
    if cont.match(text[i]):
        return True
    j = i + step
    # the dotted-number rule is the SHORT kind's; the far side is tested
    # with the same class
    return kind == "short" and text[i] == "." and lo <= j < hi and bool(cont.match(text[j]))


def tokens(text, pos=0, endpos=None):
    """Every bounded id token of `text[pos:endpos]`, left to right, non-
    overlapping (a candidate is read at each position by the longest kind
    first; a glued candidate is dropped and the scan resumes after it, so
    `xxxxC` yields neither `xxxx` nor `C`).  The boundary is tested inside
    `[pos, endpos)` only: the delimiters of a code span bound what is read
    out of it."""
    hi = len(text) if endpos is None else endpos
    for m in _TOKEN.finditer(text, pos, hi):
        t = Token(m)
        if not t.l and _glued(text, t.start - 1, -1, t.kind, pos, hi):
            continue
        if not t.r and _glued(text, t.end, +1, t.kind, pos, hi):
            continue
        yield t


def kind_of(s):
    """The kind an undecorated id string is, or None when it is no id."""
    return next((k for k, pat in _KIND.items() if pat.fullmatch(s)), None)


_CITE_LABEL = re.compile(CITE_LABEL)


def is_cite_label(normalized_label):
    """Whether a NORMALISED (§6.3 case-folded) link label is a citation
    label -- the one predicate the lexer's mask and the unresolved-reference
    exemption share (`CITE_LABEL` admits either case, so the fold is moot)."""
    return _CITE_LABEL.fullmatch(normalized_label) is not None
