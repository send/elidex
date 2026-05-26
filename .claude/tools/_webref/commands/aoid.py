"""`aoid` subcommand — tc39 abstract operation aoid → §number + anchor."""
from __future__ import annotations

import argparse
import sys

from ..section_sort import sec_number_key
from ..sources.tc39 import TC39_FAMILY, tc39_biblio, tc39_clauses_by_id


def cmd_aoid(args: argparse.Namespace) -> None:
    if args.shortname not in TC39_FAMILY:
        sys.exit(f"webref: aoid is tc39-only (got shortname {args.shortname!r}); "
                 f"try ecma262 / ecma402")
    d = tc39_biblio(args.shortname)
    entries = d.get("entries", [])
    clauses = tc39_clauses_by_id(entries)
    location = d.get("location", "")
    needle = args.name.lower()
    # Entries that carry `aoid` directly: ops (abstract operations / SDOs /
    # numeric methods / host-defined), built-in functions, concrete methods,
    # and AO-defining clauses. Filter by aoid match, then cross-ref to clause
    # for §number + rendered title.
    # Same AO often appears twice — once as `type=op` (carrying `kind` and
    # `signature`) and once as `type=clause` (carrying `title` and `number`).
    # Group by anchor and prefer the op-style entry (more metadata), then fall
    # back to the clause.
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
        sys.exit(f"webref: no AO with aoid={args.name!r} in {args.shortname}")

    rows = [
        (ref_id, e, clauses.get(ref_id))
        for ref_id, e in by_ref.items()
    ]
    rows.sort(key=lambda r: sec_number_key((r[2] or {}).get("number", "")))

    for ref_id, e, clause in rows:
        aoid = e.get("aoid", "")
        kind = e.get("kind") or e.get("type") or "?"
        if clause:
            n = clause.get("number", "?")
            t = clause.get("title", "?")
            print(f"  {aoid:<28} §{n:<10} {t:<55} ({kind})  {location}#{ref_id}")
        else:
            print(f"  {aoid:<28} ({kind})  {location}#{ref_id}")
