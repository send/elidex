"""TC39 biblio (machine-readable spec internals published by tc39).

tc39 ships `@tc39/<spec>-biblio` on npm; jsdelivr CDN serves the single
biblio.json file with proper ETag headers, no tarball extraction needed.
"""
from __future__ import annotations

import json

from ..cache import cached_fetch_url

TC39_FAMILY = {"ecma262", "ecma402"}


def tc39_biblio(shortname: str) -> dict:
    """Return parsed biblio.json for `shortname` (ecma262 / ecma402).

    biblio.json shape: `{"location": "https://tc39.es/...", "entries": [...]}`.
    Each entry has `type` ∈ {clause, op, built-in function, term, production,
    table, step, concrete method, figure, note} plus per-type fields:
      - clause:           {id, number, title, titleHTML, [aoid]}
      - op:               {aoid, refId, kind, signature, ...}
      - built-in function:{aoid, refId, ...}
      - term:             {term, refId, ...}
    Cross-refs use `refId` → clause `id`.

    Pinned to the `@2` major tag rather than `@latest` so a hypothetical future
    schema break doesn't silently corrupt audits — the helper degrades visibly
    (new fields missing) and the bump becomes a deliberate update.
    """
    url = f"https://cdn.jsdelivr.net/npm/@tc39/{shortname}-biblio@2/biblio.json"
    return json.loads(cached_fetch_url(url).decode("utf-8"))


def tc39_clauses_by_id(entries: list[dict]) -> dict[str, dict]:
    return {e["id"]: e for e in entries if e.get("type") == "clause" and e.get("id")}
