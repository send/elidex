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


def _bracket_delta(line: str) -> int:
    """Net `[` minus `]` on `line`, ignoring string literals ("...")."""
    delta = 0
    in_str = False
    for ch in line:
        if ch == '"':
            in_str = not in_str
        elif not in_str:
            if ch == "[":
                delta += 1
            elif ch == "]":
                delta -= 1
    return delta


def extract_idl_blocks(text: str, iface: str) -> list[str]:
    """Return the output lines (leading extended attributes + block) for every
    top-level block whose declaration line names `iface`.

    Extended-attribute lists are accumulated by bracket depth, from the line
    that opens `[` until the line where it is balanced again, so a list split
    over several lines (`[Exposed=Window,\n LegacyOverrideBuiltIns]`) is kept
    whole. Attributes of a non-matching block never leak into the next block.
    """
    iface_re = re.compile(rf"\b{re.escape(iface)}\b")

    out: list[str] = []
    buf: list[str] = []  # leading [attr] lines awaiting a decl line
    depth = 0  # open `[` count of the attribute list being accumulated
    in_block = False

    for line in text.splitlines():
        if not in_block:
            # Accumulate an extended-attribute list: it starts at a line whose
            # first non-blank char is `[` and lasts until brackets balance.
            # lstrip() so an indented list between blocks still buffers (and is
            # dropped by the next non-decl line) rather than silently discarded.
            if depth > 0 or line.lstrip().startswith("["):
                buf.append(line)
                depth = max(depth + _bracket_delta(line), 0)
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
    return out


def cmd_idl(args: argparse.Namespace) -> None:
    text = fetch_data("idl", args.shortname).decode("utf-8")
    out = extract_idl_blocks(text, args.iface)
    if not out:
        sys.exit(f"webref: no IDL block matching {args.iface!r} in {args.shortname}")
    print("\n".join(out))
