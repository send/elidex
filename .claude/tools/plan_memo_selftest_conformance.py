#!/usr/bin/env python3
"""The CommonMark 0.31.2 spec's OWN examples as the falsifier of Phase 1
(`plan_memo_blocks.py` / `plan_memo_memo.Memo._parse`) -- the control behind every row of
the plan's §3.0 table.

WHY.  Two review rounds in a row were spec-table transcription errors, in
both directions (a type-6 tag name dropped; a `<!` rule cited from 0.29), and
a hand-written control per clause is a transcription of the same reading.
The spec ships its examples machine-readable (`spec.json`: markdown, html,
example number, section); the ones for the block sections Phase 1 lexes are
vendored in `commonmark-0.31.2-block-examples.json` (295 examples over
`Tabs` §2.2, `Thematic breaks` §4.1, `ATX headings` §4.2, `Setext headings`
§4.3, `Indented code blocks` §4.4, `Fenced code blocks` §4.5, `HTML blocks`
§4.6, `Link reference definitions` §4.7, `Paragraphs` §4.8, `Blank lines`
§4.9, `Block quotes` §5.1 (Examples 228-252, since design re-gate 3) and --
since PR #510 R15, when the list item became a container -- `List items`
§5.2 (Examples 253-300) and `Lists` §5.3 (Examples 301-326), each
container's own falsifier), and this control runs every one through `Memo`
and checks the html for what Phase 1 CLAIMED -- never a rendering.

WHAT IS CHECKED (`align`).  Phase 1 STATES its output as a block sequence
(`Memo.sequence`: `[kind, first line, last line]` in document order, a
container before its content -- the one place the pass says what it read,
so nothing is re-derived here from dropped lines).  The sequence is CONSUMED
against the expected html in order: a block quote `<blockquote>` … its
content … `</blockquote>`, a list `<ul>` / `<ol>` (`<ol start="N">` off the
first marker the entry states) … its items … `</ul>`, an item `<li>` … its
content … `</li>`, a raw HTML extent VERBATIM (an HTML block renders
as written; its content lines are the `"html"` entries of `Memo.raw`), an indented or fenced
extent `<pre><code` … `</code></pre>`, a thematic break `<hr />`, a heading
`<hN>` … `</hN>` at its `#` count or 1 / 2 for a `=` / `-` underline, a
paragraph `<p>` … `</p>` -- or, as a direct child of a TIGHT list's item,
its bare inline text up to the `</li>` or the next block's tag, the
TIGHTNESS being Phase 1's claim too (§5.3, the list entry's last field; a
loose list's items wrap every paragraph, a tight list's none -- the two
renderings differ, so the claim is checked, not skipped) -- a definition
nothing; and the html must be exhausted at the end.  Inline content
(Phase 2) is skipped over, so this is a block-structure oracle only, and
position is what it checks: a paragraph read where the spec has a heading,
a definition swallowed as prose, a fence not closed, an HTML block not
opened, a quote's lazy line read outside it, a code chunk split at a blank
line, an item's second paragraph read as indented code, a loose list
claimed tight, all break the alignment.

WHAT IS EXCLUDED, by predicate over Phase 1's own output (printed per run;
the plan's §3.0 dispositions decide, not a hand list): a GFM table Phase 1
admitted (local policy over pure CommonMark; none of the vendored examples
holds a `|` where a table could open, so this arm is empty by construction).
Nothing else: since R15 every block type of the spec's closed list is
modelled, and the §5.2 exclusion ("a paragraph headed by a list-marker
line: LEXED-FLAT", 13 examples at R14) is gone with the flat reading.  An
aligned example is PASS; an example neither excluded nor aligned is a FAIL,
and a FAIL here is a defect in Phase 1 or a disposition the plan does not
state -- never a reason to rewrite the property.
"""

import json
import pathlib
import re
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
EXAMPLES = HERE / "commonmark-0.31.2-block-examples.json"

# where a tight item's bare paragraph text ends: the item's close, or the
# next block's tag on its own line (the renderer starts every block tag on a
# fresh line, and puts nothing else at a line start inside inline text)
_TIGHT_END = re.compile(r"</li>|\n<(?:ul|ol|pre|blockquote|h[1-6]|hr)\b")


def excluded(memo):
    """The disposition that keeps `memo` out of the alignment, or None."""
    if any(b[0] == "table" for b in memo.sequence):
        return "a GFM table (local policy over pure CommonMark)"
    return None


def align(memo, html):
    """Consume `html` block by block against Phase 1's sequence; None when it
    aligns, else the first disagreement."""
    pos, k = 0, 0
    sequence, raw_html = memo.sequence, {l: t for l, t, reading in memo.raw if reading == "html"}

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

    def bare_paragraph():
        # a tight item's paragraph: inline text, no `<p>`, up to `_TIGHT_END`
        # -- and a `<p>` here is the html of a LOOSE list, so a tight claim
        # is refused at it rather than read past it (a search to `</li>`
        # alone let every "loose" mutant survive: the claim could not go red)
        nonlocal pos
        if html.startswith("<p>", pos):
            return "a TIGHT list claimed where the html wraps the item's paragraph: %r" % html[pos:pos + 40]
        m = _TIGHT_END.search(html, pos)
        if m is None:
            return "a tight item's paragraph at html[%d:] has no end (%r)" % (pos, html[pos:pos + 40])
        pos = m.start() + (1 if html[m.start()] == "\n" else 0)
        return None

    def consume(last, tight=False):
        """The blocks whose first line is at most `last` -- a container's
        content, or the whole document; `tight` while they are the direct
        children of a tight list's item."""
        nonlocal pos, k
        while k < len(sequence) and sequence[k][1] <= last:
            kind, lo, hi = sequence[k][:3]
            entry = sequence[k]
            k += 1
            if kind == "quote":
                err = expect("<blockquote>\n") or consume(hi) or expect("</blockquote>\n")
            elif kind == "list":
                marker, is_tight = entry[3], entry[4]
                if marker in "-+*":
                    tags = ("<ul>\n", "</ul>\n")
                else:
                    start = int(marker[:-1])
                    tags = ("<ol>\n" if start == 1 else '<ol start="%d">\n' % start, "</ol>\n")
                err = expect(tags[0]) or consume(hi, is_tight) or expect(tags[1])
            elif kind == "item":
                err = expect("<li>")
                if not err:
                    if html.startswith("\n", pos):     # a block, not bare text, follows
                        pos += 1
                    err = consume(hi, tight) or expect("</li>\n")
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
                err = bare_paragraph() if tight else expect("<p>", "</p>\n")
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


def run(M):
    """(ok, detail): every vendored example through `M.Memo` (the freshly
    loaded `plan_memo_memo`), aligned or excluded; a crash on any example is
    a FAIL of that example."""
    data = json.loads(EXAMPLES.read_text(encoding="utf-8"))
    passed, fails, skips = 0, [], {}
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "example.md"
        for ex in data["examples"]:
            no = ex["example"]
            p.write_text(ex["markdown"], encoding="utf-8")
            try:
                memo = M.Memo(p)
                why = excluded(memo)
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
