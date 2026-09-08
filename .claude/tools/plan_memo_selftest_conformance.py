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

THE INLINE HALF (`run_inline`; §6.6 since PR #510 R17, the whole §3.0b closed
list since R21).  The spec's example lists for every inline section the plan's
§3.0b calls LEXED or MASKED -- `Backslash escapes` §2.4 (12-24), `Entity
and numeric character references` §2.5 (25-41), `Code spans` §6.1 (328-349),
`Links` §6.3 (482-571), `Images` §6.4 (572-593), `Autolinks` §6.5 (594-612),
`Raw HTML` §6.6 (613-632): 203 examples -- are vendored in
`commonmark-0.31.2-inline-examples.json` and run through the SAME `align` the
block corpus runs through.  One aligner, two corpora: an inline example's
BLOCK structure is checked exactly as a block example's is (eight of them are
not paragraphs at all -- §2.4 Examples 18 / 19 / 21 / 24 and §2.5 31 / 34 / 36
/ 38 are headings, fences, indented code and a definition -- and forty-odd
§6.3 / §6.4 examples are a reference definition plus a paragraph), and a block
example's INLINE claim is checked exactly as an inline example's, so neither
half can be green through a hole the other would show.

The inline property is `inline_claim`: each masked §6.6 span verbatim and in
order in its `<p>` body -- those spans are then BLANKED, since a raw `<a
href=…>` a span carries is the author's text, not the renderer's -- then one
`<a href=` per §6.3 link plus §6.5 autolink, one `<code>` per §6.1 code span,
one `<img src=` per §6.4 image that RESOLVED.  A span masked where the html
escapes (`<33>`, `< a>`, `<a href='bar'title=title>`) is not verbatim; a tag
left unmasked where the html emits it takes an `<a href=` with it; a link read
where the spec has none, a link lost inside an autolink or inside a resolved
image's description, an autolink read as a raw tag, a code span mis-closed, an
image read as a link, all move a count.  §6.2 emphasis, §6.7 / §6.8 line
breaks and §6.9 textual content are PROSE-AS-WRITTEN (§3.0b) and the tags they
emit (`<em>`, `<strong>`, `<br />`) are counted by nothing here -- which is
why the R17 property's bare `<` count, correct over §6.6 alone, could not
survive the corpus reaching §6.3.

NOTHING is excluded from either corpus (both print 0), and the one exclusion
predicate is the GFM-table arm shared with the block half.  ⚠ The first R17
draft excluded "a non-paragraph block" by predicate, and under the mutant that
makes whitespace before an attribute optional Example 622 (`<a
href='bar'title=title>`) became a type-7 HTML BLOCK (condition 7 reads the
same tag grammar), was excluded, and the mutant survived: an exclusion arm
over a set that is empty by construction is a hole, not a disposition -- the
unified aligner has no such arm, because a block of another kind is checked as
that kind.
"""

import json
import pathlib
import re
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
EXAMPLES = HERE / "commonmark-0.31.2-block-examples.json"
INLINE_EXAMPLES = HERE / "commonmark-0.31.2-inline-examples.json"

# where a tight item's bare paragraph text ends: the item's close, or the
# next block's tag on its own line (the renderer starts every block tag on a
# fresh line, and puts nothing else at a line start inside inline text)
_TIGHT_END = re.compile(r"</li>|\n<(?:ul|ol|pre|blockquote|h[1-6]|hr)\b")


def excluded(memo):
    """The disposition that keeps `memo` out of the alignment, or None."""
    if any(b[0] == "table" for b in memo.sequence):
        return "a GFM table (local policy over pure CommonMark)"
    return None


def _blank(s, spans):
    """`s` with every span replaced by spaces, so a later count cannot see
    inside it -- spelled here over the HTML rather than over the source, since
    this module must not import the subject to check it (the checker itself no
    longer blanks anything wholesale: `stream` renders each span as §3.0b's
    `Renders` column says)."""
    buf = list(s)
    for a, b in spans:
        for k in range(a, b):
            if buf[k] != "\n":
                buf[k] = " "
    return "".join(buf)


_A = re.compile(r"<a href=")
_IMG = re.compile(r"<img src=")
# The tags the renderer emits for the §3.0b PROSE-AS-WRITTEN constructs, which
# Phase 2 claims nothing about: since PR #510 design re-gate 4 that is §6.7
# hard line breaks alone -- §6.2 emphasis is LEXED and claimed below.
# Measured over both corpora: no other tag stands in a `<p>` body outside a
# raw HTML span (`<responsive-image>`, `<?php`, `<!ELEMENT` and the rest are
# §6.6 spans, blanked before this is read).
_PROSE_TAGS = re.compile(r"<br />")


def inline_claim(lx, body):
    """Phase 2's claim about ONE paragraph against the `<p>` body the spec
    renders for it (the plan's §3.0b falsifier); None when they agree.

    Four properties, one per LEXED / MASKED disposition, each read off
    the html's own tags rather than off a re-rendering:
      * §6.6 raw HTML -- "rendered in HTML without escaping", so each masked
        span must stand VERBATIM, in order, in the body (a mis-bounded span
        or one masked where the html escapes fails here);
      * the spans just matched are then blanked, since a raw `<a href=…>` or
        a literal `<code>` inside one is the author's text, not the
        renderer's, and nothing below may count it;
      * §6.1 code spans -- one `<code>` per span Phase 2 read;
      * §6.3 links and §6.5 autolinks -- one `<a href=` per link Phase 2
        recorded plus one per autolink it masked (an autolink IS a link to
        the renderer, §6.5: "parsed as links"), so a link read where the
        spec has none, a link lost inside an autolink or a resolved image's
        description, and an autolink read as a raw tag all go red;
      * §6.4 images -- one `<img src=` per image that RESOLVED (the tails
        Phase 2 records as kind `image`; a demoted link or a nested image
        inside a description renders no tag of its own, which is why the
        kind is recorded);
      * §6.2 emphasis (PR #510 design re-gate 4) -- one `<em>` per pair
        Phase 2 matched with one delimiter character a side and one
        `<strong>` per pair with two, `_` and `*` alike, and a GFM
        strikethrough pair (`<del>`) for none of them, since pure
        CommonMark has no such extension: a run paired where the spec
        leaves it literal, or left literal where the spec pairs it, moves
        one of these counts, and the `Emphasis and strong emphasis`
        examples 350-481 are vendored for exactly this.
    §6.7 / §6.8 line breaks and §6.9 textual content are PROSE-AS-WRITTEN
    (§3.0b) and emit a tag Phase 2 makes no claim about (`<br />`), so
    nothing here counts a bare `<`."""
    spans, pos, cut = [lx.text[a:b] for a, b in lx.html], 0, []
    for sp in spans:
        k = body.find(sp, pos)
        if k < 0:
            return "span %r masked but not verbatim in the html after [%d:] (%r)" % (sp, pos, body[pos:pos + 40])
        cut.append((k, k + len(sp)))
        pos = k + len(sp)
    rest = _blank(body, cut)
    got, want = len(_A.findall(rest)), len(lx.links) + len(lx.autolinks)
    if got != want:
        return ("the html emits %d `<a href=` outside the masked raw HTML, Phase 2 claims %d link(s) "
                "+ %d autolink(s)" % (got, len(lx.links), len(lx.autolinks)))
    got, want = rest.count("<code>"), len(lx.code)
    if got != want:
        return "the html emits %d `<code>`, Phase 2 claims %d code span(s)" % (got, want)
    got, want = len(_IMG.findall(rest)), sum(1 for e in lx.images if e[2] == "image")
    if got != want:
        return "the html emits %d `<img src=`, Phase 2 claims %d resolved image(s)" % (got, want)
    img = want
    em = [p for p in lx.emphasis if p[4] != "~" and p[6] == "em"]
    for tag, use in (("em", 1), ("strong", 2)):
        got, want = rest.count("<%s>" % tag), sum(1 for p in em if p[5] == use)
        if got != want:
            return "the html emits %d `<%s>`, Phase 2 claims %d such pair(s)" % (got, tag, want)
    got, want = rest.count("<del>"), sum(1 for p in lx.emphasis if p[4] == "~" and p[6] == "em")
    if got != want:
        return "the html emits %d `<del>`, Phase 2 claims %d GFM strikethrough pair(s)" % (got, want)
    # ... and NOTHING is left over: every `<` still standing outside the
    # masked spans must belong to a tag one of the three claims above
    # accounts for (2 per link / autolink, 2 per code span, 1 per image --
    # `<img …/>` has no closing tag) or to a PROSE-AS-WRITTEN construct's.
    # This is the direction the counts alone do not give: a §6.6 arm dropped
    # from the tag grammar leaves the comment / instruction / CDATA section
    # unmasked, the html emits it verbatim, and no count above moves (⚠ the
    # R17 property's `<` conservation had this direction; four of its mutants
    # survived the first R21 draft, which had counts only).
    left = rest.count("<") - 2 * (len(lx.links) + len(lx.autolinks)) - 2 * len(lx.code) - img
    left -= 2 * (len(em) + sum(1 for p in lx.emphasis if p[4] == "~" and p[6] == "em"))
    left -= len(_PROSE_TAGS.findall(rest))
    if left:
        return ("%d `<` of the html belong to no tag Phase 2 accounts for (a construct the html emits "
                "verbatim that no span masks?): %r" % (left, rest[:80]))
    return None


def align(memo, html):
    """Consume `html` block by block against Phase 1's sequence, and each
    paragraph's `<p>` body against Phase 2's inline claim (`inline_claim`);
    None when it aligns, else the first disagreement."""
    pos, k = 0, 0
    sequence, raw_html = memo.sequence, {l: t for l, t, reading in memo.raw if reading == "html"}
    paras = {p.lines[0][0]: p for p in memo.paragraphs}

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

    def bare_paragraph(lo):
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
        end = m.start()
        err = inline_claim(paras[lo].lexed, html[pos:end])
        pos = end + (1 if html[end] == "\n" else 0)
        return err

    def paragraph(lo):
        """A `<p>…</p>` block, and Phase 2's inline claim over its body."""
        nonlocal pos
        if not html.startswith("<p>", pos):
            return "'<p>' expected at html[%d:], found %r" % (pos, html[pos:pos + 40])
        end = html.find("</p>\n", pos)
        if end < 0:
            return "'<p>' not closed by '</p>'"
        err = inline_claim(paras[lo].lexed, html[pos + 3:end])
        pos = end + len("</p>\n")
        return err

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
                err = bare_paragraph(lo) if tight else paragraph(lo)
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
    """The BLOCK corpus: the spec's §2.2 / §4 / §5 example lists (the plan's
    §3.0 closed list)."""
    return _run(M, EXAMPLES)


def run_inline(M):
    """The INLINE corpus: the spec's §2.4 / §2.5 / §6.1 / §6.3 / §6.4 / §6.5
    / §6.6 example lists (the plan's §3.0b closed list).  The SAME aligner:
    an inline example's block structure is checked exactly as a block
    example's is, and a block example's inline claim exactly as an inline
    example's -- one property, two corpora, so neither half can be green
    through a hole the other would show."""
    return _run(M, INLINE_EXAMPLES)


def _run(M, corpus):
    """(ok, detail): every example of `corpus` through `M.Memo` (the freshly
    loaded `plan_memo_memo`), aligned or excluded; a crash on any example is
    a FAIL of that example."""
    data = json.loads(corpus.read_text(encoding="utf-8"))
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
    return _report(data, passed, fails, skips)


def _report(data, passed, fails, skips):
    n_skip = sum(len(v) for v in skips.values())
    lines = ["%d examples: %d aligned, %d excluded, %d FAIL" % (len(data["examples"]), passed, n_skip, len(fails))]
    for why, nos in sorted(skips.items()):
        lines.append("  excluded (%s): %s" % (why, " ".join(str(x) for x in nos)))
    for no, section, err in fails:
        lines.append("  FAIL Example %d (%s): %s" % (no, section, err))
    return not fails and passed > 0, "\n".join(lines)
