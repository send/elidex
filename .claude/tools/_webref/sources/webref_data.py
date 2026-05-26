"""w3c/webref machine-readable extracts of WHATWG / W3C specs."""
from __future__ import annotations

import json
import re
import sys

from ..cache import NotFound, cached_fetch_url

BASE = "https://raw.githubusercontent.com/w3c/webref/main/ed"


def fetch_bytes(path: str) -> bytes:
    """GET BASE/path via cache. Raises NotFound on 404, exits on other errors."""
    return cached_fetch_url(f"{BASE}/{path}")


def fetch_json(path: str) -> dict:
    return json.loads(fetch_bytes(path).decode("utf-8"))


def fetch_with_level_fallback(template: str, shortname: str) -> bytes:
    """Fetch `template.format(shortname=...)`; on 404, strip trailing -N and retry.

    webref convention: per-level draft URLs (`headings/css-overflow-3.json`)
    coexist with version-less stable URLs (`idl/geometry.idl`, `css/css-flexbox.json`).
    Callers passing the level-suffixed shortname (matching the headings/ convention)
    still resolve idl/ and css/ via this fallback.
    """
    path = template.format(shortname=shortname)
    try:
        return fetch_bytes(path)
    except NotFound:
        stripped = re.sub(r"-\d+$", "", shortname)
        if stripped != shortname:
            path2 = template.format(shortname=stripped)
            try:
                return fetch_bytes(path2)
            except NotFound:
                sys.exit(f"webref: fetch failed (tried both {BASE}/{path} and {BASE}/{path2})")
        sys.exit(f"webref: fetch failed: {BASE}/{path}")
