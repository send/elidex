"""`body` subcommand — fetch + extract section prose by anchor."""
from __future__ import annotations

import argparse
import sys

from ..cache import FetchError, cached_fetch_url, copy_date
from ..extractor import extract_section_body
from ..respec import (SOURCE_NONE_LINE, Outcome, is_rendered_respec,
                      is_unrendered_respec, miss_is_final, source_line,
                      unreachable_names)
from ..resolver import resolve_body_candidates


def cmd_body(args: argparse.Namespace) -> None:
    """Fetch + extract section prose by anchor (or AO name for tc39)."""
    raw = args.anchor.lstrip("#").strip()
    if not raw:
        sys.exit("webref: body needs a non-empty anchor")

    anchor, candidates = resolve_body_candidates(args.shortname, raw)

    # One loop over the resolver's candidate sources: the first that fetches
    # and carries the anchor wins. Each source's outcome (hit / miss /
    # unreachable / unrendered) goes into `record`, and every diagnostic below
    # (the `# source:` line, the outage or not-found message) is built from it.
    # Advancing the iterator can itself fail: the tail's catalog lookup is a
    # source too (label "catalog").
    record: list[Outcome] = []
    first_error: FetchError | None = None
    last = None  # last candidate fetched, named in the not-found error
    found = False
    text = ""
    it = iter(candidates)
    while True:
        try:
            cand = next(it)
        except StopIteration:
            break
        except FetchError as e:
            first_error = first_error or e
            record.append(Outcome("catalog", "unreachable", str(e)))
            break
        try:
            data = cached_fetch_url(cand.url)
        except FetchError as e:
            first_error = first_error or e
            record.append(Outcome(cand.label, "unreachable", str(e)))
            continue
        last = cand
        found, text = extract_section_body(data, anchor)
        # An unrendered ReSpec source's hand-written `<section id=...>` would
        # print raw markup; only rendered text may answer, else go on to the
        # generator (the catalog is consulted only when the loop gets here).
        if is_unrendered_respec(data):
            found = False
            record.append(Outcome(cand.label, "unrendered"))
        elif found:
            record.append(Outcome(cand.label, "hit"))
            # Every ReSpec-path answer names its source and the date of the
            # fetched copy on the first line: a fallback by its label, the
            # first page by the rendered marker in its bytes (no catalog).
            if cand.label or is_rendered_respec(data):
                print(source_line(cand.label, data, copy_date(cand.url), record[:-1]))
            break
        else:
            record.append(Outcome(cand.label, "miss"))
        if miss_is_final(cand.label, data):
            break
    if last is None:
        raise first_error  # every fetch failed: surface the first (primary) one
    if not found and first_error is not None:
        # A source could not be fetched and none of the others carried the
        # anchor: that is an outage, not evidence the anchor is absent.
        sys.exit(f"{first_error} (anchor #{anchor} not found in the sources "
                 f"that were reachable; unreachable: {unreachable_names(record)}, so "
                 f"absence is unverified)")
    if not found:
        if any(o.label is not None for o in record):  # a ReSpec fallback chain was tried
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
