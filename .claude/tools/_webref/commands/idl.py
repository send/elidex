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


def _scan_attr_line(line: str, depth: int) -> tuple[int, str | None]:
    """Advance the bracket depth over `line`, ignoring string literals.

    Returns (new_depth, rest) where `rest` is the text after the `]` that
    balanced the list on this line (None when the list is still open or did
    not close here), so `[Exposed=Window] interface X {` yields rest
    ` interface X {`.
    """
    in_str = False
    last_close: int | None = None
    for i, ch in enumerate(line):
        if ch == '"':
            in_str = not in_str
        elif not in_str:
            if ch == "[":
                depth += 1
            elif ch == "]":
                depth = max(depth - 1, 0)
                if depth == 0:
                    last_close = i
    # Keep scanning past a balanced list so `[A] [B,` leaves `[B` open; the
    # remainder is whatever follows the last list that closed on this line.
    if depth == 0 and last_close is not None:
        return 0, line[last_close + 1:]
    return depth, None


def extract_idl_blocks(text: str, iface: str) -> list[str]:
    """Return the output lines (leading extended attributes + block) for every
    top-level block whose declaration line names `iface`.

    Extended-attribute lists are accumulated by bracket depth, from the line
    that opens `[` until the line where it is balanced again, so a list split
    over several lines (`[Exposed=Window,\n LegacyOverrideBuiltIns]`) is kept
    whole, and a declaration on the closing line is recognised. Attributes of a non-matching block never leak into the next block.
    """
    # The declared name only: the same keyword prefix as `_IDL_DECL_RE`
    # (one home), then `iface`. `interface Document : Node {` is not `Node`.
    # `typedef` is not a block this subcommand extracts (not in the prefix).
    iface_re = re.compile(_IDL_DECL_RE.pattern + re.escape(iface) + r"\b")

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
                depth, rest = _scan_attr_line(line, depth)
                # Same-line form `[Exposed=Window] interface X {`: once the
                # list closes, the remainder may itself be the declaration.
                # None while the list is still open.
                decl_text = rest.lstrip() if rest is not None else None
                if decl_text is None or not _IDL_DECL_RE.match(decl_text):
                    buf.append(line)
                    continue
            else:
                decl_text = line
            if _IDL_DECL_RE.match(decl_text):
                if iface_re.match(decl_text):
                    out.extend(buf)
                    out.append(line)
                    # One-line declaration (`enum E { "a" };`, `callback C
                    # = ...;`): the block ends on the declaration line.
                    if decl_text.rstrip().endswith(";"):
                        out.append("")
                    else:
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
