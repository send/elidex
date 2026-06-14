"""`refresh` subcommand — capture a new snapshot and compare with previous."""
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from ..diff import diff_inventories
from ..inventory import build_inventory


def cmd_refresh(args: argparse.Namespace) -> None:
    snapshot_dir = _snapshot_dir(args.snapshot_dir)
    snapshot_dir.mkdir(parents=True, exist_ok=True)
    latest = snapshot_dir / f"{args.shortname}-latest.json"
    previous_path = _previous_snapshot_path(snapshot_dir, args.shortname, latest)
    previous = _read_json(latest) if latest.exists() else None
    current = build_inventory(args.shortname)
    out = _unique_snapshot_path(snapshot_dir, args.shortname)
    text = json.dumps(current, ensure_ascii=False, indent=2, sort_keys=True)
    out.write_text(text + "\n", encoding="utf-8")
    latest.write_text(text + "\n", encoding="utf-8")

    print(f"snapshot: {out}")
    print(f"latest:   {latest}")
    if previous is None:
        print("previous: none")
        print("next: rerun refresh after a future upstream update to get a semantic diff")
        return
    if previous_path is None:
        previous_path = _unique_snapshot_path(snapshot_dir, f"{args.shortname}-previous")
        previous_path.write_text(
            json.dumps(previous, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    print(f"previous: {previous_path}")

    result = diff_inventories(previous, current)
    counts = result["counts"]
    print(
        "diff:     "
        f"added={counts['added']} removed={counts['removed']} "
        f"renumbered={counts['renumbered']} retitled={counts['retitled']} "
        f"moved={counts['moved']} changed={counts['changed']}"
    )
    if any(counts.values()):
        print(
            "next:     "
            f".claude/tools/webref agent-brief {previous_path} {out} "
            "--paths docs crates CLAUDE.md"
        )
    else:
        print("next:     no semantic drift detected")


def _snapshot_dir(value: str | None) -> Path:
    if value:
        return Path(value).expanduser()
    return Path.home() / ".cache" / "elidex-webref" / "snapshots"


def _read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text("utf-8"))


def _previous_snapshot_path(snapshot_dir: Path, shortname: str, latest: Path) -> Path | None:
    candidates = [
        p for p in snapshot_dir.glob(f"{shortname}-*.json")
        if p != latest and not p.name.startswith(f"{shortname}-previous-")
    ]
    return max(candidates, default=None)


def _unique_snapshot_path(snapshot_dir: Path, stem: str) -> Path:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    candidate = snapshot_dir / f"{stem}-{stamp}.json"
    if not candidate.exists():
        return candidate
    suffix = 1
    while True:
        candidate = snapshot_dir / f"{stem}-{stamp}-{suffix}.json"
        if not candidate.exists():
            return candidate
        suffix += 1
