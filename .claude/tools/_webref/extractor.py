"""Stream HTML parser that extracts one section's prose by anchor id."""
from __future__ import annotations

import html.parser
import re


class _ExtractDone(Exception):
    """Internal sentinel — break out of HTMLParser.feed once section closes."""


class _SectionExtractor(html.parser.HTMLParser):
    """Stream-parse HTML to extract one section's body as plain text.

    Three delimitation strategies — selected by the element that carries the
    target id:

      - **Container** (emu-clause / section / div / article / aside / nav
        with the id): emit text from the opening tag through its matching
        closing tag, tracked by a depth counter on the same tag name. The
        whole subtree is the section. Used for tc39 (emu-clause) and any
        WHATWG/W3C spec that wraps sections in <section> blocks.
      - **Heading** (h1-h6 with the id): emit text starting from the
        heading through the next heading of equal-or-higher level. Used
        for the common WHATWG/W3C single-page layout (flat HTML5 outline
        without section wrappers).
      - **Open-ended** (any other tag — inline `<dfn>` / `<a>` / `<span>`
        / `<code>` / `<var>` / ..., or list items `<dt>` / `<dd>` / `<td>`,
        or any tag the spec attaches an id to that isn't structural):
        emit text starting from the element through the next heading of
        any level. Used for algorithm / concept dfn anchors (`#concept-X`,
        `#dom-Y-Z`) where the anchor sits inside a `<dfn>` introducing
        an algorithm or term definition. Captures the dfn text + the
        algorithm steps + remainder of the containing section.

    Strategy is auto-detected by tag class: container > heading > anything
    else (open-ended). The first matching id wins. If both an outer
    `<section id=X>` and an inner `<h2 id=X>` exist (rare, but legal),
    the section wins.

    Output preserves block-level structure:
      - One blank line between adjacent block elements.
      - `<ol>` / `<ul>` items are prefixed with "  1." / "  * "; nesting
        adds two more leading spaces per level.
      - `<pre>` content is verbatim (no whitespace collapsing).
      - Inline tags (`<code>`, `<var>`, `<em>`, `<a>`, `<emu-xref>`, etc.)
        contribute their text only.
      - `<script>` / `<style>` data is discarded.
    """

    # Block-level tags that introduce newlines on close. emu-* tags are
    # tc39-specific; including them here keeps tc39 algorithm output readable.
    _BLOCK_TAGS = frozenset({
        "p", "div", "section", "article", "aside", "header", "footer",
        "main", "nav", "ul", "ol", "dl", "li", "dt", "dd", "table",
        "tr", "td", "th", "blockquote", "figure", "figcaption", "pre",
        "h1", "h2", "h3", "h4", "h5", "h6",
        "emu-clause", "emu-alg", "emu-grammar", "emu-eqn", "emu-note",
        "emu-table", "emu-figure", "emu-example", "emu-import",
        "emu-intro", "emu-annex", "emu-production",
    })
    _CONTAINER_TAGS = frozenset({
        "section", "article", "aside", "nav", "div",
        "emu-clause", "emu-annex", "emu-intro",
    })
    _SKIP_TAGS = frozenset({"script", "style"})
    _HEADING_TAGS = frozenset({"h1", "h2", "h3", "h4", "h5", "h6"})

    def __init__(self, anchor: str) -> None:
        super().__init__(convert_charrefs=True)
        self._anchor = anchor
        self._parts: list[str] = []
        self._capturing = False
        self._done = False
        # Anchor-found state — flipped True the first time we see an element
        # carrying the target id, regardless of which strategy fires (container
        # / heading / open-ended). Distinct from `_capturing` because callers
        # need to discriminate "anchor never encountered" (probably a typo or
        # biblio→HTML drift) from "anchor encountered but extracted text is
        # empty" (a table/figure-only section the extractor renders as blank).
        self._found = False
        # Container strategy state
        self._container_tag: str | None = None
        self._container_depth = 0
        # Heading strategy state
        self._heading_level: int | None = None  # 1-6, the heading that bears the id
        # Open-ended strategy: any heading terminates. No additional state needed
        # beyond the flag self._open_ended.
        self._open_ended = False
        # List-rendering state — list_stack carries (tag, item_counter) frames
        # for each open <ol>/<ul>. <li> opens look at the top frame to know
        # whether to prefix "N." or "*", and increment the counter.
        self._list_stack: list[list] = []  # [[tag, counter], ...]
        # <pre> preserves whitespace; track depth so nested <pre> still works.
        # `_had_pre` records "ever entered a <pre>" for the post-processing
        # gate — when True, we skip the cosmetic `\n{3,}` collapse in
        # `extract()` so any intentional blank lines inside <pre> survive.
        # Boundary newlines (very start / very end of the captured region)
        # are still trimmed via `.strip("\n")` because they originate from
        # block-tag open/close emissions, not from <pre> content; this
        # preserves the in-pre content unchanged while keeping the output
        # free of leading/trailing blank lines.
        self._pre_depth = 0
        self._had_pre = False
        # Suppress the newline that `_open_block("p", ...)` would otherwise
        # emit immediately after a `<li>` open. Set True when emitting a list
        # bullet; consumed (and cleared) by the next `<p>` open, or cleared
        # by any other block open / non-whitespace text. Keeps the common
        # `<li><p>text</p></li>` markup (Bikeshed / WHATWG) rendering as
        # "1. text" instead of "1.\ntext" — the bullet and its text stay on
        # the same line by construction.
        self._suppress_next_p_newline = False
        # Skip-content depth (script/style)
        self._skip_depth = 0

    # -- public API ---------------------------------------------------------

    def extract(self, html_text: str) -> tuple[bool, str]:
        """Parse `html_text`. Returns (found, text).

        `found` indicates whether an element with the target id was
        encountered (regardless of whether the resulting text is empty —
        useful for distinguishing "anchor missing" from "empty section").
        """
        try:
            self.feed(html_text)
        except _ExtractDone:
            pass
        text = "".join(self._parts)
        # When `_had_pre`, skip the newline-collapse to honor the verbatim
        # promise — <pre> with intentional blank lines must round-trip
        # unchanged. Otherwise collapse 3+ blank lines (purely cosmetic;
        # block-tag open/close emits stack newlines that pile up).
        if not self._had_pre:
            text = re.sub(r"\n{3,}", "\n\n", text)
        # Strip only leading/trailing newlines — NOT spaces. List items emit
        # leading-indent ("  1. step") that whole-string `.strip()` would
        # erode when an <ol>/<ul> is the very first emitted block (e.g. when
        # anchor lands directly on <ol id=X>). Newlines are always safe to
        # trim from the boundary because block emissions add them lazily.
        return self._found, text.strip("\n")

    # -- HTMLParser hooks ---------------------------------------------------

    def handle_starttag(self, tag: str, attrs: list) -> None:
        if self._done:
            return
        if tag in self._SKIP_TAGS:
            self._skip_depth += 1
            return
        if self._skip_depth > 0:
            return

        attrs_d = dict(attrs)
        elem_id = attrs_d.get("id")

        # Discovery: not yet capturing — looking for the element with target id.
        if not self._capturing:
            # Pre-capture list-context tracking. tc39 algorithm steps carry
            # ids on individual <li> elements (e.g. `step-iteratorclose-3`);
            # when the anchor lands on such an <li>, `_open_block("li", ...)`
            # in the open-ended branch needs an already-populated _list_stack
            # to emit the correct ordered-list bullet ("3." not "*"). Without
            # this tracking the parser ignores all <ol>/<ul>/<li> markup that
            # precedes the anchor, breaking step-anchor numbering.
            #
            # Skip the bookkeeping for the anchor-carrying element itself —
            # _open_block (called from the open-ended strategy below) does
            # the push/increment, so duplicating it here would double-count.
            if elem_id != self._anchor:
                if tag in ("ol", "ul"):
                    self._list_stack.append([tag, 0])
                elif tag == "li" and self._list_stack:
                    self._list_stack[-1][1] += 1
            if elem_id == self._anchor:
                # Anchor encountered — record this BEFORE strategy dispatch so
                # the `_found` signal is set even if (somehow) no text emits.
                self._found = True
                # Strategy selection — container > heading > inline (open-ended).
                if tag in self._CONTAINER_TAGS:
                    self._capturing = True
                    self._container_tag = tag
                    self._container_depth = 1
                    return
                if tag in self._HEADING_TAGS:
                    self._capturing = True
                    self._heading_level = int(tag[1])
                    # Emit the heading's actual level (#### for <h4>, etc.)
                    # so the opening prefix matches the level format used by
                    # nested headings inside this section — readers don't get
                    # a "# 7.4.11" followed by "##### 7.4.11.1" jump.
                    self._parts.append("#" * self._heading_level + " ")
                    return
                # Any other tag (inline <dfn> / <a> / <code> / ..., or list
                # items <dt> / <dd> / <td>, or any tag the spec hangs an id
                # on): open-ended capture. Terminates at the next heading of
                # any level; works for algorithm / concept dfn anchors where
                # the dfn introduces an algorithm that follows in siblings.
                #
                # Initialize block state for the anchor-bearing element via
                # _open_block so anchors landing on structural tags (<pre> /
                # <ol> / <ul> / <li>) get pre_depth / list_stack initialized
                # correctly — without this, content inside an anchored <pre>
                # would lose verbatim whitespace, and items inside an
                # anchored <ol>/<ul> would render without numbering / bullets.
                self._capturing = True
                self._open_ended = True
                self._open_block(tag, attrs_d)
                return
            return

        # Already capturing:
        # Container strategy — track depth of the wrapping tag.
        if self._container_tag is not None and tag == self._container_tag:
            self._container_depth += 1
        # Heading strategy — a same-or-higher-level heading terminates.
        elif self._heading_level is not None and tag in self._HEADING_TAGS:
            level = int(tag[1])
            if level <= self._heading_level:
                self._done = True
                raise _ExtractDone
        # Open-ended strategy — any heading terminates.
        elif self._open_ended and tag in self._HEADING_TAGS:
            self._done = True
            raise _ExtractDone

        # Inside capture: emit block-level newlines + list bullets.
        self._open_block(tag, attrs_d)

    def handle_endtag(self, tag: str) -> None:
        if self._done:
            return
        if tag in self._SKIP_TAGS:
            if self._skip_depth > 0:
                self._skip_depth -= 1
            return
        if self._skip_depth > 0:
            return
        if not self._capturing:
            # Pre-capture list-context tracking — pop the matching frame
            # on </ol>/</ul> close so the stack reflects sibling-list
            # context at anchor-discovery time. Match by tag name on top
            # of stack so malformed HTML (unmatched closes) is tolerated.
            if (
                tag in ("ol", "ul")
                and self._list_stack
                and self._list_stack[-1][0] == tag
            ):
                self._list_stack.pop()
            return

        # Container close — decrement depth, stop on outermost close.
        if self._container_tag is not None and tag == self._container_tag:
            self._container_depth -= 1
            if self._container_depth <= 0:
                self._done = True
                raise _ExtractDone

        # Heading-level header close: nothing special (the heading content
        # already flowed through handle_data). Add a newline if it's a
        # heading inside the captured section (subsection heading).
        self._close_block(tag)

    def handle_data(self, data: str) -> None:
        if not self._capturing or self._done:
            return
        if self._skip_depth > 0:
            return
        if self._pre_depth > 0:
            self._parts.append(data)
            return
        # Collapse runs of whitespace to single space. Preserve the data even
        # if it's all whitespace — block-level newlines come from _open_block.
        cleaned = re.sub(r"\s+", " ", data)
        # Non-whitespace text after a list bullet means the bullet has its
        # inline content already; a later <p> in the same <li> should break
        # to a new paragraph. Clear the suppression flag.
        if cleaned.strip():
            self._suppress_next_p_newline = False
        self._parts.append(cleaned)

    # -- internal helpers ---------------------------------------------------

    def _open_block(self, tag: str, attrs_d: dict) -> None:
        if tag == "pre":
            self._pre_depth += 1
            self._had_pre = True
            self._suppress_next_p_newline = False
            self._parts.append("\n")
            return
        if tag in ("ol", "ul"):
            self._suppress_next_p_newline = False
            self._list_stack.append([tag, 0])
            self._parts.append("\n")
            return
        if tag == "li":
            indent = "  " * len(self._list_stack)
            if self._list_stack:
                frame = self._list_stack[-1]
                frame[1] += 1
                bullet = f"{frame[1]}." if frame[0] == "ol" else "*"
            else:
                bullet = "*"
            self._parts.append(f"\n{indent}{bullet} ")
            # Arm the suppression flag — the next <p> opening (typical
            # WHATWG/Bikeshed pattern `<li><p>text</p></li>`) should render
            # inline with the bullet, not break to a new line.
            self._suppress_next_p_newline = True
            return
        if tag in self._HEADING_TAGS:
            self._suppress_next_p_newline = False
            level = int(tag[1])
            self._parts.append("\n\n" + ("#" * level) + " ")
            return
        if tag in self._BLOCK_TAGS:
            if tag == "p" and self._suppress_next_p_newline:
                # Consume the flag without emitting a newline — keeps
                # `<li><p>text</p></li>` on one line.
                self._suppress_next_p_newline = False
                return
            self._suppress_next_p_newline = False
            self._parts.append("\n")
            return
        # Inline tags: no structural effect, no flag change.

    def _close_block(self, tag: str) -> None:
        if tag == "pre":
            if self._pre_depth > 0:
                self._pre_depth -= 1
            self._parts.append("\n")
            return
        if tag in ("ol", "ul"):
            if self._list_stack and self._list_stack[-1][0] == tag:
                self._list_stack.pop()
            self._parts.append("\n")
            return
        if tag == "li":
            return  # newline added on next <li> or list close
        if tag in self._HEADING_TAGS:
            self._parts.append("\n")
            return
        if tag in self._BLOCK_TAGS:
            self._parts.append("\n")
            return


def extract_section_body(html_bytes: bytes, anchor: str) -> tuple[bool, str]:
    """Parse `html_bytes` and return (found, text) for `anchor`.

    `found` is True iff an element with the requested id was encountered
    during parsing — distinct from `text != ""`. See `_SectionExtractor`
    for the rationale (table/figure-only sections render empty).
    """
    text = html_bytes.decode("utf-8", errors="replace")
    ext = _SectionExtractor(anchor)
    return ext.extract(text)
