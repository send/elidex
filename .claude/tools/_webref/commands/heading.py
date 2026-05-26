"""`heading` subcommand — §number → title + anchor."""
from __future__ import annotations

import argparse
import sys

from ..section_sort import sec_number_key
from ..sources.tc39 import TC39_FAMILY, tc39_biblio
from ..sources.webref_data import fetch_json


def cmd_heading(args: argparse.Namespace) -> None:
    if args.shortname in TC39_FAMILY:
        _cmd_heading_tc39(args)
        return
    data = fetch_json(f"headings/{args.shortname}.json")
    if args.exact:
        matches = [h for h in data.get("headings", []) if h.get("number", "") == args.prefix]
        if not matches:
            title = data.get("spec", {}).get("title", args.shortname)
            sys.exit(f"webref: no exact heading match for §{args.prefix} in {title}")
    else:
        matches = [h for h in data.get("headings", []) if h.get("number", "").startswith(args.prefix)]
        if not matches:
            title = data.get("spec", {}).get("title", args.shortname)
            sys.exit(f"webref: no headings under §{args.prefix} in {title}")
    for h in matches:
        n = h.get("number", "")
        t = h.get("title", "")
        i = h.get("id", "")
        print(f"  §{n:<14} {t:<60} #{i}")


def _cmd_heading_tc39(args: argparse.Namespace) -> None:
    d = tc39_biblio(args.shortname)
    location = d.get("location", "")
    if args.exact:
        hits = [
            e for e in d.get("entries", [])
            if e.get("type") == "clause" and e.get("number", "") == args.prefix
        ]
        if not hits:
            sys.exit(f"webref: no exact heading match for §{args.prefix} in {args.shortname}")
    else:
        hits = [
            e for e in d.get("entries", [])
            if e.get("type") == "clause" and e.get("number", "").startswith(args.prefix)
        ]
        if not hits:
            sys.exit(f"webref: no headings under §{args.prefix} in {args.shortname}")
    hits.sort(key=lambda e: sec_number_key(e.get("number", "")))
    for e in hits:
        n = e.get("number", "")
        t = e.get("title", "")
        i = e.get("id", "")
        print(f"  §{n:<14} {t:<60} {location}#{i}")
