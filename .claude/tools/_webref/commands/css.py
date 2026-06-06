"""`css` subcommand — CSS property / @rule / selector / value metadata."""
from __future__ import annotations

import argparse
import json
import sys

from ..sources.webref_data import fetch_data_json

# Display order for CSS property fields — most-relevant first; trailing href.
_CSS_FIELDS = (
    "value", "newValues", "initial", "inherited", "appliesTo",
    "percentages", "computedValue", "canonicalOrder",
    "animationType", "logicalPropertyGroup", "media",
    "styleDeclaration", "descriptors", "syntax", "href",
)
_CSS_SECTIONS = (
    ("properties", "Property"),
    ("atrules", "At-rule"),
    ("selectors", "Selector"),
    ("values", "Value"),
)


def cmd_css(args: argparse.Namespace) -> None:
    data = fetch_data_json("css", args.shortname)
    found = False
    for key, label in _CSS_SECTIONS:
        for item in data.get(key, []):
            if item.get("name") != args.name:
                continue
            found = True
            print(f"  {label}: {item.get('name','?')}")
            for k in _CSS_FIELDS:
                if k in item:
                    v = item[k]
                    if isinstance(v, (list, dict)):
                        v = json.dumps(v, ensure_ascii=False)[:200]
                    print(f"    {k:18} {v}")
            print()
    if not found:
        title = data.get("spec", {}).get("title", args.shortname)
        prop_names = sorted(p.get("name", "") for p in data.get("properties", []))
        print(f"webref: {args.name!r} not found in {title}", file=sys.stderr)
        if prop_names:
            near = [n for n in prop_names
                    if args.name.lower() in n.lower() or n.lower() in args.name.lower()][:8]
            if near:
                print(f"  near property matches: {', '.join(near)}", file=sys.stderr)
            else:
                preview = ", ".join(prop_names[:10])
                print(f"  available properties (first 10 of {len(prop_names)}): {preview}",
                      file=sys.stderr)
        sys.exit(1)
