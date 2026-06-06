"""Anchor → fetch-URL resolvers + section/AO lookup helpers.

Two roles, both keyed on (shortname, anchor-or-name):

  - `resolve_body_source` / `tc39_resolve_body_url` / `webref_resolve_body_url`
    answer "where do I fetch the spec HTML that contains this anchor"
    (multipage chapter URL when possible, single-page fallback otherwise).
  - `lookup_section` / `lookup_heading` / `lookup_aoid_first` answer
    "what §number + title + anchor does this section/AO resolve to"
    (no fetch beyond biblio / headings JSON).
"""
from __future__ import annotations

import re
import sys

from .cache import NotFound
from .section_sort import heading_number_title, sec_number_key
from .sources.tc39 import TC39_FAMILY, tc39_biblio, tc39_clauses_by_id
from .sources.webref_data import try_fetch_data_json


# tc39 multipage chapter file derivation is purely structural: the top-level
# clause's `id` (always "sec-X" by convention) maps to `X.html`. Verified
# across all 38 ecma262 chapters + annexes — no per-spec carve-outs needed.
# If tc39 ever introduces an irregular chapter file, `cmd_body` catches the
# 404 from the multipage fetch and retries against the single-page URL
# (`<location>#<anchor>`), which serves every anchor regardless of chapter
# layout. See the NotFound-handling block at the end of cmd_body.


def tc39_resolve_body_url(shortname: str, anchor: str) -> tuple[str, str] | None:
    """Resolve tc39 (shortname, anchor) → (multipage_url, anchor) or None.

    Accepts ANY biblio-known id, not just `type=clause`. Non-clause entries
    (`table-*`, `figure-*`, `prod-*`, `step-*`, `term-*`, etc.) carry a
    `refId` pointing into their enclosing clause; we walk that chain up to
    the first ancestor clause and derive the chapter file from its
    top-level §-number. This way every anchor biblio tracks resolves —
    the resolver invariant "biblio-known id → multipage URL" holds by
    construction, rather than being limited to a single entry-type subset.

    Error contract (split deliberately so callers and main()'s NotFound
    handler can surface the right diagnostic):

      - `tuple[str, str]` — anchor resolved to a URL
      - `None`            — anchor not present in biblio (typo / unknown)
      - raises `NotFound` — biblio fetch itself failed (upstream/CDN
        outage, URL drift); reserved by intent — do NOT swallow here,
        or downstream callers see a misleading "anchor not found" path.
    """
    d = tc39_biblio(shortname)
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


def resolve_body_source(shortname: str, anchor: str) -> tuple[str, str]:
    """Resolve (shortname, anchor) → (fetch_url, anchor).

    Order:
      1. tc39 multipage (deterministic chapter derivation from biblio)
      2. WHATWG/W3C multipage (href from webref headings/)
      3. tc39 single-page fallback (3 MB cached spec.html)

    Single-page non-tc39 fallback is intentionally NOT supported: webref
    `href` is universal across WHATWG/W3C specs that webref tracks, and
    untracked specs (zero `headings/<x>.json`) lack the citation metadata
    body extraction depends on anyway.
    """
    if shortname in TC39_FAMILY:
        mp = tc39_resolve_body_url(shortname, anchor)
        if mp is not None:
            return mp
        # tc39 single-page fallback (rare — would mean biblio is missing
        # the clause but the anchor is real, e.g. proposal-stage drafts).
        try:
            d = tc39_biblio(shortname)
        except NotFound:
            sys.exit(f"webref: cannot resolve body URL for {shortname}#{anchor}")
        location = d.get("location", "")
        if not location:
            sys.exit(f"webref: tc39 biblio for {shortname} missing location field")
        return (location, anchor)

    mp = webref_resolve_body_url(shortname, anchor)
    if mp is not None:
        return mp
    sys.exit(
        f"webref: no multipage URL known for {shortname}#{anchor} "
        f"(WHATWG/W3C: anchor must appear in headings/{shortname}.json "
        f"or dfns/{shortname}.json)"
    )


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
