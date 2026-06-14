"""`diff` subcommand — compare two semantic inventory snapshots."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from ..diff import diff_inventories


def cmd_diff(args: argparse.Namespace) -> None:
    old = _read_snapshot(args.old)
    new = _read_snapshot(args.new)
    result = diff_inventories(old, new)
    if args.format == "json":
        print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
        return
    _print_text(result)


def _read_snapshot(path: str) -> dict[str, Any]:
    return json.loads(Path(path).read_text("utf-8"))


def _print_text(result: dict[str, Any]) -> None:
    old = result.get("old", {})
    new = result.get("new", {})
    print(
        f"{old.get('shortname', '?')} "
        f"({old.get('itemCount', '?')} items) → "
        f"{new.get('shortname', '?')} ({new.get('itemCount', '?')} items)"
    )
    counts = result.get("counts", {})
    print(
        "added={added} removed={removed} renumbered={renumbered} "
        "retitled={retitled} moved={moved} changed={changed}".format(**counts)
    )
    for section in ("added", "removed", "renumbered", "retitled", "moved", "changed"):
        entries = result.get(section, [])
        if not entries:
            continue
        print()
        print(f"## {section}")
        for entry in entries[:50]:
            _print_entry(section, entry)
        if len(entries) > 50:
            print(f"  ... {len(entries) - 50} more")


def _print_entry(section: str, entry: dict[str, Any]) -> None:
    if section in {"added", "removed"}:
        label = (
            entry.get("title")
            or entry.get("aoid")
            or entry.get("linkingText")
            or entry.get("id")
        )
        number = entry.get("number") or entry.get("sectionNumber") or "?"
        print(f"- {entry.get('kind', '?')} {entry.get('key', '?')} §{number} {label}")
        return
    print(f"- {entry.get('kind', '?')} {entry.get('key', '?')}")
    for change in entry.get("changes", []):
        print(
            f"    {change.get('field')}: "
            f"{change.get('old')!r} → {change.get('new')!r}"
        )
