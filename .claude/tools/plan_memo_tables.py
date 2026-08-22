#!/usr/bin/env python3
"""Table parse for the VM-P4 umbrella plan memo -- rows, ids, kinds.

Split out of `plan-memo-umbrella-check.py` at the touch-time threshold: that file
had grown past 960 lines with a real cohesion seam down the middle.  Everything
here answers "what rows does this document have, what is each one's id, and what
kind does its declaring field declare?".  Everything left there answers "what
does the prose say about those rows, and is that allowed?".

⚠ This is also the residue the checker's own header names as worth collapsing
across the four in-flight plan-memo programs -- the GFM row splitter that honours
an escaped pipe.  It is a module here so that collapse is a MOVE rather than a
rewrite; the trigger for it stays what that header says.
"""

import pathlib
import re


MARKER = "UMBRELLA, not a terminal unit"

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Keying the population on the
# UMBRELLA marker alone therefore left every such row outside every scan: a
# population defined by the symptom's vocabulary rather than by the property the
# rule is about (`feedback_checks-must-not-be-defined-by-the-symptom-vocabulary`,
# cited in this file for ROLE_PATTERNS and violated one screen earlier).
#
# Two spellings are in use in the document.  This tolerates both AND reports the
# divergence, rather than silently blessing it -- a kind with two spellings is a
# kind no program can enumerate, which is the defect one level up.
UNDETERMINED = re.compile(r"KIND\s*[\u2014-]?\s*UNDETERMINED", re.IGNORECASE)

# --------------------------------------------------------------------------
# GFM row parsing.  Split on UNESCAPED pipes only: a cell containing `\|`
# is one cell, and a naive split reports a row with more fields than it has.
# --------------------------------------------------------------------------

_ESC = "\x01"


def split_row(line):
    return [c.replace(_ESC, r"\|") for c in line.replace(r"\|", _ESC).split("|")]


def is_row(line):
    # `|---|` is a row of the table too: scoping to `| ` drops the separator and
    # with it the loop that walks the body.
    return line.startswith("|")


def is_separator(cells):
    return all(re.fullmatch(r"[:\- ]*", c) for c in cells[1:-1]) and len(cells) > 2


# --------------------------------------------------------------------------
# Table schemas.  Each table is identified by its HEADER ROW, not by line
# ranges: a line range is a figure that a later edit silently invalidates,
# and this document has already lost three figures that way.
# --------------------------------------------------------------------------

SCHEMAS = [
    # name,          header cells (stripped, in order),                       declaring, id, mention-bearing columns
    ("citation", ["ID", "Citation", "Anchor", "Used by"], None, 1, [4]),
    ("stub", ["Site", "Syntax", "Emits", "Observable", "Tier", "Slice"], None, None, [6]),
    ("slice", ["#", "Slice", "Primary module(s)", "Slot", "Tier", "Deps"], 2, 1, [6]),
    ("slot", ["Slot", "Why deferred", "Trigger", "Re-eval"], 2, 1, [3]),
]


def find_tables(lines):
    """Return [(schema_name, header_lineno, [(lineno, cells), ...]), ...]."""
    out = []
    for i, line in enumerate(lines):
        if not is_row(line):
            continue
        cells = [c.strip() for c in split_row(line)]
        body = [c for c in cells[1:-1]]
        for name, hdr, decl, idc, mention in SCHEMAS:
            if body[: len(hdr)] == hdr and len(body) == len(hdr):
                rows = []
                j = i + 1
                if j < len(lines) and is_row(lines[j]) and is_separator(split_row(lines[j])):
                    j += 1
                while j < len(lines) and is_row(lines[j]):
                    rows.append((j + 1, split_row(lines[j])))
                    j += 1
                out.append((name, i + 1, rows))
                break
    return out


# --------------------------------------------------------------------------
# Row identity
# --------------------------------------------------------------------------

_BOLD = re.compile(r"^\*\*(.+?)\*\*$")
_TICK = re.compile(r"^`(.+?)`$")


def code_spans(s, keep=()):
    """Byte spans this scan must not read an id out of.

    Inline code (`Reflect.construct(C, [], D)`), link targets and bare file
    names (`2026-07-vm-p4-slice-1a-1b-call-spread-detail.md` contains `1a` and
    `1b`, and a file name is not a naming site).  A link's visible LABEL is
    prose and is scanned.  The self-test carries both as
    NEGATIVE controls; no figure is quoted here, because a count of what a mask
    removes is a property of the document on the day it was run, and this file
    has no way to re-derive it when the document changes under it.
    """
    out, i = [], 0
    while True:
        a = s.find("`", i)
        if a < 0:
            break
        b = s.find("`", a + 1)
        if b < 0:
            break
        # A backtick run whose whole content is a row id is the document
        # spelling an id, not code -- `A` in "naming `A` itself would name
        # nobody".  Masking those hid every one of row M's violations.
        inner = s[a + 1:b]
        # `10b → 10a` is two ids and an arrow: a Deps-shaped edge the document
        # wrote inside backticks, not code.  Masking it hid an edge between two
        # umbrella rows that neither triage generation ever saw.
        toks = [x for x in re.split(r"[\s,;/→>+&|-]+", inner) if x]
        if inner not in keep and not (toks and all(x in keep for x in toks)):
            out.append((a, b + 1))
        i = b + 1
    # ⚠ A link's visible LABEL is prose and must be scanned; only the
    # destination and a bare filename are not.  Masking `\[[^\]]*\]` removed the
    # label too, so `[Slice 9z lands first](detail.md)` -- an ordering claim a
    # reader sees -- was never reported.  The existing control only exercised an
    # id in the DESTINATION, so it could not catch this.
    #
    # `[C19]`-style citation ids stay masked: a bracketed token that is a
    # citation id, not a sentence.
    # The bare-filename alternative is a filename character class, not `\S+`:
    # `\S+\.md` started at the last label token of `[Slice 9z](detail.md)` and
    # masked `9z` together with the destination, so an id that ends a label was
    # never reported -- the control above kept `9z` safe only because extra
    # label words followed it.
    for m in re.finditer(r"\]\([^)]*\)|\[[A-Z][0-9]+\]|[\w./-]+\.md\b", s):
        out.append(m.span())
    return out


# An id is a short alphanumeric token or a `#11-` slug -- nothing else.  Without
# a grammar the whole cell was taken as the id, so a retired row printed as
# ``0a — MERGED `658cc302` `` and its real id `0a` never entered `all_row_ids`,
# which meant a backticked `` `0a` `` was masked as code instead of read as a
# mention.  Harmless only because `0a` is not an umbrella.
ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"
"""How this document names a row when it refers to one.

Lives here rather than with the licensing rule because it is a fact about row
IDENTITY -- `Memo._attributed_to_other` needs it to decide whose kind a marker
declares, and that dependency is what the touch-time split surfaced.
"""

_ID_GRAMMAR = re.compile(r"^(#11-[a-z0-9-]+|[0-9A-Za-z]{1,4})(?![0-9A-Za-z-])")


def bare_id(cell):
    """The row id as written, stripped of decoration and of trailing prose."""
    s = cell.strip()
    for pat in (_BOLD, _TICK, _BOLD):
        m = pat.match(s)
        if m:
            s = m.group(1).strip()
    g = _ID_GRAMMAR.match(s)
    return g.group(1) if g else s


class Memo:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text()
        self.lines = self.text.split("\n")
        self.tables = find_tables(self.lines)
        self.rows = {}          # table name -> [(lineno, cells)]
        for name, _, rows in self.tables:
            self.rows.setdefault(name, []).extend(rows)

    # -- carved siblings ---------------------------------------------------

    _LINK = re.compile(r"\]\(([^)\s]+\.md)\)")

    def linked_memos(self):
        """Every `.md` this memo links, resolved beside it, in first-link order.

        The naming population is the memo AND the files it carved prose into;
        the memo's own links are the only authoritative list of those.  Taking
        them as optional positional arguments made completeness depend on the
        caller remembering three file names: the documented main-only
        invocation scanned 611 sites and exited 0, the two-sibling one 658,
        the three-sibling one 706 -- the same exit code for three populations.
        """
        out = []
        for m in self._LINK.finditer(self.text):
            p = (self.path.parent / m.group(1)).resolve()
            if p != self.path.resolve() and p not in out:
                out.append(p)
        return out

    # -- row inventories ---------------------------------------------------

    def data_rows(self, table):
        out = []
        for lineno, cells in self.rows.get(table, []):
            if is_separator([c.strip() for c in cells]):
                continue
            out.append((lineno, cells))
        return out

    def umbrella_ids(self, attributed=None):
        """Umbrella ids, read from each table's DECLARING field.

        Not from a grep over the marker: the marker's literal is a substring of
        itself, so a whole-row or whole-file grep returns the marker count by
        construction and cannot disagree for any content.

        ⚠ Cell-scoping is not enough either.  A marker can sit in the declaring
        field and attribute the kind to a DIFFERENT row -- a §8 slot whose cell
        opens `Slice **E** -- **UMBRELLA, not a terminal unit**` is declaring
        that §5's row E is an umbrella, not that the slot is.  §5's own rule
        says a slot that is a pointer into §5 "carries no marker of its own", so
        such a row must not be in the count, and the earlier program put it
        there.  Markers attributed to another row are collected in `attributed`
        and excluded, so the figure is what §5's rule says it is.
        """
        ids = {}
        for name, hdr, decl, idc, _ in SCHEMAS:
            if decl is None or idc is None:
                continue
            for lineno, cells in self.data_rows(name):
                if len(cells) <= max(decl, idc):
                    continue
                if MARKER not in cells[decl]:
                    continue
                rid = bare_id(cells[idc])
                other = self._attributed_to_other(cells[decl], rid)
                if other:
                    if attributed is not None:
                        attributed.append((name, lineno, rid, other))
                    continue
                ids[rid] = (name, lineno)
        return ids

    @staticmethod
    def _attributed_to_other(field, rid):
        """The row id a marker names, when it is not this row's own."""
        for m in re.finditer(re.escape(MARKER), field):
            pre = field[max(0, m.start() - 70): m.start()]
            # The id must be the marker's APPOSITIVE subject -- `Slice **E** —
            # **UMBRELLA, …**` -- with nothing between them but dash punctuation
            # and emphasis.  A 70-char proximity window instead excluded a
            # genuine self-declaration that merely MENTIONED a sibling
            # ("Unlike Slice 7z, **UMBRELLA, not a terminal unit.**"), which
            # dropped that row from the census AND, silently, from the naming
            # population -- so every site naming it as an owner stopped being
            # reported.  The self-test carries both directions.
            g = re.search(
                ROW_NOUN + r"[\s-]+(?:\*\*|`)*([0-9A-Za-z]{1,4})(?:\*\*|`)*"
                r"\s*[\u2014\u2013-]\s*(?:\*\*|`)*\s*$", pre)
            if g and g.group(1) != rid:
                return g.group(1)
        return None

    def undetermined_ids(self, spellings=None):
        """Rows whose declaring field says their kind is not settled."""
        ids = {}
        for name, hdr, decl, idc, _ in SCHEMAS:
            if decl is None or idc is None:
                continue
            for lineno, cells in self.data_rows(name):
                if len(cells) <= max(decl, idc):
                    continue
                m = UNDETERMINED.search(cells[decl])
                if m:
                    ids[bare_id(cells[idc])] = (name, lineno)
                    if spellings is not None:
                        spellings.add(m.group(0))
        return ids

    def no_owner_ids(self):
        """Every row that carries no owner and no ordering.

        This -- not "carries the UMBRELLA marker" -- is the property §5's naming
        rule is stated over, so it is what the naming scan and the `Deps`
        assertion are both derived from.
        """
        out = dict(self.umbrella_ids())
        out.update(self.undetermined_ids())
        return out

    def all_row_ids(self):
        ids = {}
        for name, hdr, decl, idc, _ in SCHEMAS:
            if idc is None:
                continue
            for lineno, cells in self.data_rows(name):
                if len(cells) <= idc:
                    continue
                rid = bare_id(cells[idc])
                if rid and rid != "—":
                    ids.setdefault(rid, []).append((name, lineno))
        return ids


