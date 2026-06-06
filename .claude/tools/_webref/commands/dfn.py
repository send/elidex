"""`dfn` subcommand — concept dfn → §heading + anchor."""
from __future__ import annotations

import argparse
import sys

from ..section_sort import heading_number_title
from ..sources.webref_data import fetch_data_json


def cmd_dfn(args: argparse.Namespace) -> None:
    data = fetch_data_json("dfns", args.shortname)
    dfns = data.get("dfns", [])
    needle = args.term.lower()

    def texts(d: dict) -> list[str]:
        return [t.lower() for t in d.get("linkingText", []) + d.get("localLinkingText", [])]

    exact = [d for d in dfns if needle in texts(d)]
    hits = exact if exact else [d for d in dfns if any(needle in t for t in texts(d))]

    if not hits:
        sys.exit(f"webref: no dfn matching {args.term!r} in {args.shortname}")

    mode = "exact" if exact else "substring"
    print(f"  ({len(hits)} {mode} hit{'s' if len(hits) > 1 else ''})")
    for d in hits[:25]:
        lt = d.get("linkingText", ["?"])[0]
        ty = d.get("type", "?")
        fr = ",".join(d.get("for", [])) or "-"
        h = d.get("heading") or {}
        hn, ht = heading_number_title(h)
        print(f"  {lt!r} (type={ty}, for={fr})  →  §{hn or '?'} {ht or '?'}  #{d.get('id','?')}")
    if len(hits) > 25:
        print(f"  ... ({len(hits)-25} more, narrow the term)")
