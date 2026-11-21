"""ReSpec editor's drafts: render before extracting a section body.

A ReSpec spec's nightly ED (`w3c.github.io/<spec>/`) is served as *unrendered
source* — the ids that webref's dfns/headings extracts record are generated in
the browser, so they are absent from the HTML we fetch. The body path therefore
tries, in order:

  1. the nightly URL as fetched (works when the author pre-renders it);
  2. the W3C spec-generator's rendering of that nightly URL;
  3. the release (TR) URL — a dated snapshot, possibly older than the ED.

Every fetch goes through `cache.cached_fetch_url` (no extra headers; see that
module). Hosts contacted by this module, in addition to the existing
raw.githubusercontent.com (webref data) and the spec's own nightly host
(w3c.github.io and friends, already used by `body`):
  - www.w3.org  (spec-generator endpoint and the /TR/ release URL)
The only data sent is the spec's public nightly URL, as a query parameter.
"""
from __future__ import annotations

import datetime
import re
import urllib.parse
from typing import Callable

from .cache import NotFound, cached_fetch_url
from .extractor import extract_section_body
from .sources.webref_data import _data_index

SPEC_GENERATOR = "https://www.w3.org/publications/spec-generator/"

_TR_DATE_RE = re.compile(
    r'<time[^>]*class="[^"]*dt-published[^"]*"[^>]*datetime="([^"]+)"'
    r'|<time[^>]*datetime="([^"]+)"[^>]*class="[^"]*dt-published[^"]*"'
)


def respec_entry(shortname: str) -> tuple[str, str | None] | None:
    """(nightly_url, release_url) when the index says `shortname` is ReSpec."""
    entry = _data_index().get(shortname)
    if not entry or entry.get("generator") != "respec":
        return None
    nightly = (entry.get("nightly") or {}).get("url")
    if not nightly:
        return None
    return nightly, (entry.get("release") or {}).get("url")


def generator_url(nightly_url: str) -> str:
    q = urllib.parse.urlencode({"type": "respec", "url": nightly_url})
    return f"{SPEC_GENERATOR}?{q}"


def _tr_date(html: bytes) -> str:
    m = _TR_DATE_RE.search(html.decode("utf-8", errors="replace"))
    return (m.group(1) or m.group(2)) if m else "date unknown"


def resolve_respec_body(
    shortname: str,
    fetch_url: str,
    anchor: str,
    nightly_bytes: bytes,
    fetch: Callable[[str], bytes] | None = None,
    today: datetime.date | None = None,
) -> tuple[bool, str, str, str] | None:
    """Respec-aware extraction. Returns None when this path does not apply
    (not a ReSpec spec, or `fetch_url` is not the nightly URL) — the caller
    keeps its unchanged behaviour. Otherwise returns
    (found, text, url_used, source_line); `found` False means every source
    lacked the anchor (url_used/source_line then describe the last tried)."""
    ent = respec_entry(shortname)
    if ent is None:
        return None
    nightly, release = ent
    if fetch_url.rstrip("/") != nightly.rstrip("/"):
        return None
    fetch = fetch or cached_fetch_url
    today = today or datetime.date.today()

    found, text = extract_section_body(nightly_bytes, anchor)
    if found:
        return True, text, fetch_url, f"# source: ED nightly as served ({today})"

    gen = generator_url(nightly)
    try:
        found, text = extract_section_body(fetch(gen), anchor)
        if found:
            return (True, text, fetch_url,
                    f"# source: ED rendered via spec-generator ({today})")
    except (NotFound, SystemExit):
        pass  # spec-generator failed (HTTP/network error): try the TR

    if release:
        try:
            tr = fetch(release)
        except (NotFound, SystemExit):
            tr = None
        if tr is not None:
            found, text = extract_section_body(tr, anchor)
            return (found, text, release,
                    f"# source: TR {_tr_date(tr)} (ED not renderable or lacks the anchor)")
    return False, "", fetch_url, "# source: none (nightly unrendered, spec-generator and TR failed or lack the anchor)"
