"""w3c/webref machine-readable extracts of WHATWG / W3C specs."""
from __future__ import annotations

import json
import sys

from ..cache import NotFound, cached_fetch_url

BASE = "https://raw.githubusercontent.com/w3c/webref/main/ed"

# Data-kind → file extension. Every webref per-spec extract is JSON except the
# raw WebIDL dump (`idl/<spec>.idl`).
_DATA_EXT = {
    "headings": "json",
    "dfns": "json",
    "ids": "json",
    "css": "json",
    "elements": "json",
    "idl": "idl",
}


def fetch_bytes(path: str) -> bytes:
    """GET BASE/path via cache. Raises NotFound on 404, exits on other errors."""
    return cached_fetch_url(f"{BASE}/{path}")


def fetch_json(path: str) -> dict:
    return json.loads(fetch_bytes(path).decode("utf-8"))


# Process-local memo of the webref catalog (`ed/index.json`). The catalog is
# ~1.5 MB / 700+ specs, so we fetch it lazily — only when a direct data-file
# fetch misses (see `fetch_data`) — and parse it once per invocation.
_INDEX: dict[str, dict] | None = None


def _data_index() -> dict[str, dict]:
    """Memoized map: spec-or-series shortname → webref spec result entry.

    Built from webref's `ed/index.json`, the authoritative catalog that
    declares, per spec, the exact data-file path for each kind (`headings`,
    `dfns`, `idl`, `css`, `ids`, `elements`) AND the series→current-spec
    relationship. We index every spec by its own shortname, then add each
    series shortname pointing at its current specification — without
    shadowing a real spec of the same name.

    This collapses two naming conventions onto one upstream source of truth,
    replacing the old strip-trailing-`-N` heuristic:

      - leveled naming: spec `geometry-1` publishes `idl/geometry.idl`
        (version-less) but `headings/geometry-1.json` (leveled).
      - series shortname: the convenient `webcrypto` is a *series*, whose
        current spec is `webcrypto-2` (so `headings/webcrypto-2.json`,
        yet `idl/webcrypto.idl`).

    No hand-maintained alias map — the catalog answers both.
    """
    global _INDEX
    if _INDEX is None:
        results = fetch_json("index.json").get("results", [])
        idx: dict[str, dict] = {}
        for r in results:
            sn = r.get("shortname")
            if sn:
                idx.setdefault(sn, r)
        # Second pass so a series shortname never shadows a spec of the same
        # name registered above (e.g. `dom` is both its own spec and series).
        for r in results:
            series = r.get("series") or {}
            ssn = series.get("shortname")
            cur = series.get("currentSpecification")
            if ssn and ssn not in idx and cur and cur in idx:
                idx[ssn] = idx[cur]
        _INDEX = idx
    return _INDEX


def try_fetch_data(kind: str, shortname: str) -> bytes | None:
    """Fetch webref `<kind>` data for `shortname`, or None if it doesn't exist.

    `kind` ∈ {headings, dfns, idl, css, ids, elements}. Tries the direct
    `<kind>/<shortname>.<ext>` path first — the zero-cost common case where
    the user-facing shortname IS the data-file name (`html`, `dom`,
    `css-overflow-3`, …). On 404, consults webref's catalog (series→current
    spec + leveled-naming map) for the authoritative data-file path and
    retries. Returns None when neither resolves — for callers (the body
    resolver) that chain headings→dfns and treat a missing extract as
    "look elsewhere" rather than a hard error.
    """
    ext = _DATA_EXT.get(kind, "json")
    direct = f"{kind}/{shortname}.{ext}"
    try:
        return fetch_bytes(direct)
    except NotFound:
        pass

    entry = _data_index().get(shortname)
    resolved = entry.get(kind) if entry else None
    if resolved and resolved != direct:
        try:
            return fetch_bytes(resolved)
        except NotFound:
            return None
    return None


def fetch_data(kind: str, shortname: str) -> bytes:
    """`try_fetch_data` + hard exit on miss. Use from command sites where a
    missing extract is a user-facing error (bad shortname / wrong spec)."""
    data = try_fetch_data(kind, shortname)
    if data is None:
        ext = _DATA_EXT.get(kind, "json")
        sys.exit(
            f"webref: no {kind} data for {shortname!r} "
            f"(tried {BASE}/{kind}/{shortname}.{ext}; shortname not in webref "
            f"index, or that spec publishes no {kind} extract)"
        )
    return data


def fetch_data_json(kind: str, shortname: str) -> dict:
    """`fetch_data` + JSON decode. Use for every kind except `idl`."""
    return json.loads(fetch_data(kind, shortname).decode("utf-8"))


def try_fetch_data_json(kind: str, shortname: str) -> dict | None:
    """`try_fetch_data` + JSON decode; None when the extract doesn't exist."""
    raw = try_fetch_data(kind, shortname)
    return json.loads(raw.decode("utf-8")) if raw is not None else None
