"""`idl` subcommand — interface IDL fragment extraction."""
from __future__ import annotations

import argparse
import re
import sys

from ..sources.webref_data import fetch_data

# IDL block start: optional `partial`, then one of:
#   interface [mixin] / dictionary / enum / callback [interface] / namespace
# followed by whitespace. The interface-name match (\biface\b) is done separately
# so we can buffer leading [attr] lines until we know whether the block matches.
_IDL_DECL_RE = re.compile(
    r"^(?:partial\s+)?"
    r"(?:interface(?:\s+mixin)?|dictionary|enum|callback(?:\s+interface)?|namespace)"
    r"\s+"
)


def cmd_idl(args: argparse.Namespace) -> None:
    text = fetch_data("idl", args.shortname).decode("utf-8")
    iface_re = re.compile(rf"\b{re.escape(args.iface)}\b")

    out: list[str] = []
    buf: list[str] = []  # leading [attr] lines awaiting a decl line
    in_block = False

    for line in text.splitlines():
        if not in_block:
            # Buffer leading [attr] lines awaiting a decl. Use lstrip() so an
            # indented [attr] line between blocks still buffers (and is dropped
            # by the next non-decl line) rather than silently discarded.
            if line.lstrip().startswith("["):
                buf.append(line)
                continue
            if _IDL_DECL_RE.match(line):
                if iface_re.search(line):
                    out.extend(buf)
                    out.append(line)
                    in_block = True
                buf = []
                continue
            # Discard any buffered attrs not followed by a matching decl.
            buf = []
            continue
        # Inside a matching block — emit every line through `};`. Member-level
        # [CEReactions] / [SameObject] / etc. flow through here unconditionally
        # (must NOT be buffered, else they get dropped at the next non-[ line).
        out.append(line)
        if line.strip() == "};":
            in_block = False
            out.append("")

    if not out:
        sys.exit(f"webref: no IDL block matching {args.iface!r} in {args.shortname}")
    print("\n".join(out))
