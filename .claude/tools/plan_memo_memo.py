#!/usr/bin/env python3
"""One memo, and the memo set reachable from it -- for
`plan-memo-umbrella-check.py`.

`Memo` is the Phase-1 driver of CommonMark's "Appendix: A parsing strategy"
(`_parse` / `_quote`: ONE forward pass over raw lines with the open block in
hand, re-entered once per block quote), the Phase-2 resolution of its
paragraphs and cells (`Lexed.resolve` with the Phase-1 definitions), the
file I/O (`read_text(encoding="utf-8")`), and the ONE destination -> sibling
resolver (`sibling_path` / `_resolve` / `linked_files` /
`unresolved_references`).  `Population` is the transitive closure over the
memos a memo links (a visited set, so a cycle is not an error) and the ONE
map `ids` every scan and assertion reads.  The row grammar, the table
schemas and the one admission site (`admit_table`), and the mask
disposition are `plan_memo_tables.py`'s, imported here and never the
reverse; the block grammar is `plan_memo_blocks.py`'s.

Two rules decided here, once:
  * a memo that cannot be opened, read or decoded is an UNAVAILABLE linked
    memo -- the documented exit-2 miss at the population's one I/O
    chokepoint, never an exception out of the population;
  * the population is transitive over the memos a memo links; the same id
    declared twice, and a reference no definition answers (the memo it meant
    to link is outside the population), are schema misses.
"""

import bisect
import pathlib
import re
from urllib.parse import unquote

from plan_memo_blocks import (
    block_end, container_text, definition_block, indentation, is_blank, list_item_line,
    one_line_block, quote_content, raw_extent, raw_opener, run_end, setext_underline, starts_block,
    table_header_at,
)
from plan_memo_lexer import Lexed, normalize_label
from plan_memo_tables import (
    MARKER, POINTER, SCHEMAS, UNDETERMINED, admit_table, attributed_to_other, dispose,
    is_blank_id_cell, stream,
)


class Paragraph:
    """The lines of one run no definition consumed, grouped by the driver
    (`Memo._parse`) at the block boundaries; `lexed.text` is the inline
    content code spans are lexed over, and `offsets` maps each line to its
    start offset in it (a line is a reporting coordinate only)."""

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
        """Offset `i` of the content -> (lineno, line text, column).  A
        bisect over the line offsets: a linear scan per call made every
        per-site lookup quadratic in the paragraph's length (8,000 reference
        lines: 6.4 s, 16,000 calls)."""
        k = bisect.bisect_right(self.offsets, i) - 1
        lineno, text = self.lines[k]
        return lineno, text, i - self.offsets[k]


class Memo:
    """One memo, in the two phases of CommonMark's "Appendix: A parsing
    strategy".  Phase 1 (block structure, over RAW lines, `_parse`: ONE
    forward pass, the way the Appendix reads a document line by line with
    its open block in hand, re-entered once per block quote over the quote's
    content): the raw extents -- indented code (§4.4), fenced blocks (§4.5)
    and HTML blocks (§4.6), one opener rule and one extent rule, consumed in
    place -- block quotes (§5.1, the one container: marker lines stripped,
    lazy continuation lines gathered, the same pass over the content), GFM
    tables (§4.10, ending at a blank line or any block start), runs and
    their reference definitions (§4.7 -- a block of its own, recognised
    only at a block start; a definition-shaped line INSIDE a paragraph is an
    orphan, recorded in `orphans`), paragraphs.  Phase 1 STATES its result
    as a block sequence (`sequence`: `[kind, first raw line, last raw line]`
    in document order, a quote before its content) -- the claim the
    conformance control consumes against the spec's html.  Phase 2 (inline
    structure, `Lexed.resolve` / `inline_pass`) then runs over each
    paragraph's and cell's content only, with `defs` from the Phase-1
    definition blocks."""

    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.text = self.path.read_text(encoding="utf-8")   # not the locale's codec
        self.lines = self.text.split("\n")
        if len(self.lines) > 1 and self.lines[-1] == "":
            self.lines.pop()        # a line ending ENDS the last line (§2.1); it begins no empty one
        self.tables = []            # [Table], in document order
        self.sequence = []          # Phase 1's block sequence: [kind, first lineno, last lineno]
        self.defs = {}              # normalised label -> destination (§4.7: the first wins)
        self.orphans = {}           # normalised label -> {1-based line numbers}
        # [(lineno, content line, reading)] every line of a raw extent that is
        # not a fence, in ONE list -- the LEX-UNSUPPORTED? seed's population,
        # under one seed rule; `reading` says which grammar hides the line:
        # "html" (an HTML block, §4.6), "indented" (indented code, §4.4), or
        # "item" (indented code opened while a list item may still be open
        # under LEXED-FLAT -- the item's content under CommonMark, §5.2
        # Example 108).  A fence is not recorded: it is the author's explicit
        # code marker, where an indented `| row |` is the I-C silent-skip class.
        self.raw = []
        self.paragraphs, _ = self._parse(self.lines, list(range(1, len(self.lines) + 1)), None)
        for lx in self.lexed():
            lx.resolve(self.defs)

    def _parse(self, lines, linenos, lazy):
        """Phase 1 as ONE forward pass over one container's content `lines`
        (raw line numbers `linenos`; `lazy[i]`, None at the document level,
        marks a block quote's line that carried no marker -- a §5.1 lazy
        continuation candidate), the block STATE in hand -- which of a raw
        extent, a table or a run is open -- and every boundary decided by
        the ONE predicate `block_end` (a line inside a run was already found
        not to be one, with a paragraph open; a line outside a run is asked
        with none open, so a type-7 HTML opener or an indented line after a
        table, a one-line block or a setext heading opens a raw extent there
        and stays paragraph text after a run line).  Returns (paragraphs,
        lines consumed): the pass STOPS at a lazy candidate where no
        paragraph is open -- such a line is content only as paragraph
        continuation text, so the quote ends there (`> # h\\nlazy` is a
        heading in the quote and a paragraph after it; the spec's instance
        is Example 237, `> ```\\nfoo\\n```` -- an unclosed fence in the quote,
        then a paragraph and a fence outside it).  With a paragraph open
        (`cur` holds its lines: the run ended AT this line) the ONE boundary
        a lazy line can be is a GFM table header whose delimiter row carries
        the marker -- the only `block_end` arm that needs the next line,
        which `_quote`'s gather could not see -- and that line is CONTENT:
        cmark-gfm reads the header out of the paragraph's last line
        (`> a\\n| h |\\n> |---|\\n> | 1 |` is quote[p(a), table(h; 1)],
        measured), so the table is admitted there instead of the quote
        ending.  (Measured divergence, not modelled: after a reference
        DEFINITION cmark-gfm still hands the lazy line to the table and
        prints the definition as a paragraph, `> [a]: /u\\n| h |\\n> |---|`;
        here a definition is a block of its own, so the quote ends.)  Each
        line is classified once, in order:

          * a raw-extent opener (`raw_opener`; indented code, fences and HTML
            blocks share the one rule): its lines to `raw_extent` are never
            inline-parsed and are consumed here; every line of an HTML block
            (§4.6) or an indented code block (§4.4) is recorded in `raw`
            with its READING for the LEX-UNSUPPORTED? seed (one seed rule
            for raw lines: a line holding a `|` or a declared id is
            printed, never assumed -- an indented schema row after a
            table's rows is raw under cmark-gfm too, and the I-C
            silent-skip class without the seed), the reading being "item"
            where a LIST ITEM may still be open under LEXED-FLAT: under §5.2
            that is the item's next paragraph (Example 108 `- foo\\n\\n
            bar`), the one place the flat reading hides prose.  THE
            ITEM-OPEN BIT (`item_open`, the flat reading's whole model of
            §5.2 nesting): SET when a paragraph headed by a list-marker line
            is flushed; CLEARED by a block start -- any line classified
            outside a run, a blank line excepted -- whose indentation is
            below 2 columns (the smallest content indent any marker gives,
            `- `; the item's real content indent is not tracked, so a
            paragraph at 2 columns after `1. ` keeps the bit and the seed
            over-approximates -- a seed, never an inventory).  Verified
            against commonmark.js 0.31.2: `- item\\n\\n  para\\n\\n    x`,
            `- item\\n\\n  > q\\n\\n    x` and `1. item\\n\\n   para\\n\\n
            x` are each a `<p>x</p>` inside the item (the bit survives the
            2-column paragraph, the quote, the 3-column paragraph); `-
            item\\n\\n para\\n\\n    x` and `- item\\n\\n  para\\n\\n# h\\n\\n
            x` are a code block outside it (a 1-column paragraph, a
            0-column heading, close the item).  A one-block memory ("the
            last block is the item's paragraph") lost the bit at the
            2-column paragraph and read the prose after it as raw, unseeded
            (design re-gate 3, IMP-1);
          * a blank line ends the paragraph;
          * a `>` line opens a block quote: `_quote`, the same pass over
            its content;
          * a GFM table header off a block start: `admit_table`, the one
            admission site;
          * a setext underline after paragraph text (§4.3; not after a run
            headed by a list-item line, Example 94 -- `container_text`): the
            paragraph is the heading, at the level `setext_underline` reads
            (the one reading of the underline), and the underline closes
            it, content of nothing; with no paragraph open the line is not
            an underline at all (`block_end`'s `para_open` arm) -- `---` is
            a thematic break and `===` paragraph text;
          * otherwise a RUN starts (`run_end`; joined once, each line mapped
            to its offset, the text a definition is parsed over) and its
            lines are read one by one: a definition at a block start is a
            block of its own (`defs`, first wins); a VALID definition that
            cannot take effect because a paragraph is open ("a link
            reference definition cannot interrupt a paragraph") is an
            ORPHAN -- exactly that class: a label-and-colon line that is not
            a valid definition is plain prose (commonmark.js: `[C1]:
            ECMA-262 §1 says so` is a paragraph), and a shortcut naming it is
            exempt; the rest is paragraph text, grouped so that a run start
            begins a new paragraph (a lazy setext-shaped line after a list
            item is the item's text) and a one-line block is a paragraph of
            its own.

        Linear: one lookahead per run, one parse per line."""
        out, cur, i, n = [], [], 0, len(lines)
        run_text, run_off, defs_at = {}, {}, {}
        sequence = self.sequence
        item_open = False       # LEXED-FLAT: a list item may still be open (the docstring's bit)

        def flush(kind="p", last=None):
            nonlocal item_open
            if cur:
                sequence.append([kind, cur[0][0], cur[-1][0] if last is None else last])
                out.append(Paragraph(list(cur)))
                if kind == "p" and list_item_line(cur[0][1]):
                    item_open = True
                cur.clear()

        def close(line):
            # a block start: the paragraph before it is closed, and -- at
            # fewer than 2 columns of indentation -- so is the list item
            # (the bit's CLEAR rule; a blank line closes neither)
            nonlocal item_open
            flush()
            if not is_blank(line) and indentation(line)[0] < 2:
                item_open = False

        def definition_at(i):
            # the reference definition starting at content line `i`, parsed
            # over the rest of its run (`definition_block`; once per line)
            if i not in defs_at:
                defs_at[i] = definition_block(run_text[i], run_off[i])
            return defs_at[i]

        while i < n:
            line = lines[i]
            new_run = i not in run_text
            if new_run:
                if lazy is not None and lazy[i] and not (cur and table_header_at(lines, i, lazy)):
                    break       # §5.1: no paragraph is open, so the quote ends here
                # outside a run no paragraph is open: the block state is the
                # run map itself, not a look at the previous line
                opener = raw_opener(line, False)
                if opener is not None:
                    close(line)
                    end = raw_extent(lines, i, opener, lazy)
                    if opener[0] != "fence":
                        reading = "item" if opener[0] == "indented" and item_open else opener[0]
                        self.raw.extend((linenos[k], lines[k], reading) for k in range(i, end))
                    sequence.append([opener[0], linenos[i], linenos[end - 1]])
                    i = end
                    continue
                if is_blank(line):
                    close(line)
                    i += 1
                    continue
                if quote_content(line) is not None:
                    close(line)
                    i += self._quote(lines, linenos, i, out)
                    continue
                if not starts_block(line) and table_header_at(lines, i, lazy):
                    close(line)
                    t, end = admit_table(self, lines, linenos, i, lazy)
                    self.tables.append(t)
                    sequence.append(["table", linenos[i], linenos[end - 1]])
                    i = end
                    continue
                heading = setext_underline(line) if cur else None
                if heading is not None and not container_text(cur[0][1]):
                    # §4.3: the paragraph is a heading; the underline closes
                    # it and is not content
                    flush(heading, linenos[i])
                    i += 1
                    continue
                j = run_end(lines, i, lazy)
                text, off = "\n".join(lines[i:j]), 0
                for k in range(i, j):
                    run_text[k], run_off[k] = text, off
                    off += len(lines[k]) + 1
            d = definition_at(i)
            if d is not None and not cur:
                # a block start: the definition is a block of its own
                raw, dest, stop = d
                self.defs.setdefault(normalize_label(raw), dest)
                consumed = run_text[i][run_off[i]:stop]
                k = consumed.count("\n") + (0 if consumed.endswith("\n") else 1)
                sequence.append(["def", linenos[i], linenos[i + k - 1]])
                close(line)
                i += k
                continue
            if d is not None:
                # a valid definition that cannot take effect: the orphan
                self.orphans.setdefault(normalize_label(d[0]), set()).add(linenos[i])
            elif new_run and not (cur and setext_underline(line) is not None and not one_line_block(line)):
                # a run start begins a paragraph -- unless it is the lazy
                # `==` after a list item (Example 94), which is that
                # paragraph's text
                close(line)
            cur.append((linenos[i], line))
            kind = one_line_block(line)
            if kind:
                flush(kind)
            i += 1
        flush()
        return out, i

    def _quote(self, lines, linenos, i, out):
        """The block quote opening at content line `i` (a `>` line, §5.1)
        -> the number of lines it spans; its paragraphs are appended to
        `out`, its tables / definitions / raw lines / sequence entries to
        this memo's like every other block's.  The content: every marker
        line stripped (`quote_content`) and, between and after the marker
        lines, the lines without a marker as LAZY CONTINUATION CANDIDATES --
        content only as paragraph continuation text (§5.1, verbatim: "the
        result of deleting the initial block quote marker from one or more
        lines in which the next character other than a space or tab after
        the block quote marker is paragraph continuation text is a block
        quote with Bs as its content") -- run through the
        SAME `_parse`, so a definition inside registers (Example 218), a
        table inside is a table, a raw extent inside is raw, a paragraph
        inside is a paragraph at its real line, and a nested quote is the
        same again.  Where a candidate is a boundary even with a paragraph
        open (`block_end`: a blank line, a fence, a heading, a list item --
        never a setext underline, Example 93) no quote reaches it, so the
        candidates gathered here stop there; `_parse` stops earlier at a
        candidate where no paragraph is open (after a raw extent, a table, a
        heading: Example 237 `> ```\\nfoo\\n```` -- ⚠ an earlier docstring
        cited Examples 128 / 174, which end their quotes at a BLANK line,
        this gather's stop, not `_parse`'s) -- or hands it to a table when
        it is the header the paragraph's last line becomes.  Linear: each line of the document is
        gathered once per enclosing quote, never re-scanned across quotes."""
        n = len(lines)
        content, nos, inner_lazy, j = [], [], [], i
        while j < n:
            rest = quote_content(lines[j])
            if rest is None:
                content.append(lines[j])
                nos.append(linenos[j])
                inner_lazy.append(True)
                if block_end(content, len(content) - 1, True, inner_lazy):
                    content.pop()
                    nos.pop()
                    inner_lazy.pop()
                    break
            else:
                content.append(rest)
                nos.append(linenos[j])
                inner_lazy.append(False)
            j += 1
        entry = ["quote", linenos[i], None]
        self.sequence.append(entry)
        paragraphs, used = self._parse(content, nos, inner_lazy)
        out.extend(paragraphs)
        entry[2] = nos[used - 1]
        return used

    @property
    def key(self):
        """The memo's identity for every per-memo map (mention identity, the
        seeds' row maps): the RESOLVED path.  Two memos in different
        directories may share a basename, and a map keyed on the basename
        aliases their rows; the basename (`path.name`) is for display only."""
        return str(self.path)

    def lexed(self):
        """Every lexed block of this memo: each cell of each table row (header
        rows too), then each paragraph."""
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    yield cell.lexed
        for p in self.paragraphs:
            yield p.lexed

    def sibling_path(self, dest):
        """The ONE destination -> sibling mapping: the memo on disk a link
        destination names, or None when it names none.  POLICY (CommonMark
        §6.3 / GFM say nothing about siblings on disk): a sibling is a
        RELATIVE `.md` path beside this memo.  Stages, in spec order:
          (a) the scheme test on the RAW path component -- per WHATWG URL a
              scheme is read before percent-decoding, so `notes%3Achild.md`
              has no scheme: it is the local file `notes:child.md`;
          (b) percent-decode (`slice%20sib.md` is `slice sib.md`, as
              `<slice sib.md>` is);
          (c) the DECODED name must not be absolute (`/x`, `//host/x` -- a
              site URL joined to the memo's directory would probe the host's
              filesystem root) nor hold a C0 control / DEL (`child%00.md`
              would make `resolve()` raise);
          (d) the `.md` suffix;
          (e) `resolve()` beside the memo (`_resolve`); an `OSError` or --
              on Python 3.9-3.12, for a symlink loop -- a `RuntimeError`
              there makes the sibling UNAVAILABLE: the joined, unresolved
              path is returned and the population's one I/O chokepoint
              reports it as an unavailable linked memo (exit 2), never a
              crash, never a silent drop.
        """
        raw = re.split(r"[#?]", dest, 1)[0]
        if _SCHEME.match(raw):                                       # (a)
            return None
        name = unquote(raw)                                          # (b)
        if name.startswith("/") or _CONTROL.search(name):            # (c)
            return None
        if not name.endswith(".md"):                                 # (d)
            return None
        return _resolve(self.path.parent / name)                     # (e)

    def linked_files(self):
        """Every sibling this memo links (`sibling_path`) -- from any block,
        cells included -- in first-link order, each once, the memo itself
        excluded."""
        out, seen = [], {_resolve(self.path)}
        for lx in self.lexed():
            for _, _, dest in lx.links:
                f = self.sibling_path(dest)
                if f is not None and f not in seen:
                    seen.add(f)
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
                # as a shortcut and exempted it.  Each site is recorded once
                # there: the label bracket of a failed `[text][label]` is
                # re-scanned (§6.3 Example 571) but not recorded again
                exempt = _CITE_LABEL.fullmatch(key) is not None or form == "shortcut"
                lineno = lineno_of(off)
                if exempt and (key not in orphans or lineno in orphans[key]):
                    continue
                out.append((lineno, label))

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


def _resolve(path):
    """`path.resolve()`, or `path` itself when resolving raises -- `OSError`
    (an over-long name) or, on Python 3.9-3.12, `RuntimeError` for a symlink
    loop (3.13 made that an `OSError`).  The ONE site that guards it; the
    unresolved path then reaches the population's I/O chokepoint as an
    unavailable memo."""
    try:
        return path.resolve()
    except (OSError, RuntimeError):
        return path
_CONTROL = re.compile(r"[\x00-\x1f\x7f]")
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
        queue, seen = [_resolve(pathlib.Path(main_path))], set()
        while queue:
            p = queue.pop(0)
            if p in seen:
                continue
            seen.add(p)
            # the ONE I/O chokepoint: a memo that cannot be opened, read or
            # decoded (absent, a directory, over-long, invalid UTF-8) is an
            # UNAVAILABLE linked memo -- the documented exit-2 miss, never an
            # exception out of the population
            try:
                memo = Memo(p)
            except (OSError, RuntimeError, UnicodeDecodeError) as e:
                self.misses.append((p.name, 0, "linked memo unavailable (%s) -- its population is "
                                    "unscanned" % type(e).__name__))
                continue
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
