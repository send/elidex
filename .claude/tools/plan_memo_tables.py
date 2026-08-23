#!/usr/bin/env python3
"""Row inventory for `plan-memo-umbrella-check.py` -- tables, rows, ids, kinds,
and the one transitive `Population` every scan and assertion reads.

Everything here answers "what rows does this document set have, what is each
one's id, and what kind does its declaring field declare?".  The lexical
substrate (fences, rows, code spans, links, `Lexed`) is `plan_memo_lexer.py`;
what the prose says about the rows, and whether that is allowed, is the
checker and `plan_memo_roles.py`.

Three rules decided here, once:
  * a table is admitted in `find_tables` and nowhere else: header and
    delimiter rows of equal width (GFM §4.10), and -- local policy over GFM's
    "may vary" clause -- a body row of a SCHEMA table whose width differs from
    the header is a schema miss (exit 2), never a silent skip or a shifted read;
  * row ids are read from the raw id cell (`bare_id` reads the one decorated-id
    grammar at the cell's START; a cell that does not start with an id declares
    nothing), kind markers from the MASKED declaring field (a quoted marker
    declares nothing);
  * the population is transitive over the memos a memo links, and the same id
    declared twice is a schema miss.

Every block is lexed ONCE, where it is minted (a cell in `split_row`, a
paragraph in `Paragraph`); `Memo` resolves the links; the disposition step
(`Population`) then tags each block's mask with the kind of every span --
`code` / `def` / `link` / `cite` / `file` -- minus the id-only code spans.
"""

import pathlib
import re

from plan_memo_lexer import (
    Lexed, blank_spans, delimiter_width, fenced_lines, is_blank, normalize_label,
    one_line_block, split_row, starts_block,
)

# A cell that carries nothing: the one predicate every reader of an optional
# cell (an id cell, a `Deps` cell) decides emptiness by.
EMPTY_CELL = frozenset({"", "\u2014", "-", "n/a"})


def is_empty(cell_text):
    """Decoration does not fill a cell: `**—**` is as empty as `—`."""
    return cell_text.strip().strip("*`").strip() in EMPTY_CELL


MARKER = "UMBRELLA, not a terminal unit"

# §5's fifth row kind.  A kind-undetermined row carries the split and NOTHING
# else -- no ordering, no owner, no acceptance -- which is the same obligation
# the naming rule enforces against umbrellas.  Two spellings are in use; both
# are tolerated and the divergence is reported (a kind with two spellings is a
# kind no program can enumerate).
UNDETERMINED = re.compile(r"KIND\s*[—-]?\s*UNDETERMINED", re.IGNORECASE)

# A row that is a POINTER into a slot rather than a slice of its own (§1.0's
# "SCHEDULED FROM ITS OWN SLOT" rows).  ⚠ Keyed on one spelling, and the safe
# polarity: a differently-spelled pointer row is terminal, and so REPORTED by
# the acceptance seed, never missed.
POINTER = re.compile(r"is a pointer rather than a slice")

# --------------------------------------------------------------------------
# Id grammar, spelled ONCE.  An id is a short alphanumeric token, a `#11-`
# slug, or a `[C19]`-style citation id (the citation table's id column) --
# nothing else -- and may be decorated with bold, backticks, or both, in
# either order.  `decorated_id(core, tag)` is the one spelling every reader
# uses (the id cell, the prose anchor, the bare cell token, the owner
# reference); it binds groups `<tag>l` / `<tag>id` / `<tag>r`, and
# `balanced(m, tag)` says whether the decoration closes what it opened.
# --------------------------------------------------------------------------

SHORT_ID = r"[0-9A-Za-z]{1,4}"
SLUG_ID = r"#11-[a-z0-9-]+"
CITE_ID = r"\[[A-Z][0-9]+\]"
DECOR = r"(?:\*\*|`)*"


def decorated_id(core, tag=""):
    return r"(?P<%sl>%s)(?P<%sid>%s)(?P<%sr>%s)" % (tag, DECOR, tag, core, tag, DECOR)


_DECOR_TOKENS = re.compile(r"\*\*|`")


def balanced(m, tag=""):
    """The decoration around `<tag>id` closes, in reverse order, exactly what
    it opened -- `**x**`, `` `x` ``, `` **`x`** `` -- and is not empty."""
    left = _DECOR_TOKENS.findall(m.group(tag + "l"))
    return bool(left) and _DECOR_TOKENS.findall(m.group(tag + "r")) == left[::-1]


ROW_NOUN = r"(?:Slices?|slices?|Rows?|rows?|Umbrellas?|umbrellas?)"
"""How this document names a row when it refers to one.  Lives here because it
is a fact about row IDENTITY -- `attributed_to_other` needs it to decide whose
kind a marker declares."""

# `Slice-M` / `Slice-4a` are the same anchor with a hyphen.  Requiring `\s+`
# left them invisible to both passes; measured, four of five such sites in this
# memo are real violations.
ROW_NOUN_ID = ROW_NOUN + r"[\s-]+" + decorated_id(SHORT_ID)

# The id cell: the grammar at the cell's START, then a non-id character (so
# `**7z** — MERGED` and `` `#11-x` (carved from #483) `` read `7z` / `#11-x`,
# and `xxxxC` is not `C`).  A cell that does not start with an id is not an
# id cell.
_ID_CELL = re.compile("^" + decorated_id("(?:%s|%s|%s)" % (SLUG_ID, CITE_ID, SHORT_ID))
                      + r"(?![0-9A-Za-z-])")
_SLUG_IN_CODE = re.compile(r"(?<![\w-])" + SLUG_ID)

# The separators an id run is tokenised on.  `ID_SEP` is the shared core; an
# id-only code span also splits on `|` and `-` (a `Deps`-shaped edge, `9z | 7z`
# / `0a-0b`), while a cell boundary also splits on brackets and `·` but NOT on
# `-` (a hyphen glues `slice-9z-sib` into one token, which is not an id).
ID_SEP = r"\s,;/→>+&"
_ID_RUN_SPLIT = re.compile("[" + ID_SEP + "|-]+")
CELL_SPLIT = re.compile("[" + ID_SEP + r"()\[\]·]+")

# --------------------------------------------------------------------------
# Table schemas, identified by HEADER ROW (a line range is a figure a later
# edit silently invalidates).  Column indexes are BODY columns: the optional
# leading pipe is stripped at admission, so column 0 is the first cell.
# --------------------------------------------------------------------------


class Schema:
    """A table family: its exact header cells (trimmed, in order), the column
    whose text DECLARES the row's kind, and the column holding the row id --
    both named by header cell and resolved to body-column indexes here."""

    __slots__ = ("name", "header", "decl", "idc")

    def __init__(self, name, header, decl=None, idc=None):
        self.name, self.header = name, header
        self.decl = header.index(decl) if decl is not None else None
        self.idc = header.index(idc) if idc is not None else None


SCHEMAS = [
    Schema("citation", ["ID", "Citation", "Anchor", "Used by"], idc="ID"),
    Schema("stub", ["Site", "Syntax", "Emits", "Observable", "Tier", "Slice"]),
    Schema("slice", ["#", "Slice", "Primary module(s)", "Slot", "Tier", "Deps"], decl="Slice", idc="#"),
    Schema("slot", ["Slot", "Why deferred", "Trigger", "Re-eval"], decl="Why deferred", idc="Slot"),
]


class Row:
    """One table row, minted once at admission: its cells, its schema (None for
    a non-schema table or a header row), its own id (`self_id`, from the raw
    id cell), and -- set by `Population` -- `field`, the masked declaring
    field, and `kind` ("umbrella" / "undetermined" / "pointer" / "terminal")."""

    __slots__ = ("memo", "lineno", "cells", "schema", "self_id", "field", "kind")

    def __init__(self, memo, lineno, cells, schema):
        self.memo, self.lineno, self.cells, self.schema = memo, lineno, cells, schema
        self.self_id = bare_id(cells[schema.idc].text) if schema and schema.idc is not None else None
        self.field, self.kind = None, None

    def id_cell(self):
        """The raw text of this row's id cell (None for a row without one)."""
        return self.cells[self.schema.idc].text if self.schema and self.schema.idc is not None else None

    def col(self, header_cell):
        """The cell under the schema's header cell named `header_cell`."""
        return self.cells[self.schema.header.index(header_cell)]


class Table:
    __slots__ = ("schema", "header", "rows", "misses")

    def __init__(self, schema, header):
        self.schema = schema            # Schema or None
        self.header = header            # Row (schema None)
        self.rows = []                  # [Row], body rows only
        self.misses = []                # [(lineno, message)] width policy


def find_tables(memo):
    """Admit every GFM table in `memo` (fenced lines skipped).  Returns
    ([Table], {0-based line index owned by a table}).  The table runs from the
    header to the first blank line or block start; every line in between is a
    body row, pipes or not (GFM §4.10).
    """
    lines, fenced = memo.lines, memo.fenced
    tables, owned, i, n = [], set(), 0, len(lines)
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
        schema = next((s for s in SCHEMAS if hdr_text == s.header), None)
        t = Table(schema, Row(memo, i + 1, header, None))
        owned.update((i, i + 1))
        j = i + 2
        while j < n and j not in fenced and not is_blank(lines[j]) and not starts_block(lines[j]):
            body = split_row(lines[j])
            if schema is not None and len(body) != width:
                t.misses.append((j + 1, "row has %d cell(s); the %r header has %d -- "
                                 "a shifted read fabricates findings, so this row is "
                                 "unscanned" % (len(body), schema.name, width)))
            else:
                t.rows.append(Row(memo, j + 1, body, schema))
            owned.add(j)
            j += 1
        tables.append(t)
        i = j
    return tables, owned


# --------------------------------------------------------------------------
# Row identity
# --------------------------------------------------------------------------

def bare_id(cell_text):
    """The row id an id cell declares: the decorated-id grammar at the cell's
    start, decoration stripped, trailing prose ignored -- or None when the
    cell does not start with an id (then the row declares nothing)."""
    g = _ID_CELL.match(cell_text.strip())
    return g.group("id") if g else None


_APPOSITIVE = re.compile(ROW_NOUN_ID + r"\s*[—–-]\s*" + DECOR + r"\s*$")


def attributed_to_other(field, rid):
    """The row id a marker names, when it is not this row's own: the marker's
    APPOSITIVE subject -- `Slice **E** — **UMBRELLA, …**` -- with nothing
    between them but dash punctuation and emphasis.  A proximity window
    instead excluded a genuine self-declaration that merely MENTIONED a sibling
    ("Unlike Slice 7z, **UMBRELLA, not a terminal unit.**"); the self-test
    carries both directions.  The FIRST marker occurrence decides: a field
    that declares itself and then says a sibling "is not it" is
    self-declaring, and a later occurrence never overrides the first."""
    m = re.search(re.escape(MARKER), field)
    if m is None:
        return None
    g = _APPOSITIVE.search(field[max(0, m.start() - 70): m.start()])
    if g and g.group("id") != rid:
        return g.group("id")
    return None


# --------------------------------------------------------------------------
# Disposition: the one place a lexical span meets the row ids
# --------------------------------------------------------------------------


def id_only(inner, keep):
    """The disposition exception: a code span whose content is only row ids
    (and separators) is the document SPELLING an id, and is a mention."""
    toks = [x for x in _ID_RUN_SPLIT.split(inner) if x]
    return inner in keep or (bool(toks) and all(x in keep for x in toks))


def code_mask(lx, keep):
    """Code spans of `lx` minus the id-only ones, and minus the `#11-` slugs
    of `keep` INSIDE the remaining spans (a backticked slug, alone or in a
    command line, is the document spelling an id: the same exception) -- the
    spans a reader of prose must skip."""
    out = []
    for a, b in lx.code:
        if id_only(lx.text[a:b].strip("`").strip(), keep):
            continue
        cut = a
        for m in _SLUG_IN_CODE.finditer(lx.text, a, b):
            if m.group(0) in keep:
                out.append((cut, m.start()))
                cut = m.end()
        out.append((cut, b))
    return out


def dispose(lx, keep):
    """Tag `lx.mask`: every span the scanners must not read an id out of, as
    (start, end, kind) -- `code` (minus id-only spans and kept slugs), `def`
    (a definition renders nothing, so none of it, label included, is prose),
    `link` (the tail; the visible text stays, it is prose), `cite`, `file`."""
    out = [(a, b, "code") for a, b in code_mask(lx, keep)]
    if lx.defs_end:
        out.append((0, lx.defs_end, "def"))
    out += [(a, b, "link") for a, b, _ in lx.links]
    out += lx.tokens
    lx.mask = out


def prose(lx):
    """`lx.text` with the code spans of its disposed mask blanked (id-only
    spans and kept slugs were excepted there) -- the stream the kind-marker
    reader reads (a quoted marker is not a declaration).  `dispose` first."""
    return blank_spans(lx.text, [(a, b) for a, b, kind in lx.mask if kind == "code"])


class Paragraph:
    """Lines outside tables and fences, grouped at blank lines and block
    starts; `lexed.text` is the inline content code spans are lexed over, and
    `offsets` maps each line to its start offset in it (a line is a reporting
    coordinate only)."""

    __slots__ = ("lines", "offsets", "lexed")

    def __init__(self, numbered):
        self.lines = numbered                       # [(lineno, text)]
        self.lexed = Lexed("\n".join(t for _, t in numbered))
        self.offsets = []
        off = 0
        for _, t in numbered:
            self.offsets.append(off)
            off += len(t) + 1

    def locate(self, i):
        """Offset `i` of the content -> (lineno, line text, column)."""
        k = 0
        while k + 1 < len(self.offsets) and self.offsets[k + 1] <= i:
            k += 1
        lineno, text = self.lines[k]
        return lineno, text, i - self.offsets[k]


class Memo:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text()
        self.lines = self.text.split("\n")
        self.fenced = fenced_lines(self.lines)
        self.tables, self.table_lines = find_tables(self)
        self.paragraphs = self._paragraphs()
        # §4.7: the first definition of a label wins
        self.defs = {}
        for p in self.paragraphs:
            for raw, dest, _ in p.lexed.definitions:
                self.defs.setdefault(normalize_label(raw), dest)
        for lx in self.lexed():
            lx.resolve(self.defs)

    def _paragraphs(self):
        out, cur = [], []

        def flush():
            if cur:
                out.append(Paragraph(list(cur)))
                cur.clear()

        for i, line in enumerate(self.lines):
            if i in self.fenced or i in self.table_lines or is_blank(line):
                flush()
                continue
            if starts_block(line):
                flush()
            cur.append((i + 1, line))
            if one_line_block(line):
                flush()
        flush()
        return out

    def lexed(self):
        """Every lexed block of this memo: each cell of each table row (header
        rows too), then each paragraph."""
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    yield cell.lexed
        for p in self.paragraphs:
            yield p.lexed

    def linked_files(self):
        """Every LOCAL `.md` this memo links -- from any block, cells included
        -- resolved beside it, in first-link order.  A destination with a
        scheme (`https:`, `mailto:`) or a protocol-relative `//` host is not a
        sibling on disk, whatever its path ends in."""
        out = []
        for lx in self.lexed():
            for _, _, dest in lx.links:
                if _SCHEME.match(dest) or dest.startswith("//"):
                    continue
                name = re.split(r"[#?]", dest, 1)[0]
                if not name.endswith(".md"):
                    continue
                f = (self.path.parent / name).resolve()
                if f != self.path.resolve() and f not in out:
                    out.append(f)
        return out

    def unresolved_references(self):
        """[(lineno, label)]: every full / collapsed reference no definition
        answers, plus every shortcut whose label has a definition the grammar
        could not read (a §4.7 definition cannot interrupt a paragraph) -- the
        sites where a population the author meant to link is lost."""
        orphans = {}         # normalised label -> {offsets of its definition lines}
        for p in self.paragraphs:
            for off, raw in p.lexed.orphan_definitions():
                orphans.setdefault(normalize_label(raw), set()).add((p, off))
        out = []

        def walk(lx, lineno_of, block=None):
            for off, label in lx.unresolved:
                key = normalize_label(label)
                if _is_shortcut(lx, off) and (key not in orphans or (block, off) in orphans[key]):
                    continue     # a plain `[C19]`, or the orphan definition's own bracket
                site = (lineno_of(off), label)
                if site not in out:  # `[text][label]` re-scans `[label]` as a shortcut
                    out.append(site)

        for p in self.paragraphs:
            walk(p.lexed, lambda off, p=p: p.locate(off)[0], p)
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    walk(cell.lexed, lambda off, row=row: row.lineno)
        return out

    def schema_rows(self, name):
        """[Row] body rows of every table matching schema `name`."""
        return [r for t in self.tables if t.schema is not None and t.schema.name == name for r in t.rows]


_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def _is_shortcut(lx, off):
    """Whether the unresolved reference at `off` is a bare `[text]` (no `[`
    follows its closing bracket) rather than a full / collapsed form."""
    depth, j, s = 0, off, lx.text
    while j < len(s):
        if s[j] == "[":
            depth += 1
        elif s[j] == "]":
            depth -= 1
            if depth == 0:
                return not (j + 1 < len(s) and s[j + 1] == "[")
        j += 1
    return True


class Population:
    """The memo set reachable from one memo through its links (visited set, so
    a cycle is not an error), with ONE map `ids`: id -> its declaring `Row`
    (`row.kind` = "umbrella" / "undetermined" / "pointer" / "terminal").  `misses` holds
    every schema miss (absent memo, unmatched schema, row width, duplicate
    declaration); a non-empty `misses` is exit 2 -- never a clean run.
    """

    def __init__(self, main_path):
        self.memos = []
        self.misses = []            # [(file, lineno, message)]
        self.spellings = set()
        self.attributed = []        # [(file, table, lineno, rid, other)]
        self.undeclared = []        # [(file, lineno, schema, id cell text)]
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
            matched = {t.schema.name for t in self.main.tables if t.schema is not None}
            for s in SCHEMAS:
                if s.name not in matched:
                    self.misses.append((self.main.path.name, 0,
                                        "no table matched schema %r -- its whole population is unscanned" % s.name))
        for memo in self.memos:
            for t in memo.tables:
                for lineno, msg in t.misses:
                    self.misses.append((memo.path.name, lineno, msg))
        # ids first (the keep-set the disposition needs), then masks, then kinds
        for memo in self.memos:
            self._declare(memo)
        keep = self.keep()
        for memo in self.memos:
            for lx in memo.lexed():
                dispose(lx, keep)
        for row in self.declaring_rows():
            row.field = prose(row.cells[row.schema.decl].lexed)
        for row in self.ids.values():
            row.kind = self._kind(row)

    # -- declarations ------------------------------------------------------

    def _declare(self, memo):
        for s in SCHEMAS:
            if s.idc is None:
                continue
            for row in memo.schema_rows(s.name):
                rid = row.self_id
                if rid is None:
                    # an empty id cell is a deliberate non-row; anything else
                    # that is not an id is reported (never minted as one)
                    if not is_empty(row.id_cell()):
                        self.undeclared.append((memo.path.name, row.lineno, s.name, row.id_cell()))
                    continue
                if rid in self.ids:
                    r2 = self.ids[rid]
                    self.misses.append((memo.path.name, row.lineno,
                                        "row %r is declared twice (also %s:%d in %r); a population "
                                        "with two declarations of one id cannot be scanned"
                                        % (rid, r2.memo.path.name, r2.lineno, r2.schema.name)))
                    continue
                self.ids[rid] = row

    def _kind(self, row):
        """The kind the row's masked declaring field declares.  The
        undetermined SPELLING is collected independently of the marker (a row
        can carry both; its kind stays umbrella, its spelling still joins
        `spellings`).  A row whose marker is attributed to another row is a
        POINTER (§5: a pointer slot carries no marker of its own -- assertion
        (a) reports it), as is a row that says so in words."""
        if row.field is None:
            return "terminal"
        m = UNDETERMINED.search(row.field)
        if m:
            self.spellings.add(m.group(0))
        if MARKER in row.field:
            other = attributed_to_other(row.field, row.self_id)
            if other:
                self.attributed.append((row.memo.path.name, row.schema.name, row.lineno, row.self_id, other))
                return "pointer"
            return "umbrella"
        if m:
            return "undetermined"
        if POINTER.search(row.field):
            return "pointer"
        return "terminal"

    # -- inventories -------------------------------------------------------

    def ids_of_kind(self, kind):
        return {rid: r for rid, r in self.ids.items() if r.kind == kind}

    def no_owner_ids(self):
        """Every row that carries no owner and no ordering -- the property §5's
        naming rule is stated over (umbrella + kind-undetermined)."""
        return {rid: r for rid, r in self.ids.items() if r.kind in ("umbrella", "undetermined")}

    def data_rows(self, name):
        """[Row] over every memo, for schema `name`."""
        return [r for memo in self.memos for r in memo.schema_rows(name)]

    def declaring_rows(self):
        """Every row of a schema with a declaring field and an id column, over
        every memo -- id-less rows included, since their field is read too."""
        return [r for s in SCHEMAS if s.decl is not None and s.idc is not None
                for r in self.data_rows(s.name)]

    def keep(self):
        """The code-span keep-set: every declared id, from every memo."""
        return set(self.ids)
