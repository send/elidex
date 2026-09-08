#!/usr/bin/env python3
"""ONE memo -- for `plan-memo-umbrella-check.py`.

`Memo` is the Phase-1 driver of CommonMark's "Appendix: A parsing strategy"
(`_parse` / `_quote` / `_list` / `_item`: ONE forward pass over raw lines
with the open block in hand, re-entered once per container -- a block quote,
a list item -- through an EXPLICIT frame stack, `_run`, never the
interpreter's call stack), the Phase-2 resolution of its
paragraphs and cells (`Lexed.resolve` with the Phase-1 definitions), the
file I/O (`read_text(encoding="utf-8")`), and the ONE destination -> sibling
resolver (`sibling_path` / `_resolve` / `linked_files` /
`unresolved_references`).  The memo SET reachable from one memo through its
links is `plan_memo_population.py`'s `Population`, which imports this module
and never the reverse.  The row grammar, the table
schemas and the one admission site (`admit_table`), and the mask
disposition are `plan_memo_tables.py`'s, imported here and never the
reverse; the block grammar is `plan_memo_blocks.py`'s.
"""

import bisect
import pathlib
import re
from urllib.parse import unquote

from plan_memo_blocks import (
    _is_lazy, _same_list, block_end, definition_block, indentation, is_blank, item_marker,
    one_line_block, quote_content, raw_extent, raw_opener, run_end, setext_underline, starts_block,
    strip_columns, table_header_at,
)
from plan_memo_ids import is_cite_label
from plan_memo_lexer import FILE_SUFFIX, Lexed, normalize_label
from plan_memo_tables import admit_table


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
    its open block in hand, re-entered once per container over the
    container's content -- each pass a generator FRAME, a container's pass
    yielded to `_run`'s explicit stack and its result sent back, so the
    nesting depth is bounded by memory, not by the interpreter's recursion
    limit: 1,000 nested quotes or items parse, as commonmark.js 0.31.2
    parses them, measured; PR #510 R16): the raw extents -- indented code (§4.4), fenced
    blocks (§4.5) and HTML blocks (§4.6), one opener rule and one extent
    rule, consumed in place -- the two containers, block quotes (§5.1:
    marker lines stripped, lazy continuation lines gathered, the same pass
    over the content) and list items (§5.2: the content indentation the
    marker sets stripped, the lines short of it gathered as lazy
    continuation candidates by the SAME mechanism, the same pass over the
    content; sibling items of one type grouped into a §5.3 list, loose or
    tight), GFM tables (§4.10, ending at a blank line or any block start),
    runs and their reference definitions (§4.7 -- a block of its own,
    recognised only at a block start; a definition-shaped line INSIDE a
    paragraph is an orphan, recorded in `orphans` with its destination),
    paragraphs.  Phase 1 STATES its result as a block sequence (`sequence`:
    `[kind, first raw line, last raw line]` in document order, a container
    before its content; a list entry carries its first marker and its
    tightness, `["list", first, last, marker, tight]`) -- the claim the
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
        # normalised label -> [(1-based line number, column of the label's `[`,
        # destination)]: the orphan's OWN bracket, by exact position -- a line
        # number alone exempted a second shortcut of the same label on the
        # definition's line (`[sib]: child.md "[sib]"`: the title's `[sib]` is
        # a shortcut whose label has an orphan definition, the documented
        # miss; PR #510 R14) -- and its DESTINATION, which decides whether a
        # shortcut naming the label lost a memo at all (PR #510 R15: an orphan
        # `[x]: #section` or `[x]: https://…` names no sibling on disk, so
        # `[x]` later is prose, not the population miss; `sibling_path` is the
        # one resolver that says so, in `unresolved_references`)
        self.orphans = {}
        # [(lineno, raw text, reading)] every line of a raw extent that is not
        # a fence, and every inline raw HTML span (§6.6, keyed on the line it
        # starts on -- a comment may span lines), in ONE list -- the
        # LEX-UNSUPPORTED? seed's population, under one seed rule; `reading`
        # says which grammar hides the text: "html" (an HTML block, §4.6),
        # "indented" (indented code, §4.4) or "inline" (a §6.6 span inside a
        # paragraph or a cell; PR #510 R17 -- the same disposition as the
        # block form, "raw, seeded": an id inside an attribute is no site).
        # A fence is not recorded: it is the author's explicit code marker,
        # where an indented `| row |` is the I-C silent-skip class.
        self.raw = []
        self.paragraphs, _, _, _ = self._run(self._parse(self.lines, list(range(1, len(self.lines) + 1)), None))
        for lx in self.lexed():
            lx.resolve(self.defs)
        self.raw.extend((lineno, text, "inline") for lineno, text in self._inline_raw())

    @staticmethod
    def _run(frame):
        """Drive the Phase-1 passes off an EXPLICIT stack.  `_parse`, `_quote`,
        `_list` and `_item` are generators: where one re-enters the pass over
        a container's content it YIELDS the inner frame and receives its
        return value back (`x = yield self._quote(...)` reads as the call it
        replaces).  The stack here is a list of suspended frames -- a nested
        container pushes one, a finished frame pops and hands its value to
        the frame below -- so the depth of container nesting a memo may have
        is bounded by memory, never by `sys.getrecursionlimit()`: with the
        passes as plain calls, ~500 nested `>` markers raised
        `RecursionError` inside `_parse` (PR #510 R16), which the population's
        chokepoint then mis-read as an unavailable memo.  commonmark.js
        0.31.2 parses 1,000 nested quotes and 1,000 nested items (measured);
        so does this.  Returns the outermost frame's value."""
        frames, value = [frame], None
        while frames:
            try:
                child = frames[-1].send(value)
            except StopIteration as done:
                value = done.value
                frames.pop()
            else:
                frames.append(child)
                value = None
        return value

    def _parse(self, lines, linenos, lazy):
        """Phase 1 as ONE forward pass over one container's content `lines`
        (raw line numbers `linenos`; `lazy[i]`, None at the document level,
        marks a block quote's line that carried no marker or a list item's
        line short of its content indentation -- a §5.1 / §5.2 lazy
        continuation candidate, ONE mechanism), the block STATE in hand --
        which of a raw extent, a table or a run is open -- and every
        boundary decided by the ONE predicate `block_end` (a line inside a
        run was already found not to be one, with a paragraph open; a line
        outside a run is asked with none open, so a type-7 HTML opener or an
        indented line after a table, a one-line block or a setext heading
        opens a raw extent there and stays paragraph text after a run line).
        Returns (paragraphs, lines consumed, ends with a blank line, loose):
        the pass STOPS at a lazy candidate where no paragraph is open --
        such a line is content only as paragraph continuation text, so the
        container ends there (`> # h\\nlazy` is a heading in the quote and a
        paragraph after it; the spec's instance is Example 237, `> ```\\nfoo
        \\n```` -- an unclosed fence in the quote, then a paragraph and a
        fence outside it; `- a\\n\\nfoo` an item and a paragraph after it).
        The last two values are §5.3's looseness facts, stated here because
        only this pass sees which blank lines it consumed: `gap` -- what it
        consumed last was a blank line (the blank branch below: a blank
        inside a raw extent is the extent's, and one before any block --
        an item's blank first line, Example 278 -- opens no gap) or a list
        whose last item ends with one -- and `loose`, set when a block opens
        across a gap (§5.3: an item that "directly contain[s] two block-level
        elements with a blank line between them"; a nested list's own gaps
        are its own, a quote's are discarded, commonmark.js's `lastLineBlank`
        exceptions).  With a RUN open the ONE boundary a lazy line can be is
        a GFM table header whose delimiter row carries the marker -- the
        only `block_end` arm that needs the next line, which `_quote`'s
        gather could not see -- and that line is CONTENT: cmark-gfm's table
        extension reads the header out of the open paragraph's last line,
        and a reference definition is paragraph text until the paragraph
        ends (§4.7 is parsed out of it at finalisation), so a definition
        keeps the run open exactly as paragraph text does.  ONE rule (PR
        #510 R17): a lazy candidate `table_header_at` accepts is a table
        header iff a run is open -- `cur` holds paragraph lines, or the
        line before was the last line of a definition consumed at this
        level (`def_end`) -- and is handed to `admit_table` instead of
        ending the quote.  cmark-gfm, MEASURED (`gh api -X POST /markdown
        -f mode=gfm -f text=…`, 2026-09-08) on `<X>\\n| h |\\n> |---|\\n> | 1
        |` for each X, the lazy `| h |` between:
          * `> a` (a paragraph): quote[p, table] -- design re-gate 3;
          * `> [a]: /u` (a definition): quote[table]; `> [a]: /u\\n> [b]:
            /v` (two): quote[table]; `- [a]: /u` with `  |---|` (an item):
            item[table] -- the R17 fix, and the reviewer's shape with a
            schema table (`> [a]: /u\\n| # | Slice | … |\\n> |---|…`) is
            that table inside the quote;
          * `> a\\n>` (a blank quote line), `> ```\\n> x\\n> ```` (a fence),
            `> # h` (a heading), `>     code` (indented code), `> <div>` (an
            HTML block), `> ***` (a thematic break), `> | a |\\n> |---|` (a
            table): the quote ENDS before `| h |`, which is a paragraph
            outside it, and `|---|` / `| 1 |` a second quote's paragraph --
            no run is open, so the lazy line is a boundary, as before;
          * `> [a]: /u\\n>` (a blank line after the definition): the same
            quote end -- the blank closes the run;
          * a lazy DELIMITER (`> [a]: /u\\n| h |\\n|---|`) or a lazy BODY row
            (`…> |---|\\n| 1 |`): paragraph text / the quote's end,
            unchanged (`table_header_at` / `admit_table`'s lazy arms).
        ⚠ One divergence stays, stated: cmark-gfm PRINTS the definition as
        a paragraph and registers nothing (`[a]: /u\\n| h |\\n|---|\\n\\n[a]`
        renders `[a]` literally, at the top level and in the quote alike --
        its table extension re-creates the residual paragraph without the
        §4.7 reference parse); here the definition registers (§4.7: a valid
        definition at a block start is a block; a sibling it names is
        walked -- the polarity that loses no memo) and the header forms
        the table.  The lazy rule models WHERE the header lands, not the
        fate of the definition.  Each line is classified once, in order:

          * a raw-extent opener (`raw_opener`; indented code, fences and HTML
            blocks share the one rule): its lines to `raw_extent` are never
            inline-parsed and are consumed here; every line of an HTML block
            (§4.6) or an indented code block (§4.4) is recorded in `raw`
            with its READING for the LEX-UNSUPPORTED? seed (one seed rule
            for raw lines: a line holding a `|` or a declared id is
            printed, never assumed -- an indented schema row after a
            table's rows is raw under cmark-gfm too, and the I-C
            silent-skip class without the seed).  ⚠ Until PR #510 R15 a
            list item was read LEXED-FLAT -- its marker line headed a
            paragraph and nothing tracked its content indentation -- so
            the item's NEXT paragraph, indented to that content (Example
            108 `- foo\\n\\n    bar`), was consumed here as indented code:
            a link there was never found and the memo it named never
            walked, rc 0 (the reviewer's input `- item\\n\\n
            [child](child.md)`); an item-open bit only seeded it.  The
            item is a container now, and that line is its paragraph;
          * a blank line ends the paragraph (and opens a §5.3 gap where a
            block precedes it at this level);
          * a `>` line opens a block quote: `_quote`, the same pass over
            its content;
          * a setext underline after paragraph text (§4.3): the paragraph
            is the heading, at the level `setext_underline` reads (the one
            reading of the underline), and the underline closes it, content
            of nothing; with no paragraph open the line is not an underline
            at all (`block_end`'s `para_open` arm) -- `---` is a thematic
            break and `===` paragraph text.  Read BEFORE a list marker is,
            in commonmark.js's block-start order: `foo\\n-` is a heading,
            `- a\\n  -` an item holding one; a lazy `---` never reaches
            here (`block_end`: a thematic break there, Example 94);
          * a list-item line (§5.2) opens a LIST: `_list` groups the items
            of one type, each an `_item` -- the same pass over the item's
            content -- and reports whether the list ends with a blank line
            (the gap the next block at this level opens across);
          * a GFM table header off a block start: `admit_table`, the one
            admission site;
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

        Linear: one lookahead per run, one parse per line.  A generator
        frame under `_run` (the container branches `yield` the inner pass and
        receive its result); `return` hands the tuple to the frame below."""
        out, cur, i, n = [], [], 0, len(lines)
        run_text, run_off, defs_at = {}, {}, {}
        sequence, seq0 = self.sequence, len(self.sequence)
        gap, loose = False, False      # §5.3 (the docstring): a gap is open / a block opened across one
        def_end = -1                   # the content index just past the last definition consumed here

        def flush(kind="p", last=None):
            if cur:
                sequence.append([kind, cur[0][0], cur[-1][0] if last is None else last])
                out.append(Paragraph(list(cur)))
                cur.clear()

        def open_block():
            # a block starts at this level: the paragraph before it is
            # closed, and a gap before it makes the containing item loose
            nonlocal gap, loose
            flush()
            loose = loose or gap
            gap = False

        def blank_line():
            # a blank line closes the paragraph and, after a block at this
            # level (`seq0`: anything stated since this pass began is inside
            # one), opens a gap
            nonlocal gap
            flush()
            gap = gap or len(sequence) > seq0

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
                # §5.1: a lazy candidate is content only with a RUN open
                # (paragraph text, or the definition just consumed), and
                # then only as the table header cmark-gfm reads out of it
                if lazy is not None and lazy[i] and not ((cur or i == def_end) and table_header_at(lines, i, lazy)):
                    break       # no run is open, so the container ends here
                # outside a run no paragraph is open: the block state is the
                # run map itself, not a look at the previous line
                opener = raw_opener(line, False)
                if opener is not None:
                    open_block()
                    end = raw_extent(lines, i, opener, lazy)
                    if opener[0] != "fence":
                        self.raw.extend((linenos[k], lines[k], opener[0]) for k in range(i, end))
                    sequence.append([opener[0], linenos[i], linenos[end - 1]])
                    i = end
                    continue
                if is_blank(line):
                    blank_line()
                    i += 1
                    continue
                if quote_content(line) is not None:
                    open_block()
                    i += yield self._quote(lines, linenos, i, out)
                    continue
                heading = setext_underline(line) if cur else None
                if heading is not None:
                    # §4.3: the paragraph is a heading; the underline closes
                    # it and is not content
                    flush(heading, linenos[i])
                    i += 1
                    continue
                if item_marker(line) is not None:
                    open_block()
                    used, gap = yield self._list(lines, linenos, i, out, lazy)
                    i += used
                    continue
                if not starts_block(line) and table_header_at(lines, i, lazy):
                    open_block()
                    t, end = admit_table(self, lines, linenos, i, lazy)
                    self.tables.append(t)
                    sequence.append(["table", linenos[i], linenos[end - 1]])
                    i = end
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
                open_block()
                i += k
                def_end = i     # the run stays open through the definition (the lazy-header rule)
                continue
            if d is not None:
                # a valid definition that cannot take effect: the orphan, keyed
                # on its label's `[` (§4.7: after <=3 columns of indentation),
                # its destination kept
                self.orphans.setdefault(normalize_label(d[0]), []).append((linenos[i], indentation(line)[1], d[1]))
            elif new_run:
                open_block()        # a run start begins a paragraph
            cur.append((linenos[i], line))
            kind = one_line_block(line)
            if kind:
                flush(kind)
            i += 1
        flush()
        return out, i, gap, loose

    def _list(self, lines, linenos, i, out, lazy):
        """The list (§5.3) whose first item opens at content line `i` -> (the
        number of lines it spans, whether its last item ends with a blank
        line -- the gap the caller's next block opens across).  "A list is a
        sequence of one or more list items of the same type" (`_same_list`:
        the same bullet character, or the same ordered delimiter -- `- a\\n2.
        b` and `1. a\\n1) b` are two lists, Examples 301-302), each an
        `_item`; between two items the blank lines belong to the list.
        Its sequence entry, before its items, states the first marker (the
        `<ul>` / `<ol start>` the html renders) and its TIGHTNESS: §5.3, "A
        list is loose if any of its constituent list items are separated by
        blank lines, or if any of its constituent list items directly
        contain two block-level elements with a blank line between them.
        Otherwise a list is tight" -- so loose when a non-final item ends
        with a blank line (`_parse`'s gap, recursing into a nested list's
        last item as commonmark.js's `endsWithBlankLine` does), when blank
        lines stand between two items at this level (`- a\\n-\\n\\n- b`: the
        empty item took none of them, Example 280's rule, yet the list is
        loose -- measured), or when an item's own pass opened a block
        across a gap; every shape checked against commonmark.js 0.31.2 and
        the spec's `Lists` examples."""
        n = len(lines)
        marker = item_marker(lines[i])[0]
        entry = ["list", linenos[i], None, marker, None]
        self.sequence.append(entry)
        loose, j = False, i
        while True:
            used, ends_blank, inner_loose = yield self._item(lines, linenos, j, out, lazy)
            loose = loose or inner_loose
            j += used
            k = j
            while k < n and is_blank(lines[k]):
                k += 1
            nxt = item_marker(lines[k]) if k < n else None
            if nxt is None or not _same_list(marker, nxt[0]):
                break
            loose = loose or ends_blank or k > j
            j = k
        entry[2], entry[4] = linenos[j - 1], not loose
        return j - i, ends_blank

    def _item(self, lines, linenos, i, out, lazy):
        """The list item opening at content line `i` (a marker line, §5.2)
        -> (the number of lines it spans, whether it ends with a blank
        line, whether it is loose inside); its paragraphs are appended to
        `out`, everything else to this memo's like every other block's.
        The content (`item_marker`: what follows the marker and N spaces on
        the first line; on each later line what follows the item's content
        indentation, `strip_columns` -- a blank line stays, blank), the
        lines short of that indentation gathered as LAZY CONTINUATION
        CANDIDATES exactly as `_quote` gathers its marker-less lines (§5.2
        rule 5 restates §5.1's: "paragraph continuation text" only), and
        the enclosing container's own candidates kept whole and lazy here
        too (`> - a\\n    ---`: the raw line is the quote's candidate -- the
        quote is unmatched, so nothing inside it matches either, and the
        text is the item paragraph's, not a thematic break at four columns
        stripped to zero; measured), then the SAME `_parse` over the
        content, which stops at a candidate where no paragraph is open
        (`- a\\n\\nfoo`: the item is `a` and its blank line; `foo` is the
        paragraph after the list).  "A list item can begin with at most one
        blank line" (§5.2 rule 3, Example 280 `-\\n\\n  foo`): an item whose
        first line is blank ends before a blank next line, and is the empty
        item.  Linear: each line is gathered once per enclosing container,
        as for quotes."""
        n = len(lines)
        _, offset, first = item_marker(lines[i])
        content, nos, inner_lazy, j = [first], [linenos[i]], [False], i + 1
        if not (is_blank(first) and j < n and is_blank(lines[j])):
            while j < n:
                line = lines[j]
                if _is_lazy(lazy, j) or (not is_blank(line) and indentation(line)[0] < offset):
                    content.append(line)
                    nos.append(linenos[j])
                    inner_lazy.append(True)
                    if block_end(content, len(content) - 1, True, inner_lazy):
                        content.pop()
                        nos.pop()
                        inner_lazy.pop()
                        break
                else:
                    content.append(strip_columns(line, min(offset, indentation(line)[0])))
                    nos.append(linenos[j])
                    inner_lazy.append(False)
                j += 1
        entry = ["item", linenos[i], None]
        self.sequence.append(entry)
        paragraphs, used, ends_blank, loose = yield self._parse(content, nos, inner_lazy)
        out.extend(paragraphs)
        entry[2] = nos[used - 1]
        return used, ends_blank, loose

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
        # a quote's gaps are its own: a blank content line of a quote inside
        # an item (`- > a\n  >\n- b`) loosens no list (measured)
        paragraphs, used, _, _ = yield self._parse(content, nos, inner_lazy)
        out.extend(paragraphs)
        entry[2] = nos[used - 1]
        return used

    @property
    def key(self):
        """The memo's identity for every per-memo map (mention identity, the
        seeds' row maps): the RESOLVED path.  Two memos in different
        directories may share a basename, and a map keyed on the basename
        aliases their rows; the DISPLAY name every printer uses is the
        population's `display` (the path relative to the root memo's
        directory), never `path.name` (PR #510 R19)."""
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

    def _inline_raw(self):
        """(lineno, text) of every §6.6 raw HTML span Phase 2 found (`Lexed.html`),
        in the order `lexed` yields the blocks -- a cell's span at its row's
        line, a paragraph's at the line the span STARTS on (`Paragraph.locate`;
        a comment may cross a line ending).  Read once, after `resolve`, into
        `raw` for the LEX-UNSUPPORTED? seed."""
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    for a, b in cell.lexed.html:
                        yield row.lineno, cell.lexed.text[a:b]
        for p in self.paragraphs:
            for a, b in p.lexed.html:
                yield p.locate(a)[0], p.lexed.text[a:b]

    def sibling_path(self, dest):
        """The ONE destination -> sibling mapping: the memo on disk a link
        destination names, or None when it names none.  POLICY (CommonMark
        §6.3 / GFM say nothing about siblings on disk): a sibling is a
        RELATIVE `.md` path beside this memo.  Stages, in spec order:
          (a) the scheme test on the RAW path component -- WHATWG URL §4.4
              "URL parsing", the basic URL parser's *scheme start state*
              (https://url.spec.whatwg.org/#scheme-start-state, step 1: "If
              c is an ASCII alpha, append c, lowercased, to buffer, and set
              state to scheme state") and *scheme state*
              (https://url.spec.whatwg.org/#scheme-state, step 1: "If c is
              an ASCII alphanumeric, U+002B (+), U+002D (-), or U+002E (.),
              append c, lowercased, to buffer"; step 2: "Otherwise, if c is
              U+003A (:)" -- the scheme is set) read the input's code points
              AS WRITTEN; `%` is in neither class, so at `%` the parser
              leaves for the *no scheme state* and `notes%3Achild.md` has no
              scheme (`_SCHEME` is that class and that terminator) -- it is
              the local file `notes:child.md`.  Percent-decoding is no step
              of the parser at all: the *path state*
              (https://url.spec.whatwg.org/#path-state) percent-ENCODES and
              keeps `%xx` as written;
          (b) percent-decode (`slice%20sib.md` is `slice sib.md`, as
              `<slice sib.md>` is) -- WHATWG URL §1.3 "Percent-encoded
              bytes", *percent-decode* on a string
              (https://url.spec.whatwg.org/#string-percent-decode: "Let bytes
              be the UTF-8 encoding of input. Return the percent-decoding of
              bytes"), the operation a consumer applies to a parsed path;
              `urllib.parse.unquote` is that operation;
          (c) the DECODED name must be RELATIVE on every platform, and hold
              no C0 control / DEL (`child%00.md` would make `resolve()`
              raise).  ONE platform-independent reading of the name:
              Windows path syntax (`pathlib.PureWindowsPath`), the superset
              -- `/` and `\\` both separate, and a drive letter (`C:`), a UNC
              prefix (`\\\\server\\share`) or a root (`/`, `\\`) ANCHORS.  It
              is the URL standard's own reading of a special-scheme path
              (`file` is a special scheme): *path state*
              (https://url.spec.whatwg.org/#path-state) step 1 ends a
              segment at "U+002F (/)" or, "url is special and c is U+005C
              (\\)", at a backslash (with an invalid-reverse-solidus
              validation error), and step 1.4.1's Windows drive letter rule
              is, in the spec's words, "a (platform-independent) Windows
              drive letter quirk".  So `PureWindowsPath(name).anchor` must
              be empty: `/x`, `//host/x` (a site URL joined to the memo's
              directory would probe the host's filesystem root), `\\x`,
              `C:\\temp\\x`, `\\\\server\\share\\x` and the drive-relative
              `C:x` (raw `C:x.md` is already a URL of scheme `c` at (a);
              percent-encoded `C%3Ax.md` decodes to a drive anchor here --
              so does the one-letter `n%3Ax.md`, where the multi-letter
              `notes%3Ax.md` of (a) is a file name) are all rejected.  ⚠
              Until PR #510 R19 this stage rejected a leading `/` only, so
              `C%3A%5Ctemp%5Cchild.md` (`C:\\temp\\child.md`) and
              `%5Cchild.md` (`\\child.md`) passed, and on Windows `parent /
              name` discarded the memo's directory.  An empty anchor is not
              enough: a RELATIVE name whose component is a DOS device or
              ends in a dot or a space is no file beside the memo either
              (`_is_reserved_component`, per PART, so `dir/NUL.md` and
              `NUL/child.md` go too).  ⚠ Until PR #510 R22 they passed,
              and both halves fail SILENTLY where an anchor does not: on
              Windows reading `NUL.md` SUCCEEDS and yields an empty
              stream, so the population counted an empty linked memo and
              could exit 0 having omitted the sibling the author linked,
              and `dir /child.md` reads a different directory there than
              here.  The cost, named: a POSIX memo genuinely called
              `NUL.md` is now not a sibling either -- dropped without a
              report, exactly as `/abs/x.md` and `C:\\x.md` already are,
              which is this stage's standing polarity;
          (d) the `.md` suffix -- the lexer's `FILE_SUFFIX`, the ONE
              spelling of "is a file name" (the lexer's bare file token
              reads the same constant over prose); the stem is unconstrained
              on both sides, so `.md` alone is a sibling file (PR #510 R20);
          (e) the name's PARTS joined beside the memo -- the same Windows
              syntax, so `sub%5Cchild.md` is the sibling `sub/child.md` on
              POSIX as on Windows (never the POSIX file named `sub\\child.md`:
              a backslash is a separator everywhere, never a name character)
              -- then `resolve()` (`_resolve`); an `OSError` or -- on Python
              3.9-3.12, for a symlink loop -- a `RuntimeError` there makes
              the sibling UNAVAILABLE: the joined, unresolved path is
              returned and the population's one I/O chokepoint reports it as
              an unavailable linked memo (exit 2), never a crash, never a
              silent drop.
        """
        raw = re.split(r"[#?]", dest, 1)[0]
        if _SCHEME.match(raw):                                       # (a)
            return None
        name = unquote(raw)                                          # (b)
        p = pathlib.PureWindowsPath(name)
        if (_CONTROL.search(name) or p.anchor                        # (c)
                or any(_is_reserved_component(s) for s in p.parts)):
            return None
        if not name.endswith(FILE_SUFFIX):                           # (d)
            return None
        return _resolve(self.path.parent.joinpath(*p.parts))         # (e)

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
        could not read (a §4.7 definition cannot interrupt a paragraph) AND
        whose destination names a sibling memo -- the sites where a
        population the author meant to link is lost.  An orphan whose
        destination is `#section` or an external URL lost no memo (a
        resolved reference to such a target expands the population by
        nothing either), so a shortcut naming it is prose: `sibling_path`,
        the ONE resolver, decides -- there is no second spelling of "is a
        sibling" here (PR #510 R15)."""
        out = []
        # label -> the orphans' own brackets, for the labels whose orphan
        # names a sibling on disk; a label with no such orphan is absent
        orphans = {}
        for key, entries in self.orphans.items():
            if any(self.sibling_path(dest) is not None for _, _, dest in entries):
                orphans[key] = {(lineno, col) for lineno, col, _ in entries}

        def walk(lx, site_of):
            for off, label, form, is_image in lx.unresolved:
                if is_image:
                    continue     # §6.4: literal image syntax; an image never links a memo
                key = normalize_label(label)
                # a `[C19]`-style citation id (`is_cite_label`: the grammar's
                # one predicate, the lexer's mask reads the same) is never a
                # memo reference, in ANY form -- shortcut, full (`[C19][C20]`
                # adjacent citations) or collapsed (`[C19][]`); a plain
                # shortcut of any other label is prose too.  An orphan
                # definition of the label (one the grammar could not read) is
                # still reported, citation or not, except at the definition's
                # own bracket -- by exact (line, column), never by line.
                # the FORM comes from the lexer's one bracket parse (escapes
                # honoured); a raw re-walk here once read `[foo\]][missing]`
                # as a shortcut and exempted it.  Each site is recorded once
                # there: the label bracket of a failed `[text][label]` is
                # re-scanned (§6.3 Example 571) but not recorded again
                exempt = is_cite_label(key) or form == "shortcut"
                site = site_of(off)
                if exempt and (key not in orphans or site in orphans[key]):
                    continue
                out.append((site[0], label))

        for p in self.paragraphs:
            walk(p.lexed, lambda off, p=p: p.locate(off)[0::2])
        for t in self.tables:
            for row in [t.header] + t.rows:
                for cell in row.cells:
                    walk(cell.lexed, lambda off, row=row, cell=cell: (row.lineno, cell.raw(off)))
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

# The DOS DEVICE names -- Microsoft, "Naming Files, Paths, and Namespaces"
# (https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file):
# "Do not use the following reserved names for the name of a file", and the
# rule holds "regardless of the file extension" and in any directory.  The
# set is CPython `ntpath._reserved_names`' reading of that paragraph, spelled
# here as data rather than called: `PureWindowsPath.is_reserved()` is
# deprecated in 3.13 and removed in 3.15 (it raises a DeprecationWarning on
# the 3.14 this runs on), and `os.path.isreserved` exists only on 3.13+ and
# only in `ntpath` -- on POSIX `os.path` IS `posixpath` and has no such
# function, so neither is available to a checker that must decide this the
# same way on every platform.
_DEVICE_NAMES = frozenset(
    {"AUX", "CON", "CONIN$", "CONOUT$", "NUL", "PRN"}
    | {p + n for p in ("COM", "LPT") for n in "123456789¹²³"})


def _is_reserved_component(part):
    """Whether one already-parsed `PureWindowsPath` component is a name
    Windows does NOT resolve to a file of that name beside its parent --
    pure string logic, so it is decided identically on POSIX and testable
    there.  Two halves, CPython `ntpath._isreservedname`'s reading:

      * a component ending in `.` or ` ` (the components `.` and `..`
        excepted): Windows strips the trailing run, so `dir /child.md`
        reads `dir\\child.md` there and a different directory here;
      * otherwise the stem before the first `.`, its trailing spaces
        stripped, upper-cased, is a DOS device (`_DEVICE_NAMES`): `NUL.md`,
        `dir/NUL.md`, `nul.md` and `prn .md` all resolve to a device.

    WHICH STAGE OWNS WHAT, so the reading is spelled once.
    `_isreservedname`'s CHARACTER half -- `*?"<>/\\:|` and the ASCII
    controls -- is NOT repeated here: the controls (and DEL, wider)
    are `sibling_path` stage (c)'s `_CONTROL`, and `/` and `\\` are
    SEPARATORS to the `PureWindowsPath` parse that produced these parts, so
    neither can stand inside a component.  The rest (`*?"<>:|`) is
    deliberately left to stage (e): they are ordinary POSIX name characters,
    the population resolves siblings on the RUNNING platform, and on Windows
    a name holding one raises `OSError` there -- reported as an unavailable
    linked memo, exit 2.  That is the discriminator this predicate is drawn
    on: a device name is the class that opens SUCCESSFULLY and returns an
    empty stream, so the population would count a memo it never scanned and
    exit 0 -- the "clean exit for content that could not be scanned" the
    plan's §1 forbids.  Rejecting `*?"<>:|` as well would also contradict
    the decided reading of `notes%3Achild.md` as the local file
    `notes:child.md` (PR #510 R8)."""
    if part[-1:] in (".", " "):
        return part not in (".", "..")
    return part.partition(".")[0].rstrip(" ").upper() in _DEVICE_NAMES
