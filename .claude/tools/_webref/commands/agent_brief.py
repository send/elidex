"""`agent-brief` subcommand — turn semantic drift into an elidex work queue."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from ..diff import diff_inventories

_TEXT_SUFFIXES = {
    ".md", ".rs", ".toml", ".yml", ".yaml", ".json", ".txt", ".py", ".sh",
}


def cmd_agent_brief(args: argparse.Namespace) -> None:
    old = _read_json(args.old)
    new = _read_json(args.new)
    result = diff_inventories(old, new)
    impacts = _scan_impacts(Path(args.repo_root).resolve(), args.paths, result)
    brief = {
        "schemaVersion": 1,
        "old": result["old"],
        "new": result["new"],
        "counts": result["counts"],
        "impacts": impacts,
    }
    if args.format == "json":
        print(json.dumps(brief, ensure_ascii=False, indent=2, sort_keys=True))
        return
    _print_markdown(brief)


def _read_json(path: str) -> dict[str, Any]:
    return json.loads(Path(path).read_text("utf-8"))


def _scan_impacts(
    repo_root: Path,
    paths: list[str],
    diff: dict[str, Any],
) -> list[dict[str, Any]]:
    candidates = _candidate_files(repo_root, paths)
    entries = _changed_entries(diff)
    impacts: list[dict[str, Any]] = []
    for entry in entries:
        needles = _needles_for_entry(entry)
        if not needles:
            continue
        matches = []
        for path in candidates:
            text = _read_text(path)
            if text is None:
                continue
            for line_no, line in enumerate(text.splitlines(), start=1):
                hits = sorted(n for n in needles if n and n in line)
                if hits:
                    matches.append({
                        "path": str(path.relative_to(repo_root)),
                        "line": line_no,
                        "needles": hits,
                        "text": line.strip()[:240],
                    })
        if matches:
            impacts.append({
                "key": entry.get("key"),
                "kind": entry.get("kind"),
                "id": entry.get("id"),
                "summary": _entry_summary(entry),
                "matches": matches,
                "truncated": len(matches) > 50,
            })
    return impacts


def _candidate_files(repo_root: Path, paths: list[str]) -> list[Path]:
    out: list[Path] = []
    for raw in paths:
        root = (repo_root / raw).resolve()
        if not _within(root, repo_root.resolve()) or not root.exists():
            continue
        if root.is_file():
            if root.suffix in _TEXT_SUFFIXES:
                out.append(root)
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix in _TEXT_SUFFIXES:
                out.append(path)
    return sorted(set(out))


def _within(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _read_text(path: Path) -> str | None:
    try:
        return path.read_text("utf-8")
    except (UnicodeDecodeError, OSError):
        return None


def _changed_entries(diff: dict[str, Any]) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for section in ("added", "removed"):
        for item in diff.get(section, []):
            if isinstance(item, dict) and _mark_seen(seen, item):
                entries.append(item)
    for section in ("renumbered", "retitled", "moved", "changed"):
        for item in diff.get(section, []):
            if isinstance(item, dict) and _mark_seen(seen, item):
                entries.append(item)
    return entries


def _mark_seen(seen: set[str], entry: dict[str, Any]) -> bool:
    key = entry.get("key")
    if not isinstance(key, str):
        key = f"{entry.get('kind', '')}:{entry.get('id', '')}:{len(seen)}"
    if key in seen:
        return False
    seen.add(key)
    return True


def _needles_for_entry(entry: dict[str, Any]) -> list[str]:
    needles: set[str] = set()
    for nested in ("before", "after"):
        value = entry.get(nested)
        if isinstance(value, dict):
            needles.update(_needles_for_entry(value))
    for key in ("id", "aoid", "title", "sectionTitle"):
        value = entry.get(key)
        if isinstance(value, str):
            needles.add(value)
            if key == "id":
                needles.add(f"#{value}")
    for key in ("number", "sectionNumber", "headingNumber"):
        value = entry.get(key)
        if isinstance(value, str):
            needles.add(f"§{value}")
            needles.add(f"§ {value}")
    for key in ("linkingText", "localLinkingText"):
        values = entry.get(key)
        if isinstance(values, list):
            needles.update(v for v in values if isinstance(v, str))
    href = entry.get("href")
    if isinstance(href, str) and "#" in href:
        frag = href.rsplit("#", 1)[-1]
        needles.add(frag)
        needles.add(f"#{frag}")
    return sorted(n for n in needles if len(n) >= 3)


def _entry_summary(entry: dict[str, Any]) -> str:
    after = entry.get("after")
    before = entry.get("before")
    if isinstance(after, dict):
        return _entry_summary(after)
    if isinstance(before, dict):
        return _entry_summary(before)
    label = (
        entry.get("title")
        or entry.get("aoid")
        or entry.get("sectionTitle")
        or entry.get("id")
        or entry.get("key")
    )
    number = entry.get("number") or entry.get("sectionNumber") or entry.get("headingNumber")
    if number:
        return f"§{number} {label}"
    return str(label)


def _print_markdown(brief: dict[str, Any]) -> None:
    old = brief["old"]
    new = brief["new"]
    counts = brief["counts"]
    impacts = brief["impacts"]
    print(f"# webref Agent Brief: {old.get('shortname')} → {new.get('shortname')}")
    print()
    print(
        "Diff counts: "
        f"added={counts['added']} removed={counts['removed']} "
        f"renumbered={counts['renumbered']} retitled={counts['retitled']} "
        f"moved={counts['moved']} changed={counts['changed']}"
    )
    print()
    if not impacts:
        print("No matching repository citations found in the scanned paths.")
        return
    print("## Impact Queue")
    for impact in impacts:
        print()
        print(f"### {impact['key']} — {impact['summary']}")
        visible_matches = impact["matches"][:50]
        for match in visible_matches:
            print(
                f"- `{match['path']}:{match['line']}` "
                f"matched {', '.join(f'`{n}`' for n in match['needles'])}"
            )
        omitted = len(impact["matches"]) - len(visible_matches)
        if omitted > 0:
            print(f"- ... {omitted} more matches omitted; rerun with `--format json`.")
