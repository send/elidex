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

If (1) is already a rendered ReSpec document (`classify_respec`) and lacks the anchor, (2) and (3)
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


# A ReSpec loader script: a `<script src=...>` (the `src` attribute itself,
# not `data-src`) whose URL's file name is a ReSpec build. The loader URLs in
# the cached catalog nightlies are `.../Tools/respec/respec-w3c`, `.../respec-w3c-common`
# (with or without `.js`, or protocol-relative) and a local `respec-w3c.js`;
# plugin bundles (`respec-vc@x/dist/main.js`) and config files
# (`respec-config.js`) are not loaders and always sit beside the real one.
_SCRIPT_SRC_RE = re.compile(
    rb'<script\b[^>]*?(?<![-\w])src\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|([^\s>]*))', re.I)
_LOADER_NAME_RE = re.compile(r"respec-w3c(?:-common)?(?:\.min)?(?:\.js)?", re.I)


def _has_respec_loader(html: bytes) -> bool:
    for m in _SCRIPT_SRC_RE.finditer(html):
        url = next(g for g in m.groups() if g is not None).decode("utf-8", "replace")
        name = re.split(r"[?#]", url, maxsplit=1)[0].rsplit("/", 1)[-1]
        if _LOADER_NAME_RE.fullmatch(name):
            return True
    return False


def classify_respec(html: bytes) -> str:
    """The one classifier for a fetched page: 'rendered' | 'unrendered' |
    'ordinary'. Fixed precedence, first match wins:
      1. `<meta name="generator" content="ReSpec ...">` -> rendered (ReSpec
         adds it to the document it renders);
      2. a ReSpec loader script (see `_has_respec_loader`) or the
         `respecConfig` literal -> unrendered (rendering strips the
         `class="remove"` scripts, so a loader means source);
      3. `id="respecDocument"` -> rendered (older rendered output, e.g. N-Quads
         TR; hand-written ReSpec sources also carry it on `<body>`, which is why
         it ranks after the loader test);
      4. else ordinary (fail-safe: not recognisably ReSpec).
    On the cached catalog pages this puts every ReSpec nightly in rendered or
    unrendered (the config can be external, so `respecConfig` alone misses
    about a seventh of them)."""
    if _RENDERED_MARKER in html:
        return "rendered"
    if _has_respec_loader(html) or b"respecConfig" in html:
        return "unrendered"
    if b'id="respecDocument"' in html:
        return "rendered"
    return "ordinary"


def miss_is_final(label: str | None, html: bytes) -> bool:
    """True when a miss on this candidate's text rules out the later ones:
    the first (unlabelled) page was already rendered ReSpec, so the
    spec-generator would return the same ids and the TR is only an older
    snapshot of it."""
    return label is None and classify_respec(html) == "rendered"


class Outcome(NamedTuple):
    """What one body source (a candidate, or the catalog lookup that yields
    the later candidates) answered. `body` records one per source tried and
    builds every diagnostic from that record."""
    label: str | None  # candidate label: None (first page) / generator / tr / catalog
    state: str  # hit | miss | unreachable | unrendered
    detail: str = ""  # the FetchError text when unreachable
    date: datetime.date | None = None  # date of the fetched copy (`cache.Fetched.date`)


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


def source_line(label: str | None, html: bytes, date: datetime.date,
                earlier: list[Outcome] = ()) -> str:
    """The `# source:` first line of a ReSpec-path hit, naming the source and
    its date. `date` is the date of the fetched copy (see
    `cache.Fetched.date`), `earlier` the outcomes of the sources tried before
    this one: a source that was unreachable is said so, so a TR answer never
    reads as though the ED had merely lacked the anchor."""
    if label is None:
        return f"# source: rendered ReSpec page as served ({date})"
    if label == "generator":
        down = [_note(o) for o in earlier if o.state == "unreachable"]
        return (f"# source: ED rendered via spec-generator "
                f"({'; '.join([str(date)] + down)})")
    if label == "tr":
        why = "; ".join(_note(o) for o in earlier) or "ED not used"
        return f"# source: TR {_tr_date(html)} ({why})"
    raise ValueError(f"unknown source label {label!r}")


SOURCE_NONE_LINE = "# source: none (no ReSpec source carries the anchor)"
