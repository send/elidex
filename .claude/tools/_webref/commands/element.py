"""`element` subcommand — HTML/SVG element → interface name + href."""
from __future__ import annotations

import argparse
import json
import sys

from ..sources.webref_data import fetch_data_json


def cmd_element(args: argparse.Namespace) -> None:
    data = fetch_data_json("elements", args.shortname)
    els = data.get("elements", [])
    hits = [e for e in els if e.get("name") == args.name]
    if not hits:
        names = sorted({e.get("name", "") for e in els})
        msg = (f"webref: element {args.name!r} not in {args.shortname} "
               f"(catalog has {len(names)} elements)")
        print(msg, file=sys.stderr)
        near = [n for n in names
                if args.name.lower() in n.lower() or n.lower() in args.name.lower()][:8]
        if near:
            print(f"  near matches: {', '.join(near)}", file=sys.stderr)
        sys.exit(1)
    for e in hits:
        print(f"  <{e.get('name','?')}>")
        for k, v in e.items():
            if k == "name":
                continue
            if isinstance(v, (list, dict)):
                v = json.dumps(v, ensure_ascii=False)
            print(f"    {k:20} {v}")
