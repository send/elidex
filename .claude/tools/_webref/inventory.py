"""Semantic inventory snapshots for webref data.

The HTTP cache stores bytes; this module stores the stable facts Coding Agents
care about when specs drift: headings, definitions, and TC39 abstract operation
links. Keep this layer free of elidex-specific paths so it can move out of the
repo later.
"""
from __future__ import annotations

from datetime import datetime, timezone
from typing import Any

from .section_sort import heading_number_title, sec_number_key
from .sources.tc39 import TC39_FAMILY, tc39_biblio, tc39_clauses_by_id
from .sources.webref_data import fetch_data_json, try_fetch_data_json

SCHEMA_VERSION = 1


def build_inventory(shortname: str) -> dict[str, Any]:
    """Build a normalized semantic snapshot for `shortname`."""
    if shortname in TC39_FAMILY:
        return _build_tc39_inventory(shortname)
    return _build_webref_inventory(shortname)


def _build_webref_inventory(shortname: str) -> dict[str, Any]:
    headings_data = fetch_data_json("headings", shortname)
    spec = headings_data.get("spec", {})
    items: list[dict[str, Any]] = []

    for h in headings_data.get("headings", []):
        number, title = heading_number_title(h)
        ident = _str(h.get("id"))
        if not ident:
            continue
        items.append(_clean({
            "key": f"heading:{ident}",
            "kind": "heading",
            "id": ident,
            "number": number,
            "title": title,
            "href": _str(h.get("href")),
        }))

    dfns_data = try_fetch_data_json("dfns", shortname)
    if dfns_data is not None:
        for d in dfns_data.get("dfns", []):
            ident = _str(d.get("id"))
            if not ident:
                continue
            heading = d.get("heading") or {}
            heading_number, heading_title = heading_number_title(heading)
            items.append(_clean({
                "key": f"dfn:{ident}",
                "kind": "dfn",
                "id": ident,
                "type": _str(d.get("type")),
                "for": _sorted_strs(d.get("for", [])),
                "linkingText": _sorted_strs(d.get("linkingText", [])),
                "localLinkingText": _sorted_strs(d.get("localLinkingText", [])),
                "headingNumber": heading_number,
                "headingTitle": heading_title,
                "href": _str(d.get("href")),
            }))

    items.sort(key=_item_sort_key)
    return {
        "schemaVersion": SCHEMA_VERSION,
        "generatedAt": _now_iso(),
        "shortname": shortname,
        "family": "webref",
        "spec": _clean({
            "title": _str(spec.get("title")),
            "url": _str(spec.get("url")),
        }),
        "itemCount": len(items),
        "items": items,
    }


def _build_tc39_inventory(shortname: str) -> dict[str, Any]:
    data = tc39_biblio(shortname)
    entries = data.get("entries", [])
    clauses = tc39_clauses_by_id(entries)
    location = _str(data.get("location"))
    items: list[dict[str, Any]] = []

    for e in entries:
        if e.get("type") != "clause":
            continue
        ident = _str(e.get("id"))
        if not ident:
            continue
        number = _str(e.get("number"))
        items.append(_clean({
            "key": f"heading:{ident}",
            "kind": "heading",
            "id": ident,
            "number": number,
            "title": _str(e.get("title")),
            "aoid": _str(e.get("aoid")),
            "href": f"{location}#{ident}" if location else f"#{ident}",
        }))

    ao_items: dict[tuple[str, str], dict[str, Any]] = {}
    for e in entries:
        aoid = _str(e.get("aoid"))
        if not aoid:
            continue
        ref_id = _tc39_ref_id(e)
        clause = clauses.get(ref_id, {})
        ident = _str(e.get("id")) or ref_id or aoid
        key = (aoid, ref_id or ident)
        item = ao_items.setdefault(key, {
            "key": f"aoid:{aoid}:{ref_id or ident}",
            "kind": "aoid",
            "id": ident,
            "aoid": aoid,
            "refId": ref_id,
            "sectionNumber": _str(clause.get("number")),
            "sectionTitle": _str(clause.get("title")),
            "href": f"{location}#{ref_id}" if location and ref_id else "",
        })
        entry_type = _str(e.get("type"))
        if entry_type:
            item.setdefault("entryTypes", [])
            item["entryTypes"].append(entry_type)
        signature = _str(e.get("signature"))
        if signature:
            item.setdefault("signatures", [])
            item["signatures"].append(signature)

    for item in ao_items.values():
        for field in ("entryTypes", "signatures"):
            if field in item:
                item[field] = sorted(set(item[field]))
        items.append(_clean(item))

    items.sort(key=_item_sort_key)
    return {
        "schemaVersion": SCHEMA_VERSION,
        "generatedAt": _now_iso(),
        "shortname": shortname,
        "family": "tc39",
        "spec": _clean({
            "title": _str(data.get("title")) or shortname,
            "url": location,
        }),
        "itemCount": len(items),
        "items": items,
    }


def _item_sort_key(item: dict[str, Any]) -> tuple[str, tuple[int, ...], str, str]:
    number = item.get("number") or item.get("sectionNumber") or ""
    return (
        str(item.get("kind", "")),
        sec_number_key(str(number)),
        str(item.get("key", "")),
        str(item.get("title", "")),
    )


def _tc39_ref_id(entry: dict[str, Any]) -> str:
    return _str(entry.get("refId")) or _str(entry.get("id"))


def _clean(value: dict[str, Any]) -> dict[str, Any]:
    return {k: v for k, v in value.items() if v not in ("", [], {}, None)}


def _now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _str(value: Any) -> str:
    return value if isinstance(value, str) else ""


def _sorted_strs(values: Any) -> list[str]:
    if not isinstance(values, list):
        return []
    return sorted(v for v in values if isinstance(v, str))
