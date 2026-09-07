#!/usr/bin/env python3
"""The CommonMark 0.31.2 spec's OWN examples as the falsifier of Phase 1
(`plan_memo_blocks.py` / `Memo._phase1`) -- the control behind every LEXED
row of the plan's §3.0 table.

WHY.  Two review rounds in a row were spec-table transcription errors, in
both directions (a type-6 tag name dropped; a `<!` rule cited from 0.29), and
a hand-written control per clause is a transcription of the same reading.
The spec ships its examples machine-readable (`spec.json`: markdown, html,
example number, section); the ones for the block sections Phase 1 lexes are
vendored in `commonmark-0.31.2-block-examples.json` (196 examples over
`Tabs` §2.2, `Thematic breaks` §4.1, `ATX headings` §4.2, `Setext headings`
§4.3, `Indented code blocks` §4.4, `Fenced code blocks` §4.5, `HTML blocks`
§4.6, `Link reference definitions` §4.7, `Paragraphs` §4.8, `Blank lines`
§4.9), and this control runs every one through `Memo` and checks the html
for what Phase 1 CLAIMED -- never a rendering.

WHAT IS CHECKED (`phase1_blocks` -> `align`).  Phase 1's output is a block
sequence: each paragraph (`Memo.paragraphs`; a one-line block is its own
paragraph; a paragraph closed by a dropped setext underline is a heading),
each raw HTML extent (`Memo.unsupported`, kind `html`), each fenced extent
(NOT recorded by Phase 1 -- no map of raw lines outlives the pass -- so it is
re-derived here from the dropped lines: a dropped line that is a fence
opener starts one and `raw_extent` bounds it, every line of the extent
asserted dropped), each definition (dropped lines; renders nothing) and each
GFM table.  The sequence is then CONSUMED against the expected html in order:
a raw extent must appear VERBATIM (an HTML block renders as written), a
fence `<pre><code` … `</code></pre>`, a thematic break `<hr />`, an ATX
heading `<hN>` … `</hN>` at its `#` count, a setext heading at 1 for `=` /
2 for `-`, a paragraph `<p>` … `</p>`, a definition nothing; and the html
must be exhausted at the end.  Inline content (Phase 2) is skipped over, so
this is a block-structure oracle only, and position is what it checks: a
paragraph read where the spec has a heading, a definition swallowed as
prose, a fence not closed, an HTML block not opened, all break the
alignment.

WHAT IS EXCLUDED, by predicate over Phase 1's own output (printed per run;
the plan's §3.0 dispositions decide, not a hand list):
  * §5 containers (block quotes, list items): a paragraph line that is a
    `>` or list-marker line -- PROSE-AS-WRITTEN / LEXED-FLAT in §3.0, the
    html has a `<blockquote>` / `<ul>` this lexer never claims;
  * §4.4 indented code beyond the seed: PROSE-AS-WRITTEN (+ SEED) means the
    seed marks the line that OPENS a code block and the lines are read as
    prose -- so a seeded paragraph is aligned as `<pre><code>` (the seed's
    claim), but one that swallows a non-indented line (`    foo\\nbar`: the
    spec closes the code block, the prose reading joins `bar`) or one that
    the spec continues across blank lines into a second seeded chunk
    (Example 111's `chunk1 … chunk3`) makes no claim the html can confirm;
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


def phase1_blocks(memo, B):
    """Phase 1's block sequence for `memo` -- [(kind, payload)] with kinds
    `raw` (the verbatim lines), `pre` (a fence, or a seeded indented-code
    paragraph), `hr`, `h` (level), `p`, `quote` (a seeded `>` paragraph),
    `table` -- and the per-line ownership it was read from (`B` is the
    freshly loaded `plan_memo_blocks`)."""
    lines, n = memo.lines, len(memo.lines)
    owner = [None] * n
    for k, para in enumerate(memo.paragraphs):
        for lineno, _ in para.lines:
            owner[lineno - 1] = ("p", k)
    seeded = {}
    for lineno, kind, _ in memo.unsupported:
        if kind == "html":
            owner[lineno - 1] = ("html",)
        else:
            seeded[lineno - 1] = kind
    for t in memo.tables:
        for lineno in [t.header.lineno, t.header.lineno + 1] + [r.lineno for r in t.rows] + [m[0] for m in t.misses]:
            owner[lineno - 1] = ("table",)
    i = 0
    while i < n:            # the dropped lines: fences, definitions, setext underlines
        if owner[i] is not None:
            i += 1
            continue
        closer = B.fence_opener(lines[i])
        if closer is not None:
            end = B.raw_extent(lines, i, ("fence", closer))
            for j in range(i, end):
                if owner[j] is not None:
                    raise AssertionError("line %d owned twice: fence and %s" % (j + 1, owner[j]))
                owner[j] = ("fence", i)
            i = end
            continue
        if B.is_setext_underline(lines[i]) and i > 0 and owner[i - 1] is not None and owner[i - 1][0] == "p":
            owner[i] = ("underline",)
        elif B.is_blank(lines[i]):
            owner[i] = ("blank",)
        else:
            owner[i] = ("def",)
        i += 1
    blocks, i = [], 0
    while i < n:
        o = owner[i]
        if o[0] in ("blank", "def", "underline"):
            i += 1
        elif o[0] == "fence":
            blocks.append(("pre", i))
            while i < n and owner[i] == o:
                i += 1
        elif o[0] == "html":
            raw = []
            while i < n and owner[i] == o:
                # `memo.lines` is `text.split("\n")`: a document ending in a
                # line ending has a phantom empty last line, which an
                # unterminated extent (to "the end of the document") covers
                if not (i == n - 1 and lines[i] == "" and memo.text.endswith("\n")):
                    raw.append(lines[i])
                i += 1
            blocks.append(("raw", raw))
        elif o[0] == "table":
            blocks.append(("table", i))
            while i < n and owner[i] == o:
                i += 1
        else:
            para = memo.paragraphs[o[1]]
            last = i + len(para.lines) - 1
            first = lines[i]
            if seeded.get(i) == "indented-code":
                blocks.append(("pre", i))
            elif seeded.get(i) == "quote":
                blocks.append(("quote", i))
            elif B.one_line_block(first):
                rest = first.lstrip(" \t")      # not the predicate under test
                if rest.startswith("#"):
                    blocks.append(("h", len(rest) - len(rest.lstrip("#"))))
                else:
                    blocks.append(("hr",))
            elif last + 1 < n and owner[last + 1] == ("underline",):
                blocks.append(("h", 1 if "=" in lines[last + 1] else 2))
            else:
                blocks.append(("p", i))
            i = last + 1
    return blocks, owner, seeded


def excluded(memo, B, blocks, owner, seeded):
    """The disposition that keeps `memo` out of the alignment, or None."""
    for k, para in enumerate(memo.paragraphs):
        for _, text in para.lines:
            if B.starts_block(text) and not B.one_line_block(text):
                return "§5 container (block quote / list item) line: PROSE-AS-WRITTEN / LEXED-FLAT"
    if any(b[0] == "table" for b in blocks):
        return "a GFM table (local policy over pure CommonMark)"
    prev_seeded_pre = False
    for b in blocks:
        if b[0] == "pre" and b[1] in seeded:
            para = memo.paragraphs[owner[b[1]][1]]
            if any(not B.is_indented(text) and not B.is_blank(text) for _, text in para.lines):
                return "§4.4 PROSE-AS-WRITTEN: the seeded indented chunk swallows a non-indented line"
            if prev_seeded_pre:
                return "§4.4 PROSE-AS-WRITTEN: indented chunks continued across blank lines are one code block"
            prev_seeded_pre = True
        else:
            prev_seeded_pre = False
    return None


def align(blocks, memo, html):
    """Consume `html` block by block against Phase 1's claims; None when it
    aligns, else the first disagreement."""
    pos = 0

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

    for b in blocks:
        kind = b[0]
        if kind == "raw":
            want = "\n".join(b[1]) + "\n"
            if not html.startswith(want, pos):
                return "raw HTML lines %r not verbatim at html[%d:] (%r)" % (b[1], pos, html[pos:pos + 40])
            pos += len(want)
            err = None
        elif kind == "pre":
            err = expect("<pre><code", "</code></pre>\n")
        elif kind == "hr":
            err = expect("<hr />\n")
        elif kind == "h":
            err = expect("<h%d>" % b[1], "</h%d>\n" % b[1])
        elif kind == "p":
            err = expect("<p>", "</p>\n")
        else:
            err = "block kind %r is not modelled" % kind
        if err:
            return err
    if pos != len(html):
        return "html left over after Phase 1's last block: %r" % html[pos:pos + 60]
    return None


def run(B, T):
    """(ok, detail): every vendored example through `T.Memo` (the freshly
    loaded `plan_memo_tables`), aligned or excluded; a crash on any example
    is a FAIL of that example."""
    data = json.loads(EXAMPLES.read_text(encoding="utf-8"))
    passed, fails, skips = 0, [], {}
    with tempfile.TemporaryDirectory() as d:
        p = pathlib.Path(d) / "example.md"
        for ex in data["examples"]:
            no = ex["example"]
            p.write_text(ex["markdown"], encoding="utf-8")
            try:
                memo = T.Memo(p)
                blocks, owner, seeded = phase1_blocks(memo, B)
                why = excluded(memo, B, blocks, owner, seeded)
                if why is not None:
                    skips.setdefault(why, []).append(no)
                    continue
                err = align(blocks, memo, ex["html"])
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
