"""Anchor → fetch-URL resolvers + section/AO lookup helpers.

Two roles, both keyed on (shortname, anchor-or-name):

  - `resolve_body_candidates` (with `tc39_resolve_body_url` /
    `webref_resolve_body_url`) answers "where do I fetch the spec HTML that
    contains this anchor": an ordered list of candidate sources, the first
    that yields the section wins (see `Candidate`).
  - `lookup_section` / `lookup_heading` / `lookup_aoid_first` answer
    "what §number + title + anchor does this section/AO resolve to"
    (no fetch beyond biblio / headings JSON).
"""
from __future__ import annotations

import re
import sys
from typing import Callable, NamedTuple

from .cache import NotFound
from .respec import generator_url, respec_entry
from .section_sort import heading_number_title, sec_number_key
from .sources.tc39 import TC39_FAMILY, tc39_biblio, tc39_clauses_by_id
from .sources.webref_data import try_fetch_data_json


# tc39 multipage chapter file derivation is purely structural: the top-level
# clause's `id` (always "sec-X" by convention) maps to `X.html`. Verified
# across all 38 ecma262 chapters + annexes — no per-spec carve-outs needed.
# If tc39 ever introduces an irregular chapter file, the multipage fetch fails
# and `cmd_body` moves on to the next candidate, the single-page URL
# (`<location>#<anchor>`), which serves every anchor regardless of chapter
# layout. See `resolve_body_candidates`.


class Candidate(NamedTuple):
    """One place a section's text may come from. `cmd_body` tries a list of
    these in order; the first that fetches and carries the anchor wins.

    `url`   — what is fetched.
    `label` — provenance kind for the `# source:` line (`respec.source_line`),
              None when the output carries no such line.
    `cite`  — the URL printed in the `# <url>#<anchor>` header; `url` unless
              the text is a rendering of another URL.
    """

    url: str
    label: str | None = None
    cite: str | None = None


def tc39_resolve_body_url(d: dict, anchor: str) -> tuple[str, str] | None:
    """Resolve an anchor against a tc39 biblio (`d`, from `tc39_biblio`) →
    (multipage_url, anchor) or None.

    Accepts ANY biblio-known id, not just `type=clause`. Non-clause entries
    (`table-*`, `figure-*`, `prod-*`, `step-*`, `term-*`, etc.) carry a
    `refId` pointing into their enclosing clause; we walk that chain up to
    the first ancestor clause and derive the chapter file from its
    top-level §-number. This way every anchor biblio tracks resolves —
    the resolver invariant "biblio-known id → multipage URL" holds by
    construction, rather than being limited to a single entry-type subset.

    Returns `tuple[str, str]` when the anchor resolved to a URL, `None` when it
    is not in biblio (typo / unknown). The biblio fetch itself (which may raise
    `NotFound` / `FetchError`) is the caller's: do NOT swallow it, or callers
    see a misleading "anchor not found" path.
    """
    location = d.get("location", "")
    entries = d.get("entries", [])

    # Build id → entry index across ALL entry types. biblio uses string `id`
    # uniquely across the file (clause / op / table / figure / term / prod /
    # step), and refId always points to another such id, so a single dict
    # supports both direct lookup and the refId walk below.
    by_id: dict[str, dict] = {}
    for e in entries:
        eid = e.get("id")
        if eid and eid not in by_id:
            by_id[eid] = e

    target = by_id.get(anchor)
    if target is None:
        return None

    # Locate the containing clause for chapter routing. Direct clause uses
    # its own number; otherwise walk refId up until we hit a clause. Some
    # entry types (`table` / `figure`) have no `refId` in biblio — we can't
    # route them to a multipage chapter, so we fall back to the single-page
    # URL further below. (The biblio still knows the anchor, so the caller
    # gets a working URL — it's just the 3 MB single-page chapter file
    # rather than a 200-800 KB multipage chapter.)
    if target.get("type") == "clause":
        clause = target
    else:
        clause = None
        cur = target
        guard = 32  # paranoia: cap walk depth to detect any refId cycle
        while cur is not None and guard > 0:
            guard -= 1
            ref = cur.get("refId")
            if not ref:
                break
            cur = by_id.get(ref)
            if cur is not None and cur.get("type") == "clause":
                clause = cur
                break

    # Single-page fallback when biblio lacks the chapter routing info.
    # Distinct from the "not in biblio at all" None return above — here we
    # know the anchor is valid; we just can't pick a smaller chapter file
    # to fetch.
    if clause is None:
        return (location, anchor)

    number = clause.get("number", "")
    if not number:
        return (location, anchor)

    # Top-level chapter §-prefix is the first component (digit chapter, e.g. "7"
    # from "7.4.11"; annex letter, e.g. "B" from "B.3.1").
    top_prefix = number.split(".", 1)[0]
    top_clause = None
    for e in entries:
        if e.get("type") == "clause" and e.get("number", "") == top_prefix:
            top_clause = e
            break
    if top_clause is None:
        return (location, anchor)
    top_id = top_clause.get("id", "")
    # Strip the canonical "sec-" prefix to get the chapter file slug. Falls
    # back to the raw id if a chapter ever lacks the prefix (defensive — all
    # current ecma262/ecma402 top-level clauses have it).
    chapter = top_id[4:] if top_id.startswith("sec-") else top_id
    if not chapter:
        return (location, anchor)
    return (f"{location}multipage/{chapter}.html", anchor)


def webref_resolve_body_url(shortname: str, anchor: str) -> tuple[str, str] | None:
    """Resolve WHATWG/W3C (shortname, anchor) → (multipage_url, anchor) or None.

    Two-step lookup:
      1. `headings/<spec>.json` for section anchors (heading-style ids).
      2. `dfns/<spec>.json` for algorithm / concept / IDL-attribute anchors
         (the WHATWG `concept-X`, `dom-Y-Z` family). Both carry a full
         multipage `href`.

    The fragment is split off so the cache key is the chapter HTML URL.
    Returns None only when both lookups miss; callers then surface an error.
    """
    data = try_fetch_data_json("headings", shortname)
    if data is not None:
        for h in data.get("headings", []):
            if h.get("id") == anchor:
                href = h.get("href", "")
                if href:
                    return (href.split("#", 1)[0], anchor)

    ddata = try_fetch_data_json("dfns", shortname)
    if ddata is None:
        return None
    for d in ddata.get("dfns", []):
        if d.get("id") == anchor:
            href = d.get("href", "")
            if href:
                return (href.split("#", 1)[0], anchor)
    return None


def _tc39_body_candidates(shortname: str, raw: str) -> tuple[str, list[Candidate]]:
    # Resolution order — anchor first, AO-name as fallback:
    #
    #   1. Direct anchor lookup against biblio. Covers `sec-*` clauses +
    #      non-clause anchors like `table-*` / `figure-*` / `prod-*` /
    #      `term-*` etc.
    #   2. ONLY if (a) no anchor match AND (b) input is a single CamelCase
    #      identifier: attempt AO-name lookup, then resolve the resulting
    #      `sec-*` anchor via the same flow. The CamelCase filter prevents
    #      bogus rejection of valid non-CamelCase anchors that happen not to
    #      be in biblio (typo → clean "anchor not found" instead of
    #      misleading "not an AO" error).
    d = tc39_biblio(shortname)
    anchor = raw
    direct = tc39_resolve_body_url(d, raw)
    if direct is None:
        if not re.match(r"^[A-Z][A-Za-z0-9]*$", raw):
            sys.exit(
                f"webref: anchor {raw!r} not found in {shortname} "
                f"biblio (no id match across any entry type; if you meant "
                f"an abstract operation, use the CamelCase name like "
                f"`IteratorClose`)"
            )
        resolved = lookup_aoid_first(shortname, raw)
        if resolved is None:
            sys.exit(
                f"webref: {raw!r} is neither a known anchor (no id "
                f"match in {shortname} biblio) nor a recognized "
                f"abstract operation"
            )
        _, _, full_anchor = resolved
        anchor = full_anchor.split("#", 1)[-1]
        direct = tc39_resolve_body_url(d, anchor)
        if direct is None:
            sys.exit(
                f"webref: AO {raw} resolved to #{anchor} but biblio "
                f"chapter routing failed (unexpected — please report)"
            )
    url, anchor = direct
    # Multipage chapter first; the single-page URL (`location`, ~3 MB) serves
    # every anchor regardless of chapter layout, so it is the last resort
    # should the biblio-derived chapter file not exist.
    cands = [Candidate(url)]
    location = d.get("location", "")
    if location and location != url:
        cands.append(Candidate(location))
    return anchor, cands


class CandidateChain:
    """Iterable of candidates whose tail is computed only when iteration
    reaches it (and which raises FetchError there if the catalog is down).
    `cmd_body` asks for a second candidate only after the first
    missed, failed to fetch, or was an unrendered ReSpec source — so the
    webref catalog (`index.json`, ~1.5 MB), which the tail needs, is fetched
    only then."""

    def __init__(self, first: Candidate, tail: Callable[[], list[Candidate]]):
        self.first = first
        self._tail = tail

    def __iter__(self):
        yield self.first
        yield from self._tail()


def _respec_fallbacks(shortname: str, url: str) -> list[Candidate]:
    """[spec-generator rendering, TR release] when `url` is the nightly ED of
    a ReSpec spec, else []. Consults the catalog; an unreachable catalog
    raises FetchError, which `body` records as an unreachable source (the
    absence of the anchor is then unverified, not "not encountered").

    A ReSpec spec's other pages (multipage `href`s) are not the nightly, so
    rendering "the nightly" would answer a different page: only the nightly
    itself gets the generator -> TR chain. A nightly root may end in `.html`
    (9 catalog specs), so the URL shape is not a filter."""
    ent = respec_entry(shortname)
    if ent is None or url.rstrip("/") != ent[0].rstrip("/"):
        return []
    nightly, release = ent
    cands = [Candidate(generator_url(nightly), "generator", cite=url)]
    if release:
        cands.append(Candidate(release, "tr"))
    return cands


def resolve_body_candidates(shortname: str, raw: str) -> tuple[str, list[Candidate]]:
    """Resolve (shortname, user-supplied anchor or AO name) → (anchor,
    ordered candidate sources). The only place that decides where a section's
    text comes from.

      - tc39: [multipage chapter, single-page `location`]
      - otherwise: [the resolved page, then — only if the loop gets past it
        (miss / fetch failure / unrendered ReSpec source) and the catalog says
        it is a ReSpec spec's nightly ED — the spec-generator's rendering of
        it, the release (TR)]. An ED may be served unrendered (no ids), so the
        rendering or the dated TR may be what carries the anchor.

    Single-page non-tc39 fallback is intentionally NOT supported: webref
    `href` is universal across WHATWG/W3C specs that webref tracks, and
    untracked specs (zero `headings/<x>.json`) lack the citation metadata
    body extraction depends on anyway.
    """
    if shortname in TC39_FAMILY:
        return _tc39_body_candidates(shortname, raw)

    mp = webref_resolve_body_url(shortname, raw)
    if mp is None:
        sys.exit(
            f"webref: no multipage URL known for {shortname}#{raw} "
            f"(WHATWG/W3C: anchor must appear in headings/{shortname}.json "
            f"or dfns/{shortname}.json)"
        )
    url, anchor = mp
    return anchor, CandidateChain(Candidate(url), lambda: _respec_fallbacks(shortname, url))


def lookup_section(shortname: str, ref: str) -> tuple[str, str, str] | None:
    """Resolve (shortname, ref) → (number, title, anchor) or None.

    `ref` discriminator (structural, not enumerated):
      - Digit-leading (e.g. `15.7.14`) OR single uppercase annex letter
        followed by `.`/digit/end (e.g. `A.1`, `C.2.3`, `B`): §-number path,
        heading lookup. Covers any tc39 annex letter (A-Z) and W3C/WHATWG
        annexes without per-letter enumeration.
      - Anything else (alphabetic AO name like `ClassDefinitionEvaluation`,
        always CamelCase with at least one lowercase letter): tc39 aoid
        lookup. Only tc39 specs support aoid — for other specs the caller
        must pass a section number.
    """
    # Normalize: strip leading `§` AND surrounding whitespace.
    # `"§ 15.7.14"` (copy/paste with space after the section mark) is a
    # common author input shape — without the .strip() it would silently
    # misroute to AOID lookup.
    ref = ref.lstrip("§").strip()
    if re.match(r"^(\d|[A-Z]([.0-9]|$))", ref):
        return lookup_heading(shortname, ref)
    return lookup_aoid_first(shortname, ref)


def lookup_heading(shortname: str, prefix: str) -> tuple[str, str, str] | None:
    """Return (number, title, anchor) for §prefix in `shortname`, or None.

    **Exact-match-preferred** semantics (drift-catch invariant): if a clause
    with `number == prefix` exists, return it. Otherwise return None — even
    if prefix-matches exist (e.g. §prefix.X children). Silently descending
    to the first child made coverage-map / preflight verify accept drifted
    or imprecise refs (`§4.13` silently resolving to `§4.13.1`). Callers
    that want prefix listing should use `cmd_heading` directly (without
    `--exact`), which preserves the CLI's documented prefix-listing semantics.
    """
    if shortname in TC39_FAMILY:
        try:
            d = tc39_biblio(shortname)
        except NotFound:
            return None
        location = d.get("location", "")
        for e in d.get("entries", []):
            if e.get("type") != "clause":
                continue
            if e.get("number", "") == prefix:
                return (e.get("number", ""), e.get("title", ""), f"{location}#{e.get('id','')}")
        return None
    data = try_fetch_data_json("headings", shortname)
    if data is None:
        return None
    for h in data.get("headings", []):
        number, title = heading_number_title(h)
        if number == prefix:
            return (number, title, f"#{h.get('id','')}")
    return None


def lookup_aoid_first(shortname: str, name: str) -> tuple[str, str, str] | None:
    """Return the first (number, title, anchor) for tc39 AO name, or None."""
    if shortname not in TC39_FAMILY:
        return None
    try:
        d = tc39_biblio(shortname)
    except NotFound:
        return None
    entries = d.get("entries", [])
    clauses = tc39_clauses_by_id(entries)
    location = d.get("location", "")
    needle = name.lower()
    by_ref: dict[str, dict] = {}
    for e in entries:
        if (e.get("aoid") or "").lower() != needle:
            continue
        if e.get("type") not in {"op", "clause", "built-in function", "concrete method"}:
            continue
        ref_id = e.get("refId") or e.get("id") or ""
        if not ref_id:
            continue
        prev = by_ref.get(ref_id)
        if prev is None or (prev.get("type") == "clause" and e.get("type") != "clause"):
            by_ref[ref_id] = e
    if not by_ref:
        return None
    rows = [(ref_id, clauses.get(ref_id)) for ref_id in by_ref]
    rows.sort(key=lambda r: sec_number_key((r[1] or {}).get("number", "")))
    ref_id, clause = rows[0]
    if not clause:
        return None
    return (clause.get("number", ""), clause.get("title", ""), f"{location}#{ref_id}")
