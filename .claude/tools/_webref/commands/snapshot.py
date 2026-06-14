"""`snapshot` subcommand — emit normalized semantic inventory JSON."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from ..inventory import build_inventory


def cmd_snapshot(args: argparse.Namespace) -> None:
    snapshot = build_inventory(args.shortname)
    text = json.dumps(snapshot, ensure_ascii=False, indent=2, sort_keys=True)
    if args.output:
        Path(args.output).write_text(text + "\n", encoding="utf-8")
        return
    print(text)
