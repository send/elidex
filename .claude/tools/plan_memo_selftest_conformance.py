#!/usr/bin/env python3
"""The CommonMark 0.31.2 spec's OWN examples as the falsifier of Phase 1
(`plan_memo_blocks.py` / `plan_memo_memo.Memo._parse`) -- the control behind every row of
the plan's §3.0 table.

WHY.  Two review rounds in a row were spec-table transcription errors, in
both directions (a type-6 tag name dropped; a `<!` rule cited from 0.29), and
a hand-written control per clause is a transcription of the same reading.
The spec ships its examples machine-readable (`spec.json`: markdown, html,
example number, section); the ones for the block sections Phase 1 lexes are
vendored in `commonmark-0.31.2-block-examples.json` (196 examples over
`Tabs` §2.2, `Thematic breaks` §4.1, `ATX headings` §4.2, `Setext headings`
§4.3, `Indented code blocks` §4.4, `Fenced code blocks` §4.5, `HTML blocks`
§4.6, `Link reference definitions` §4.7, `Paragraphs` §4.8, `Blank lines`
§4.9 -- the §5.1 block quotes and §5.2 list items those sections' examples
hold included), and this control runs every one through `Memo` and checks
the html for what Phase 1 CLAIMED -- never a rendering.

WHAT IS CHECKED (`align`).  Phase 1 STATES its output as a block sequence
(`Memo.sequence`: `[kind, first line, last line]` in document order, a
block quote before its content -- the one place the pass says what it read,
so nothing is re-derived here from dropped lines).  The sequence is CONSUMED
against the expected html in order: a block quote `<blockquote>` … its
content … `</blockquote>`, a raw HTML extent VERBATIM (an HTML block renders
as written; its content lines are `Memo.raw_html`), an indented or fenced
extent `<pre><code` … `</code></pre>`, a thematic break `<hr />`, a heading
`<hN>` … `</hN>` at its `#` count or 1 / 2 for a `=` / `-` underline, a
paragraph `<p>` … `</p>`, a definition nothing; and the html must be
exhausted at the end.  Inline content (Phase 2) is skipped over, so this is
a block-structure oracle only, and position is what it checks: a paragraph
read where the spec has a heading, a definition swallowed as prose, a fence
not closed, an HTML block not opened, a quote's lazy line read outside it, a
code chunk split at a blank line, all break the alignment.

WHAT IS EXCLUDED, by predicate over Phase 1's own output (printed per run;
the plan's §3.0 dispositions decide, not a hand list):
  * §5.2 list items: a paragraph line that is a list-marker line
    (`list_item_line`) -- LEXED-FLAT in §3.0, the html has a `<ul>` / `<ol>`
    this lexer never claims.  ONLY that predicate: a `>` line in a paragraph
    is not excluded, so a block quote read as prose is a FAIL here, not a
    silent re-classification;
  * a GFM table Phase 1 admitted (local policy over pure CommonMark; none of
    the vendored examples holds a `|`, so this arm is empty by construction).
An aligned example is PASS; an example neither excluded nor aligned is a
FAIL, and a FAIL here is a defect in Phase 1 or a disposition the plan does
not state -- never a reason to rewrite the property.
"""

import json
import pathlib
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
EXAMPLES = HERE / "commonmark-0.31.2-block-examples.json"


def excluded(memo, B):
    """The disposition that keeps `memo` out of the alignment, or None."""
    for para in memo.paragraphs:
        for _, text in para.lines:
            # a marker line Phase 1 read as paragraph text; `- - -` is the
            # thematic break §4.1 names, not an item (Example 61's `- * * *`
            # IS an item holding one)
            if B.list_item_line(text) and not B.one_line_block(text):
                return "§5.2 list item line: LEXED-FLAT"
    if any(b[0] == "table" for b in memo.sequence):
        return "a GFM table (local policy over pure CommonMark)"
    return None


def align(memo, html):
    """Consume `html` block by block against Phase 1's sequence; None when it
    aligns, else the first disagreement."""
    pos, k = 0, 0
    sequence, raw_html = memo.sequence, dict(memo.raw_html)

    def expect(open_tag, close_tag=None):
        nonlocal pos
        if not html.startswith(open_tag, pos):
            return "%r expected at html[%d:], found %r" % (open_tag, pos, html[pos:pos + 40])
        if close_tag is None:               # a one-line block: the tag is the block
            pos += len(open_tag)
            return None
        end = html.find(close_tag, pos)
        if end < 0:
            return "%r not closed by %r" % (open_tag, close_tag)
        pos = end + len(close_tag)
        return None

    def consume(last):
        """The blocks whose first line is at most `last` -- a container's
        content, or the whole document."""
        nonlocal pos, k
        while k < len(sequence) and sequence[k][1] <= last:
            kind, lo, hi = sequence[k]
            k += 1
            if kind == "quote":
                err = expect("<blockquote>\n") or consume(hi) or expect("</blockquote>\n")
            elif kind == "html":
                want = "\n".join(raw_html[l] for l in range(lo, hi + 1)) + "\n"
                if html.startswith(want, pos):
                    pos += len(want)
                    err = None
                else:
                    err = "raw HTML lines %r not verbatim at html[%d:] (%r)" % (want, pos, html[pos:pos + 40])
            elif kind in ("fence", "indented"):
                err = expect("<pre><code", "</code></pre>\n")
            elif kind == "hr":
                err = expect("<hr />\n")
            elif kind[0] == "h":
                err = expect("<%s>" % kind, "</%s>\n" % kind)
            elif kind == "p":
                err = expect("<p>", "</p>\n")
            elif kind == "def":
                err = None
            else:
                err = "block kind %r is not modelled" % kind
            if err:
                return err
        return None

    err = consume(len(memo.lines))
    if err:
        return err
    if pos != len(html):
        return "html left over after Phase 1's last block: %r" % html[pos:pos + 60]
    return None


def run(B, M):
    """(ok, detail): every vendored example through `M.Memo` (the freshly
    loaded `plan_memo_memo`; `B` the freshly loaded `plan_memo_blocks`),
    aligned or excluded; a crash on any example is a FAIL of that example."""
    data = json.loads(EXAMPLES.read_text(encoding="utf-8"))
    passed, fails, skips = 0, [], {}
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "example.md"
        for ex in data["examples"]:
            no = ex["example"]
            p.write_text(ex["markdown"], encoding="utf-8")
            try:
                memo = M.Memo(p)
                why = excluded(memo, B)
                if why is not None:
                    skips.setdefault(why, []).append(no)
                    continue
                err = align(memo, ex["html"])
            except Exception as e:      # noqa: BLE001 -- a crash is the defect
                err = "crash %s: %s" % (type(e).__name__, str(e)[:60])
            if err is None:
                passed += 1
            else:
                fails.append((no, ex["section"], err))
    n_skip = sum(len(v) for v in skips.values())
    lines = ["%d examples: %d aligned, %d excluded, %d FAIL" % (len(data["examples"]), passed, n_skip, len(fails))]
    for why, nos in sorted(skips.items()):
        lines.append("  excluded (%s): %s" % (why, " ".join(str(x) for x in nos)))
    for no, section, err in fails:
        lines.append("  FAIL Example %d (%s): %s" % (no, section, err))
    return not fails and passed > 0, "\n".join(lines)
