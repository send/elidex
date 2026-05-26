"""`body` subcommand — fetch + extract section prose by anchor."""
from __future__ import annotations

import argparse
import re
import sys

from ..cache import NotFound, cached_fetch_url
from ..extractor import extract_section_body
from ..resolver import (
    lookup_aoid_first,
    resolve_body_source,
    tc39_resolve_body_url,
)
from ..sources.tc39 import TC39_FAMILY, tc39_biblio


def cmd_body(args: argparse.Namespace) -> None:
    """Fetch + extract section prose by anchor (or AO name for tc39)."""
    raw = args.anchor.lstrip("#").strip()
    if not raw:
        sys.exit("webref: body needs a non-empty anchor")

    # Resolution order — anchor first, AO-name as fallback:
    #
    #   1. Direct anchor lookup against biblio (tc39) / headings+dfns
    #      (WHATWG/W3C). Covers `sec-*` clauses + non-clause anchors like
    #      `table-*` / `figure-*` / `prod-*` / `term-*` etc. for tc39, and
    #      heading + dfn id space for WHATWG/W3C.
    #   2. ONLY if (a) no anchor match AND (b) input is a single CamelCase
    #      identifier (tc39 only): attempt AO-name lookup, then resolve the
    #      resulting `sec-*` anchor via the same flow. The CamelCase filter
    #      prevents bogus rejection of valid non-CamelCase anchors that
    #      happen not to be in biblio (typo → clean "anchor not found"
    #      instead of misleading "not an AO" error).
    anchor = raw
    fetch_url: str | None = None
    file_anchor: str | None = None

    if args.shortname in TC39_FAMILY:
        direct = tc39_resolve_body_url(args.shortname, raw)
        if direct is not None:
            fetch_url, file_anchor = direct
        elif re.match(r"^[A-Z][A-Za-z0-9]*$", raw):
            resolved = lookup_aoid_first(args.shortname, raw)
            if resolved is None:
                sys.exit(
                    f"webref: {raw!r} is neither a known anchor (no id "
                    f"match in {args.shortname} biblio) nor a recognized "
                    f"abstract operation"
                )
            _, _, full_anchor = resolved
            anchor = full_anchor.split("#", 1)[-1]
            direct = tc39_resolve_body_url(args.shortname, anchor)
            if direct is None:
                sys.exit(
                    f"webref: AO {raw} resolved to #{anchor} but biblio "
                    f"chapter routing failed (unexpected — please report)"
                )
            fetch_url, file_anchor = direct
        else:
            sys.exit(
                f"webref: anchor {raw!r} not found in {args.shortname} "
                f"biblio (no id match across any entry type; if you meant "
                f"an abstract operation, use the CamelCase name like "
                f"`IteratorClose`)"
            )
    else:
        fetch_url, file_anchor = resolve_body_source(args.shortname, anchor)

    # Multipage chapter 404 → single-page URL fallback (tc39 only). Rare:
    # would require an irregular tc39 chapter file the biblio-derived slug
    # doesn't match. The single-page URL (<location>) serves every anchor
    # regardless of chapter layout, so the fallback always works as a last
    # resort. WHATWG/W3C hrefs come from webref directly and are kept in
    # sync upstream, so no analogous fallback is wired there.
    try:
        html_bytes = cached_fetch_url(fetch_url)
    except NotFound:
        if args.shortname in TC39_FAMILY:
            try:
                d = tc39_biblio(args.shortname)
            except NotFound:
                sys.exit(f"webref: fetch failed: {fetch_url}")
            single_page = d.get("location", "")
            if single_page and fetch_url != single_page:
                fetch_url = single_page
                html_bytes = cached_fetch_url(fetch_url)
            else:
                sys.exit(f"webref: fetch failed: {fetch_url}")
        else:
            sys.exit(f"webref: fetch failed: {fetch_url}")
    found, text = extract_section_body(html_bytes, file_anchor)
    if not found:
        sys.exit(
            f"webref: anchor #{file_anchor} not encountered in {fetch_url} "
            f"(biblio/webref pointed here but the rendered chapter HTML "
            f"doesn't carry that id — possibly a multipage drift or a "
            f"stale biblio entry)"
        )
    if not text:
        # Anchor was found but the section is text-empty (e.g. a section
        # whose only content is a <emu-table> or <emu-figure> that the
        # extractor renders as nothing). Surface this as a *distinct* state
        # from "not found" so the user knows to look at the source directly.
        print(f"# {fetch_url}#{file_anchor}")
        print()
        print(f"(section found but extractor produced no text — likely "
              f"a table/figure-only section; view source at {fetch_url}#{file_anchor})")
        return
    # Header line for orientation (URL is the canonical citation).
    print(f"# {fetch_url}#{file_anchor}")
    print()
    print(text)
