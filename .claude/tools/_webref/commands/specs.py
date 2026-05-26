"""`specs` subcommand — shortname lookup by spec title."""
from __future__ import annotations

import argparse

from ..sources.webref_data import fetch_json


def cmd_specs(args: argparse.Namespace) -> None:
    data = fetch_json("index.json")
    kw = args.keyword.lower()
    for r in data.get("results", []):
        title = r.get("title", "")
        short = r.get("shortname", "")
        if kw in title.lower() or kw in short.lower():
            print(f"  {short:<35} {title}")
