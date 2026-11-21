"""ReSpec editor's drafts: render before extracting a section body.

A ReSpec spec's nightly ED (`w3c.github.io/<spec>/`) is usually served as
*unrendered source*: the ids that webref's dfns/headings extracts record are
generated in the browser, so they are typically absent from the HTML we fetch
(a hand-written `<section id=...>` may exist, and prints as raw markup). The
body path therefore tries, in order (the candidate list is built by
`resolver.resolve_body_candidates`, the loop is `commands.body.cmd_body`; this
module holds the ReSpec specifics):

  1. the resolved page as fetched (the nightly; works when pre-rendered);
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
from typing import NamedTuple

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


_RENDERED_MARKERS = (_RENDERED_MARKER, b'id="respecDocument"')
# A ReSpec loader script (`<script ... src=".../respec-w3c.js">`, usually
# `class="remove"`): the config may be inline (`respecConfig`) or external.
_RESPEC_SCRIPT_RE = re.compile(rb'<script\b[^>]*\bsrc\s*=\s*["\']?[^"\'>\s]*respec', re.I)


def is_rendered_respec(html: bytes) -> bool:
    """True when `html` is a ReSpec document that has already been rendered:
    ReSpec adds `<meta name="generator" content="ReSpec ...">` (older output:
    `id="respecDocument"`) to the document it renders."""
    return any(m in html for m in _RENDERED_MARKERS)


def is_unrendered_respec(html: bytes) -> bool:
    """True when `html` is ReSpec *source*: not rendered (see
    `is_rendered_respec`), but it loads ReSpec (a `<script src=...respec...>`
    tag) or carries the `respecConfig` literal. Decided by property, fail-safe:
    a page that is neither rendered nor recognisably ReSpec is ordinary. On the
    cached nightly EDs of all catalog ReSpec specs this classifies every page
    as rendered or unrendered (the config can be external, so `respecConfig`
    alone misses about a seventh of them)."""
    if is_rendered_respec(html):
        return False
    return bool(_RESPEC_SCRIPT_RE.search(html)) or b"respecConfig" in html


def miss_is_final(label: str | None, html: bytes) -> bool:
    """True when a miss on this candidate's text rules out the later ones:
    the first (unlabelled) page was already rendered ReSpec, so the
    spec-generator would return the same ids and the TR is only an older
    snapshot of it."""
    return label is None and is_rendered_respec(html)


class Outcome(NamedTuple):
    """What one body source (a candidate, or the catalog lookup that yields
    the later candidates) answered. `body` records one per source tried and
    builds every diagnostic from that record."""
    label: str | None  # candidate label: None (first page) / generator / tr / catalog
    state: str  # hit | miss | unreachable | unrendered
    detail: str = ""  # the FetchError text when unreachable


_NAMES = {None: "ED", "generator": "spec-generator", "tr": "TR", "catalog": "catalog"}


def _note(o: Outcome) -> str:
    name = _NAMES[o.label]
    if o.state == "unreachable":
        return f"{name} unreachable: {' '.join(o.detail.split())}"
    if o.state == "unrendered":
        return f"{name} unrendered"
    return f"{name} lacks the anchor"


def unreachable_names(record: list[Outcome]) -> str:
    """The unreachable sources in `record`, by name (for the outage message)."""
    return ", ".join(_NAMES[o.label] for o in record if o.state == "unreachable")


def source_line(label: str | None, html: bytes, copy_date: datetime.date,
                earlier: list[Outcome] = ()) -> str:
    """The `# source:` first line of a ReSpec-path hit, naming the source and
    its date. `copy_date` is the date of the fetched copy (see
    `cache.copy_date`), `earlier` the outcomes of the sources tried before
    this one: a source that was unreachable is said so, so a TR answer never
    reads as though the ED had merely lacked the anchor."""
    if label is None:
        return f"# source: rendered ReSpec page as served ({copy_date})"
    if label == "generator":
        down = [_note(o) for o in earlier if o.state == "unreachable"]
        return (f"# source: ED rendered via spec-generator "
                f"({'; '.join([str(copy_date)] + down)})")
    if label == "tr":
        why = "; ".join(_note(o) for o in earlier) or "ED not used"
        return f"# source: TR {_tr_date(html)} ({why})"
    raise ValueError(f"unknown source label {label!r}")


SOURCE_NONE_LINE = "# source: none (no ReSpec source carries the anchor)"
