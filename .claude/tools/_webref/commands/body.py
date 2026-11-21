"""`body` subcommand — fetch + extract section prose by anchor."""
from __future__ import annotations

import argparse
import sys

from ..cache import FetchError, cached_fetch_url
from ..extractor import extract_section_body
from ..respec import SOURCE_NONE_LINE, is_unrendered_respec, miss_is_final, source_line
from ..resolver import resolve_body_candidates


def cmd_body(args: argparse.Namespace) -> None:
    """Fetch + extract section prose by anchor (or AO name for tc39)."""
    raw = args.anchor.lstrip("#").strip()
    if not raw:
        sys.exit("webref: body needs a non-empty anchor")

    anchor, candidates = resolve_body_candidates(args.shortname, raw)

    # One loop over the resolver's candidate sources: the first that fetches
    # and carries the anchor wins. A failed fetch (FetchError) or a source
    # lacking the anchor moves on to the next candidate.
    first_error: FetchError | None = None
    last = None  # last candidate fetched, named in the not-found error
    labelled = False  # a fallback (generator / TR) candidate was reached
    found = False
    for cand in candidates:
        labelled = labelled or cand.label is not None
        try:
            data = cached_fetch_url(cand.url)
        except FetchError as e:
            first_error = first_error or e
            continue
        last = cand
        found, text = extract_section_body(data, anchor)
        # An unrendered ReSpec source's hand-written `<section id=...>` would
        # print raw markup; only rendered text may answer, else go on to the
        # generator (the catalog is consulted only when the loop gets here).
        if found and is_unrendered_respec(data):
            found = False
        if found:
            if cand.label:
                print(source_line(cand.label, data))
            break
        if miss_is_final(cand.label, data):
            break
    if last is None:
        raise first_error  # every fetch failed: surface the first (primary) one
    if not found and first_error is not None:
        # A source could not be fetched and none of the others carried the
        # anchor: that is an outage, not evidence the anchor is absent.
        sys.exit(f"{first_error} (anchor #{anchor} not found in the sources "
                 f"that were reachable; a source above was unreachable, so "
                 f"absence is unverified)")
    if not found:
        if labelled:  # a ReSpec ED fallback chain was tried
            print(SOURCE_NONE_LINE)
        sys.exit(
            f"webref: anchor #{anchor} not encountered in {last.cite or last.url} "
            f"(biblio/webref pointed here but the rendered chapter HTML "
            f"doesn't carry that id — possibly a multipage drift or a "
            f"stale biblio entry)"
        )
    fetch_url = last.cite or last.url
    if not text:
        # Anchor was found but the section is text-empty (e.g. a section
        # whose only content is a <emu-table> or <emu-figure> that the
        # extractor renders as nothing). Surface this as a *distinct* state
        # from "not found" so the user knows to look at the source directly.
        print(f"# {fetch_url}#{anchor}")
        print()
        print(f"(section found but extractor produced no text — likely "
              f"a table/figure-only section; view source at {fetch_url}#{anchor})")
        return
    # Header line for orientation (URL is the canonical citation).
    print(f"# {fetch_url}#{anchor}")
    print()
    print(text)
