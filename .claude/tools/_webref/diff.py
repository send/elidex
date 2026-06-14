"""Semantic diffing for webref inventory snapshots."""
from __future__ import annotations

from typing import Any


def diff_inventories(old: dict[str, Any], new: dict[str, Any]) -> dict[str, Any]:
    """Return a categorized semantic diff between two inventory snapshots."""
    old_items = _index_items(old)
    new_items = _index_items(new)

    added = [new_items[k] for k in sorted(new_items.keys() - old_items.keys())]
    removed = [old_items[k] for k in sorted(old_items.keys() - new_items.keys())]
    renumbered: list[dict[str, Any]] = []
    retitled: list[dict[str, Any]] = []
    moved: list[dict[str, Any]] = []
    changed: list[dict[str, Any]] = []

    for key in sorted(old_items.keys() & new_items.keys()):
        before = old_items[key]
        after = new_items[key]
        field_changes = _field_changes(before, after)
        if not field_changes:
            continue
        kinds = {c["field"] for c in field_changes}
        entry = {
            "key": key,
            "kind": after.get("kind", before.get("kind", "")),
            "id": after.get("id", before.get("id", "")),
            "changes": field_changes,
            "before": before,
            "after": after,
        }
        if entry["kind"] == "heading" and "number" in kinds:
            renumbered.append(entry)
        if entry["kind"] == "heading" and "title" in kinds:
            retitled.append(entry)
        if "href" in kinds:
            moved.append(entry)
        residual = [
            c for c in field_changes
            if c["field"] not in {"number", "title", "href", "generatedAt"}
        ]
        if residual:
            changed.append({**entry, "changes": residual})

    return {
        "schemaVersion": 1,
        "old": _summary(old),
        "new": _summary(new),
        "counts": {
            "added": len(added),
            "removed": len(removed),
            "renumbered": len(renumbered),
            "retitled": len(retitled),
            "moved": len(moved),
            "changed": len(changed),
        },
        "added": added,
        "removed": removed,
        "renumbered": renumbered,
        "retitled": retitled,
        "moved": moved,
        "changed": changed,
    }


def _index_items(snapshot: dict[str, Any]) -> dict[str, dict[str, Any]]:
    items = snapshot.get("items", [])
    out: dict[str, dict[str, Any]] = {}
    if not isinstance(items, list):
        return out
    for item in items:
        if not isinstance(item, dict):
            continue
        key = item.get("key")
        if isinstance(key, str) and key:
            out[key] = item
    return out


def _field_changes(before: dict[str, Any], after: dict[str, Any]) -> list[dict[str, Any]]:
    skip = {"key"}
    changes = []
    for field in sorted((before.keys() | after.keys()) - skip):
        old_value = before.get(field)
        new_value = after.get(field)
        if old_value != new_value:
            changes.append({"field": field, "old": old_value, "new": new_value})
    return changes


def _summary(snapshot: dict[str, Any]) -> dict[str, Any]:
    return {
        "shortname": snapshot.get("shortname"),
        "family": snapshot.get("family"),
        "generatedAt": snapshot.get("generatedAt"),
        "itemCount": snapshot.get("itemCount", len(snapshot.get("items", []))),
    }
