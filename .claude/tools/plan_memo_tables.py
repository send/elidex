#!/usr/bin/env python3
"""Row inventory for `plan-memo-umbrella-check.py` -- tables, rows, ids, kinds,
and the one transitive `Population` every scan and assertion reads.

Everything here answers "what rows does this document set have, what is each
one's id, and what kind does its declaring field declare?".  The lexical
substrate (fences, rows, code spans, links) is `plan_memo_lexer.py`; what the
prose says about the rows, and whether that is allowed, is the checker and
`plan_memo_roles.py`.

Three rules decided here, once:
  * a table is admitted in `find_tables` and nowhere else: header and
    delimiter rows of equal width (GFM §4.10), and -- local policy over GFM's
    "may vary" clause -- a body row of a SCHEMA table whose width differs from
    the header is a schema miss (exit 2), never a silent skip or a shifted read;
  * row ids are read from the raw id cell (`bare_id` strips backticks itself),
    kind markers from the MASKED declaring field (a quoted marker declares
    nothing);
  * the population is transitive over the memos a memo links, and the same id
    declared twice is a schema miss.
"""

import pathlib
import re

from plan_memo_lexer import (
    blank_spans, code_spans, delimiter_width, fenced_lines, is_blank, links,
    normalize_label, one_line_block, reference_definitions, split_row, starts_block,
)

MARKER = "UMBRELLA, not a terminal unit"

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Two spellings are in use; both
# are tolerated and the divergence is reported (a kind with two spellings is a
# kind no program can enumerate).
UNDETERMINED = re.compile(r"KIND\s*[—-]?\s*UNDETERMINED", re.IGNORECASE)

# --------------------------------------------------------------------------
# Table schemas, identified by HEADER ROW (a line range is a figure a later
# edit silently invalidates).  Column indexes are BODY columns: the optional
# leading pipe is stripped at admission, so column 0 is the first cell.
#   name, header cells (trimmed, in order), declaring column, id column
# --------------------------------------------------------------------------

SCHEMAS = [
    ("citation", ["ID", "Citation", "Anchor", "Used by"], None, 0),
    ("stub", ["Site", "Syntax", "Emits", "Observable", "Tier", "Slice"], None, None),
    ("slice", ["#", "Slice", "Primary module(s)", "Slot", "Tier", "Deps"], 1, 0),
    ("slot", ["Slot", "Why deferred", "Trigger", "Re-eval"], 1, 0),
]


class Table:
    __slots__ = ("schema", "header_lineno", "header", "rows", "misses")

    def __init__(self, schema, header_lineno, header):
        self.schema = schema            # schema name or None
        self.header_lineno = header_lineno
        self.header = header            # [Cell]
        self.rows = []                  # [(lineno, [Cell])], body rows only
        self.misses = []                # [(lineno, message)] width policy


def find_tables(lines, fenced):
    """Admit every GFM table in `lines` (0-based; `fenced` = line indexes to
    skip).  Returns ([Table], {line index: Table}).  The table runs from the
    header to the first blank line or block start; every line in between is a
    body row, pipes or not (GFM §4.10).
    """
    tables, owner, i, n = [], {}, 0, len(lines)
    while i < n:
        if i in fenced or is_blank(lines[i]) or i + 1 >= n or starts_block(lines[i]):
            i += 1
            continue
        width = delimiter_width(lines[i + 1])
        if width is None:
            i += 1
            continue
        header = split_row(lines[i])
        if len(header) != width:
            i += 1
            continue
        hdr_text = [c.text for c in header]
        schema = next((name for name, hdr, _, _ in SCHEMAS if hdr_text == hdr), None)
        t = Table(schema, i + 1, header)
        owner[i], owner[i + 1] = t, t
        j = i + 2
        while j < n and j not in fenced and not is_blank(lines[j]) and not starts_block(lines[j]):
            body = split_row(lines[j])
            if schema is not None and len(body) != width:
                t.misses.append((j + 1, "row has %d cell(s); the %r header has %d -- "
                                 "a shifted read fabricates findings, so this row is "
                                 "unscanned" % (len(body), schema, width)))
            else:
                t.rows.append((j + 1, body))
            owner[j] = t
            j += 1
        tables.append(t)
        i = j
    return tables, owner


# --------------------------------------------------------------------------
# Row identity
# --------------------------------------------------------------------------

_BOLD = re.compile(r"^\*\*(.+?)\*\*$")
_TICK = re.compile(r"^`(.+?)`$")

# An id is a short alphanumeric token or a `#11-` slug -- nothing else.
ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares."""

_ID_GRAMMAR = re.compile(r"^(#11-[a-z0-9-]+|[0-9A-Za-z]{1,4})(?![0-9A-Za-z-])")


def bare_id(cell_text):
    """The row id as written, stripped of decoration and of trailing prose."""
    s = cell_text.strip()
    for pat in (_BOLD, _TICK, _BOLD):
        m = pat.match(s)
        if m:
            s = m.group(1).strip()
    g = _ID_GRAMMAR.match(s)
    return g.group(1) if g else s


def attributed_to_other(field, rid):
    """The row id a marker names, when it is not this row's own: the marker's
    APPOSITIVE subject -- `Slice **E** — **UMBRELLA, …**` -- with nothing
    between them but dash punctuation and emphasis.  A proximity window
    instead excluded a genuine self-declaration that merely MENTIONED a sibling
    ("Unlike Slice 7z, **UMBRELLA, not a terminal unit.**"); the self-test
    carries both directions."""
    for m in re.finditer(re.escape(MARKER), field):
        pre = field[max(0, m.start() - 70): m.start()]
        g = re.search(
            ROW_NOUN + r"[\s-]+(?:\*\*|`)*([0-9A-Za-z]{1,4})(?:\*\*|`)*"
            r"\s*[—–-]\s*(?:\*\*|`)*\s*$", pre)
        if g and g.group(1) != rid:
            return g.group(1)
    return None


def id_only(inner, keep):
    """The disposition exception: a code span whose content is only row ids
    (and separators) is the document SPELLING an id, and is a mention."""
    toks = [x for x in re.split(r"[\s,;/→>+&|-]+", inner) if x]
    return inner in keep or (bool(toks) and all(x in keep for x in toks))


def code_mask(s, keep):
    """Code spans of `s` minus the id-only ones -- the spans a reader of prose
    must skip."""
    return [(a, b) for a, b in code_spans(s) if not id_only(s[a:b].strip("`").strip(), keep)]


def block_links(s, defs):
    """Links of a block's inline content `s` (code spans masked first; a
    leading run of reference definitions is not a link, §4.7), as
    [(tail_start, end, destination)] in `s` coordinates."""
    masked = blank_spans(s, code_spans(s))
    start = reference_definitions(masked)[1]
    return [(a + start, b + start, dest) for a, b, dest in links(masked[start:], defs)]


def mask_spans(s, keep, defs):
    """Spans of `s` (a block's inline content) the scanners must not read an
    id out of: code spans (minus id-only ones), link reference definitions,
    link tails (the visible text stays: it is prose), `[C19]`-style citation
    ids and bare `.md` file names."""
    out = code_mask(s, keep)
    # a definition renders nothing, so none of it -- label included -- is prose
    defs_end = reference_definitions(blank_spans(s, code_spans(s)))[1]
    if defs_end:
        out.append((0, defs_end))
    out += [(a, b) for a, b, _ in block_links(s, defs)]
    for m in re.finditer(r"\[[A-Z][0-9]+\]|[\w./-]+\.md\b", s):
        out.append(m.span())
    return out


def masked_text(s, keep):
    """`s` with its code spans blanked, id-only spans excepted -- the stream
    the kind-marker reader reads (a quoted marker is not a declaration)."""
    return blank_spans(s, code_mask(s, keep))


class Paragraph:
    """Lines outside tables and fences, grouped at blank lines and block
    starts; `content` is the inline content code spans are lexed over."""

    __slots__ = ("lines", "content", "offsets")

    def __init__(self, numbered):
        self.lines = numbered                       # [(lineno, text)]
        self.content = "\n".join(t for _, t in numbered)
        self.offsets = []
        off = 0
        for _, t in numbered:
            self.offsets.append(off)
            off += len(t) + 1

    def per_line(self, spans):
        """Project content spans onto lines -> {lineno: [(s, e)]}."""
        out = {}
        for a, b in spans:
            for (lineno, text), off in zip(self.lines, self.offsets):
                s, e = max(a, off) - off, min(b, off + len(text)) - off
                if s < e:
                    out.setdefault(lineno, []).append((s, e))
        return out


class Memo:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text()
        self.lines = self.text.split("\n")
        self.fenced = fenced_lines(self.lines)
        self.tables, self.owner = find_tables(self.lines, self.fenced)
        self.paragraphs = self._paragraphs()
        # §4.7: the first definition of a label wins
        self.defs = {}
        for p in self.paragraphs:
            masked = blank_spans(p.content, code_spans(p.content))
            for raw, dest, _ in reference_definitions(masked)[0]:
                self.defs.setdefault(normalize_label(raw), dest)

    def _paragraphs(self):
        out, cur = [], []

        def flush():
            if cur:
                out.append(Paragraph(list(cur)))
                cur.clear()

        for i, line in enumerate(self.lines):
            if i in self.fenced or i in self.owner or is_blank(line):
                flush()
                continue
            if starts_block(line):
                flush()
            cur.append((i + 1, line))
            if one_line_block(line):
                flush()
        flush()
        return out

    def linked_files(self):
        """Every `.md` this memo links, resolved beside it, in first-link order."""
        out = []
        for p in self.paragraphs:
            for _, _, dest in block_links(p.content, self.defs):
                name = re.split(r"[#?]", dest, 1)[0]
                if not name.endswith(".md"):
                    continue
                f = (self.path.parent / name).resolve()
                if f != self.path.resolve() and f not in out:
                    out.append(f)
        return out

    def schema_rows(self, name):
        """[(lineno, [Cell])] body rows of every table matching schema `name`."""
        return [r for t in self.tables if t.schema == name for r in t.rows]


class Population:
    """The memo set reachable from one memo through its links (visited set, so
    a cycle is not an error), with ONE map `ids`: id -> (kind, memo, table,
    lineno).  Kinds: "umbrella" / "undetermined" / "terminal".  `misses` holds
    every schema miss (absent memo, unmatched schema, row width, duplicate
    declaration); a non-empty `misses` is exit 2 -- never a clean run.
    """

    def __init__(self, main_path):
        self.memos = []
        self.misses = []            # [(file, lineno, message)]
        self.spellings = set()
        self.attributed = []        # [(file, table, lineno, rid, other)]
        self.ids = {}
        queue, seen = [pathlib.Path(main_path).resolve()], set()
        while queue:
            p = queue.pop(0)
            if p in seen:
                continue
            seen.add(p)
            if not p.is_file():
                self.misses.append((p.name, 0, "linked memo not found -- its population is unscanned"))
                continue
            memo = Memo(p)
            self.memos.append(memo)
            queue.extend(memo.linked_files())
        self.main = self.memos[0] if self.memos else None
        if self.main is not None:
            matched = {t.schema for t in self.main.tables}
            for name, _, _, _ in SCHEMAS:
                if name not in matched:
                    self.misses.append((self.main.path.name, 0,
                                        "no table matched schema %r -- its whole population is unscanned" % name))
        for memo in self.memos:
            for t in memo.tables:
                for lineno, msg in t.misses:
                    self.misses.append((memo.path.name, lineno, msg))
        # ids first (the keep-set the marker reader needs), then kinds
        for memo in self.memos:
            self._declare(memo)
        keep = set(self.ids)
        for rid, (memo, name, lineno, decl_cell) in list(self.ids.items()):
            self.ids[rid] = (self._kind(memo, name, lineno, rid, decl_cell, keep), memo, name, lineno)

    # -- declarations ------------------------------------------------------

    def _declare(self, memo):
        for name, _, decl, idc in SCHEMAS:
            if idc is None:
                continue
            for lineno, cells in memo.schema_rows(name):
                rid = bare_id(cells[idc].text)
                if not rid or rid == "—":
                    continue
                if rid in self.ids:
                    m2, t2, l2, _ = self.ids[rid]
                    self.misses.append((memo.path.name, lineno,
                                        "row %r is declared twice (also %s:%d in %r); a population "
                                        "with two declarations of one id cannot be scanned"
                                        % (rid, m2.path.name, l2, t2)))
                    continue
                self.ids[rid] = (memo, name, lineno, cells[decl].text if decl is not None else None)

    def _kind(self, memo, name, lineno, rid, decl_cell, keep):
        if decl_cell is None:
            return "terminal"
        field = masked_text(decl_cell, keep)
        if MARKER in field:
            other = attributed_to_other(field, rid)
            if other:
                self.attributed.append((memo.path.name, name, lineno, rid, other))
            else:
                return "umbrella"
        m = UNDETERMINED.search(field)
        if m:
            self.spellings.add(m.group(0))
            return "undetermined"
        return "terminal"

    # -- inventories -------------------------------------------------------

    def ids_of_kind(self, kind):
        return {rid: v for rid, v in self.ids.items() if v[0] == kind}

    def umbrella_ids(self):
        return self.ids_of_kind("umbrella")

    def undetermined_ids(self):
        return self.ids_of_kind("undetermined")

    def no_owner_ids(self):
        """Every row that carries no owner and no ordering -- the property §5's
        naming rule is stated over (umbrella + kind-undetermined)."""
        return {rid: v for rid, v in self.ids.items() if v[0] != "terminal"}

    def data_rows(self, name):
        """[(memo, lineno, [Cell])] over every memo, for schema `name`."""
        return [(memo, lineno, cells) for memo in self.memos for lineno, cells in memo.schema_rows(name)]

    def keep(self):
        """The code-span keep-set: every declared id, from every memo."""
        return set(self.ids)
