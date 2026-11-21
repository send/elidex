"""ReSpec editor's drafts: render before extracting a section body.

A ReSpec spec's nightly ED (`w3c.github.io/<spec>/`) is served as *unrendered
source* — the ids that webref's dfns/headings extracts record are generated in
the browser, so they are absent from the HTML we fetch. The body path therefore
tries, in order (the candidate list is built by `resolver.resolve_body_candidates`,
the loop is `commands.body.cmd_body`; this module holds the ReSpec specifics):

  1. the nightly URL as fetched (works when the author pre-renders it);
  2. the W3C spec-generator's rendering of that nightly URL;
  3. the release (TR) URL — a dated snapshot, possibly older than the ED.

If (1) is already a rendered ReSpec document and lacks the anchor, (2) and (3)
are skipped (`miss_is_final`). Every fetch goes through
`cache.cached_fetch_url` (no extra headers; see that module).
Hosts contacted by this module, in addition to the existing
raw.githubusercontent.com (webref data) and the spec's own nightly host
(w3c.github.io and friends, already used by `body`):
  - www.w3.org  (spec-generator endpoint and the /TR/ release URL)
The only data sent is the spec's public nightly URL, as a query parameter.
"""
from __future__ import annotations

import datetime
import re
import urllib.parse

from .sources.webref_data import spec_entry

SPEC_GENERATOR = "https://www.w3.org/publications/spec-generator/"

_RENDERED_MARKER = b'<meta name="generator" content="ReSpec'

_TR_DATE_RE = re.compile(
    r'<time[^>]*class="[^"]*dt-published[^"]*"[^>]*datetime="([^"]+)"'
    r'|<time[^>]*datetime="([^"]+)"[^>]*class="[^"]*dt-published[^"]*"'
)


def respec_entry(shortname: str) -> tuple[str, str | None] | None:
    """(nightly_url, release_url) when the index says `shortname` is ReSpec."""
    entry = spec_entry(shortname)
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


def is_rendered_respec(html: bytes) -> bool:
    """True when `html` is a ReSpec document that has already been rendered:
    ReSpec adds `<meta name="generator" content="ReSpec ...">` to the document
    it renders, and an unrendered source (which carries `respecConfig`) lacks
    it. Checked against the cached nightly EDs (webcrypto, geolocation,
    media-source, ... carry it; the `respecConfig` source ones do not)."""
    return _RENDERED_MARKER in html


def miss_is_final(label: str | None, html: bytes) -> bool:
    """True when a miss on this candidate's text rules out the later ones:
    the nightly was already rendered, so the spec-generator would return the
    same ids and the TR is only an older snapshot of it."""
    return label == "nightly" and is_rendered_respec(html)


def source_line(label: str, html: bytes, today: datetime.date | None = None) -> str:
    """The `# source:` provenance line for a candidate `label` that hit."""
    today = today or datetime.date.today()
    if label == "nightly":
        return f"# source: ED nightly as served ({today})"
    if label == "generator":
        return f"# source: ED rendered via spec-generator ({today})"
    if label == "tr":
        return (f"# source: TR {_tr_date(html)} "
                f"(ED not renderable or lacks the anchor)")
    raise ValueError(f"unknown source label {label!r}")


SOURCE_NONE_LINE = "# source: none (no ReSpec source carries the anchor)"
