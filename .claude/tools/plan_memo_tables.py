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
    grammar at the cell's START; a non-empty cell that does not start with an
    id is an UNKEYED row and a schema miss), kind markers from the disposed
    declaring field (a quoted marker declares nothing);
  * the population is transitive over the memos a memo links; the same id
    declared twice, and a reference no definition answers (the memo it meant
    to link is outside the population), are schema misses.

Every block is lexed ONCE, where it is minted (a cell in `split_row`, a
paragraph in `Paragraph`); `Memo` resolves the links; the disposition step
(`Population`) then tags each block's mask with the kind of every span --
`code` / `def` / `link` / `cite` / `file` -- minus the id-only code spans,
and `stream(lexed)` (every masked span blanked) is the ONE text each
predicate over a block reads.
"""

import pathlib
import re
from urllib.parse import unquote

from plan_memo_lexer import (
    Lexed, blank_spans, definition_block, delimiter_width, fenced_lines, is_blank, normalize_label,
    one_line_block, split_row, starts_block,
)

# A cell that carries nothing: the one predicate every reader of an optional
# cell (an id cell, a `Deps` cell) decides emptiness by.  Emptiness is decided
# by SHAPE -- after decoration is stripped, a cell with no alphanumeric
# character carries nothing (`—`, `-`, `–`, `--`, `…`, `` ` ``) -- plus the
# ONLY lexical exceptions, `EMPTY_WORDS`, matched exactly (case-insensitive)
# after the strip.  A closed word list on a gating predicate leaves the next
# spelling authoritative, so the list is the exception and the shape is the
# rule; the polarity of a word outside the list (`nil`, `(none)`) is a
# FALSE rc 1 -- the cell is read as a `Deps` edge and reported by assertion
# (b) -- never a silent skip.
EMPTY_WORDS = frozenset({"n/a", "none"})


def is_empty(cell_text):
    """Decoration does not fill a cell: `**—**` is as empty as `—`."""
    bare = cell_text.strip().strip("*`").strip()
    return not any(ch.isalnum() for ch in bare) or bare.casefold() in EMPTY_WORDS


# The ID cell is the one place the shape rule is WRONG: an id cell is either
# an id or a deliberate blank, and anything else (`?`, `…`, `**?**`, `--`) is
# an UNKEYED row -- the silent-skip class the schema miss exists for.  So the
# blanks are LITERAL here: exactly these four after decoration strip.
ID_CELL_BLANKS = frozenset({"", "\u2014", "-", "\u2013"})


def is_blank_id_cell(cell_text):
    """Whether an id cell is a deliberate non-row (a literal blank), as
    opposed to unkeyed content."""
    return cell_text.strip().strip("*`").strip() in ID_CELL_BLANKS


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

# An id-only code span is tokenised by the declared-id GRAMMAR, longest
# alternative first (a `#11-` slug is atomic -- its internal hyphens are not
# separators), with the separators whitespace, list punctuation, `|` and `-`
# (a `Deps`-shaped edge, `9z | 7z` / `0a-0b`) between tokens.  A bare id in
# a cell or in prose is NOT tokenised on a list -- it is bounded by the
# complement of the id-continuation class (the checker's `_ID_CONTINUES`); a
# hyphen bounds a short id, and `slice-9z-sib.md` is safe because a file
# name is a lexer `file` token, masked before the scan.
_ID_RUN_TOKEN = re.compile(r"(?P<id>%s|%s|%s)|(?P<sep>[\s,;/→>+&|-]+)" % (SLUG_ID, CITE_ID, SHORT_ID))

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
    header to the first blank line or block start (`Memo.block_start`: a heading,
    thematic break, list item, `>` line or a reference definition -- GFM
    §4.10 "the beginning of another block-level structure"); every line in
    between is a body row, pipes or not.
    """
    lines, fenced = memo.lines, memo.fenced
    tables, owned, i, n = [], set(), 0, len(lines)
    while i < n:
        if i in fenced or is_blank(lines[i]) or i + 1 >= n or memo.block_start(i):
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
        while j < n and j not in fenced and not is_blank(lines[j]) and not memo.block_start(j):
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
    (and separators) is the document SPELLING an id, and is a mention.  The
    run must be covered end to end by id tokens and separators."""
    if inner in keep:
        return True
    pos, ids = 0, []
    for m in _ID_RUN_TOKEN.finditer(inner):
        if m.start() != pos:
            return False
        pos = m.end()
        if m.group("id") is not None:
            ids.append(m.group("id"))
    return pos == len(inner) and bool(ids) and all(x in keep for x in ids)


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
    (start, end, kind) -- `code` (minus id-only spans and kept slugs), `link`
    (the tail; the visible text stays, it is prose), `image` (the same, for
    an image), `cite`, `file`.  A reference definition is a Phase-1 block of
    its own, never inline content, so no block holds one to mask."""
    out = [(a, b, "code") for a, b in code_mask(lx, keep)]
    out += [(a, b, "link") for a, b, _ in lx.links]
    out += [(a, b, "image") for a, b in lx.images]
    out += lx.tokens
    lx.mask = out


def stream(lx):
    """`lx.text` with EVERY span of its disposed mask blanked -- code spans
    (id-only spans and kept slugs were excepted there), link tails, citation
    ids, file names.  This is the ONE stream every predicate
    over a block reads: the kind-marker reader (a quoted marker is not a
    declaration), the seeds' vocabularies (a `gates` inside a code span is not
    ordering prose; a `MERGED` inside one is not a retirement), the licensing
    rule's context.  `dispose` first -- the mask is None before it."""
    assert lx.mask is not None, "stream() before dispose(): the Population has not run yet"
    return blank_spans(lx.text, [(a, b) for a, b, _ in lx.mask])


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
    """One memo, in the two phases of CommonMark's "Appendix: A parsing
    strategy".  Phase 1 (block structure, over RAW lines, here): fenced
    blocks (§4.5), GFM tables (§4.10, ending at a blank line or any block
    start), reference definitions (§4.7 -- a block of its own, recognised
    only at a block start; a definition-shaped line INSIDE a paragraph is an
    orphan, recorded in `orphans`), paragraphs.  Phase 2 (inline structure,
    `Lexed.resolve` / `inline_pass`) then runs over each paragraph's and
    cell's content only, with `defs` from the Phase-1 definition blocks."""

    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text()
        self.lines = self.text.split("\n")
        self.fenced = fenced_lines(self.lines)
        self._defs_at = {}          # line index -> definition_block(...) (read once per line)
        self.tables, self.table_lines = find_tables(self)
        self.defs = {}              # normalised label -> destination (§4.7: the first wins)
        self.orphans = {}           # normalised label -> {1-based line numbers}
        self.paragraphs = self._blocks()
        for lx in self.lexed():
            lx.resolve(self.defs)

    def definition_at(self, i):
        """The reference definition whose first raw line is `i`, or None
        (`definition_block`, computed once per line)."""
        if i not in self._defs_at:
            self._defs_at[i] = definition_block(self.lines, i)
        return self._defs_at[i]

    def block_start(self, i):
        """Whether raw line `i` begins a block-level structure other than a
        paragraph continuation: an ATX heading, a thematic break, a list
        item, a `>` line (`starts_block`) or a reference definition -- what
        ends a GFM table (§4.10: "the beginning of another block-level
        structure") and a paragraph."""
        return starts_block(self.lines[i]) or self.definition_at(i) is not None

    @property
    def key(self):
        """The memo's identity for every per-memo map (mention identity, the
        seeds' row maps): the RESOLVED path.  Two memos in different
        directories may share a basename, and a map keyed on the basename
        aliases their rows; the basename (`path.name`) is for display only."""
        return str(self.path)

    def _blocks(self):
        """Phase 1 over the lines no fence or table owns: definition blocks
        (filling `defs`, first wins) and paragraphs; a definition-shaped line
        that is not at a block start stays paragraph text and is an orphan.
        Linear: one three-line window per line."""
        out, cur, lines, i, n = [], [], self.lines, 0, len(self.lines)

        def flush():
            if cur:
                out.append(Paragraph(list(cur)))
                cur.clear()

        while i < n:
            line = lines[i]
            if i in self.fenced or i in self.table_lines or is_blank(line):
                flush()
                i += 1
                continue
            d = self.definition_at(i)
            if d is not None and not cur:
                # a block start: the definition is a block of its own
                raw, dest, k = d
                self.defs.setdefault(normalize_label(raw), dest)
                i += k
                continue
            if d is not None:
                # §4.7: a definition cannot interrupt a paragraph -- the line is
                # paragraph text the author meant as a definition
                self.orphans.setdefault(normalize_label(d[0]), set()).add(i + 1)
            elif starts_block(line):    # `d is None`: the other block starts
                flush()
            cur.append((i + 1, line))
            if one_line_block(line):
                flush()
            i += 1
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
        -- resolved beside it, in first-link order.  POLICY (not CommonMark
        §6.3 / GFM, which say nothing about siblings on disk): a memo's
        siblings are RELATIVE paths only: a destination with a scheme (`https:`,
        `mailto:`), a protocol-relative `//` host, or a root-relative `/`
        path (a site URL, which joined to the memo's directory would probe
        the host's filesystem root) is not a sibling on disk, whatever its
        path ends in."""
        out = []
        for lx in self.lexed():
            for _, _, dest in lx.links:
                # the PATH component, percent-decoded (`slice%20sib.md` is the
                # file `slice sib.md`, as `<slice sib.md>` is), validated ONCE
                # after decoding -- so `%2Ftmp%2Fx.md` is `/tmp/x.md` and is
                # rejected like a raw `/tmp/x.md` or `//host/x.md`
                name = unquote(re.split(r"[#?]", dest, 1)[0])
                if _SCHEME.match(name) or name.startswith("/") or not name.endswith(".md"):
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
        orphans, out = self.orphans, []

        def walk(lx, lineno_of):
            for off, label, form, is_image in lx.unresolved:
                if is_image:
                    continue     # §6.4: literal image syntax; an image never links a memo
                key = normalize_label(label)
                # a `[C19]`-style citation id is never a memo reference, in ANY
                # form -- shortcut, full (`[C19][C20]` adjacent citations) or
                # collapsed (`[C19][]`); a plain shortcut of any other label is
                # prose too.  An orphan definition of the label (one the grammar
                # could not read) is still reported, citation or not, except at
                # the definition's own bracket.
                # the FORM comes from the lexer's one bracket parse (escapes
                # honoured); a raw re-walk here once read `[foo\]][missing]`
                # as a shortcut and exempted it
                exempt = _CITE_LABEL.fullmatch(key) is not None or form == "shortcut"
                lineno = lineno_of(off)
                if exempt and (key not in orphans or lineno in orphans[key]):
                    continue
                site = (lineno, label)
                if site not in out:  # `[text][label]` re-scans `[label]` as a shortcut
                    out.append(site)

        for p in self.paragraphs:
            walk(p.lexed, lambda off, p=p: p.locate(off)[0])
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    walk(cell.lexed, lambda off, row=row: row.lineno)
        return out

    def schema_rows(self, name):
        """[Row] body rows of every table matching schema `name`."""
        return [r for t in self.tables if t.schema is not None and t.schema.name == name for r in t.rows]


_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
# `CITE_ID` without its brackets, over a NORMALISED (casefolded) label
_CITE_LABEL = re.compile(r"[a-z][0-9]+")


class Population:
    """The memo set reachable from one memo through its links (visited set, so
    a cycle is not an error), with ONE map `ids`: id -> its declaring `Row`
    (`row.kind` = "umbrella" / "undetermined" / "pointer" / "terminal").  `misses` holds
    every schema miss (absent memo, unresolved reference, unmatched schema, row
    width, duplicate declaration, unkeyed schema row); a non-empty `misses`
    is exit 2 -- never a clean run.
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
            # a reference no definition answers is prose under §6.3, and the
            # memo it meant to link is NOT in the population: never a clean run
            for lineno, label in memo.unresolved_references():
                self.misses.append((memo.path.name, lineno,
                                    "unresolved reference %r -- no definition answers it, so a memo "
                                    "it meant to link is NOT in the population" % label))
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
            row.field = stream(row.cells[row.schema.decl].lexed)
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
                    # a LITERAL blank id cell is a deliberate non-row; anything
                    # else that is not an id is an UNKEYED row: it would be dropped
                    # from `ids`, so assertion (b) would never see its Deps
                    # edge -- the I-C silent-skip class, and a schema miss
                    if not is_blank_id_cell(row.id_cell()):
                        self.misses.append((memo.path.name, row.lineno,
                                            "the %r row's id cell does not start with an id (%r); "
                                            "the row declares nothing and is unkeyed, so its cells "
                                            "would go unasserted" % (s.name, row.id_cell()[:60])))
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
        every memo -- the set `field` is written for and read over (assertion
        (a)).  It is NOT `ids.values()`: a row whose id cell is EMPTY (`**—**`)
        is a deliberate non-row, unkeyed and outside `ids`, but its declaring
        field is still read (the #506 memo has one such row, the
        `Function`/`eval` row); a non-empty non-id cell is a schema miss, so
        after `misses` those two sets differ by exactly the empty-id rows."""
        return [r for s in SCHEMAS if s.decl is not None and s.idc is not None
                for r in self.data_rows(s.name)]

    def keep(self):
        """The code-span keep-set: every declared id, from every memo."""
        return set(self.ids)
