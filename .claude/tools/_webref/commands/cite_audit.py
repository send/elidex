"""`cite-audit` subcommand — repo citation inventory + resolution.

An **adapter command** (DESIGN.md "Externalization Criteria": generic
behavior stays free of elidex-specific paths; elidex policy lives in
adapter commands). The generic half — resolving a §number to its title —
is `resolver.lookup_section`, untouched. This command adds the elidex
policy half: which tree to scan, and what a citation looks like in it.

Purpose: make citation **discovery** a checked-in detector rather than a
hand-authored pattern list. Three successive citation sweeps each shipped
a detector that could only find what its author already suspected:

  1. resolution-only-by-known-outcome  — "does `heading --exact` fail?"
  2. enumeration-only-by-known-pattern — a hand-written grep alternation
  3. triage-only-by-known-suspicion    — total enumeration, partial triage

The fix common to all three is that the *candidate set* must be derived,
and must carry enough context to classify each cite by concept. This
command emits exactly that: every distinct `§<section>` cited in the
scanned tree, its resolved title (or UNRESOLVED), and every citing line
with `file:line` plus the comment text around it.

Discovery is the tool's job; classification stays the author's.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

from ..resolver import lookup_section

# A citation is a section mark followed by a dotted number. Bare `§` with
# no digits (rare, e.g. prose "§ of the spec") is not a citation.
_CITE_RE = re.compile(r"§\s*(\d+(?:\.\d+)*)")

# Default scan root is elidex policy, not generic behavior.
_DEFAULT_ROOT = "crates"
_DEFAULT_GLOB = "*.rs"


def _iter_cites(root: Path, glob: str):
    """Yield (section, path, lineno, line_text) for every citation found."""
    for path in sorted(root.rglob(glob)):
        if "target" in path.parts:
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        if "§" not in text:
            continue
        for lineno, line in enumerate(text.splitlines(), start=1):
            for m in _CITE_RE.finditer(line):
                yield m.group(1), path, lineno, line.strip()


def cmd_cite_audit(args: argparse.Namespace) -> int:
    root = Path(args.root)
    if not root.is_dir():
        print(f"cite-audit: not a directory: {root}", file=sys.stderr)
        return 1

    by_section: dict[str, list[tuple[Path, int, str]]] = defaultdict(list)
    for section, path, lineno, line in _iter_cites(root, args.glob):
        by_section[section].append((path, lineno, line))

    if args.prefix:
        # Dotted-COMPONENT prefix, not string prefix: `4.10.2` must match
        # `4.10.2` and `4.10.2.7`, but NOT `4.10.20.3`. A bare
        # `str.startswith` silently widens the filter by an order of
        # magnitude — the same class of quiet over-match the `heading`
        # command guards with `--exact`.
        want = args.prefix.split(".")
        by_section = {
            s: v for s, v in by_section.items()
            if s.split(".")[: len(want)] == want
        }

    total_cites = sum(len(v) for v in by_section.values())
    # `lookup_section` returns (number, title, anchor) or None. It is
    # prefix-tolerant, so an EXACT match is required — otherwise `§4.13`
    # would silently pass because `§4.13.1` exists (the same drift-catch
    # invariant `heading --exact` enforces).
    resolved: dict[str, str | None] = {}
    for section in by_section:
        try:
            hit = lookup_section(args.spec, section)
        except Exception:  # noqa: BLE001 - resolver raises varied lookup errors
            hit = None
        resolved[section] = (
            hit[1] if hit and hit[0] == section else None
        )

    def _sortkey(s: str) -> tuple[int, ...]:
        return tuple(int(p) for p in s.split("."))

    sections = sorted(by_section, key=_sortkey)
    unresolved = [s for s in sections if resolved[s] is None]
    unresolved_cites = sum(len(by_section[s]) for s in unresolved)

    if args.format == "json":
        out = {
            "spec": args.spec,
            "root": str(root),
            "distinct_sections": len(sections),
            "total_cites": total_cites,
            "unresolved_sections": len(unresolved),
            "unresolved_cites": unresolved_cites,
            "sections": [
                {
                    "section": s,
                    "title": resolved[s],
                    "resolved": resolved[s] is not None,
                    "cite_count": len(by_section[s]),
                    "cites": [
                        {"file": str(p), "line": n, "text": t}
                        for p, n, t in by_section[s]
                    ],
                }
                for s in sections
            ],
        }
        json.dump(out, sys.stdout, indent=2)
        print()
        if args.strict and unresolved:
            sys.exit(1)
        return 0

    print(f"# cite-audit — spec={args.spec} root={root} glob={args.glob}"
          + (f" prefix=§{args.prefix}" if args.prefix else ""))
    print(f"# distinct sections: {len(sections)}   total cites: {total_cites}")
    print(f"# UNRESOLVED sections: {len(unresolved)}   "
          f"cites carrying them: {unresolved_cites}")
    print()
    for s in sections:
        title = resolved[s]
        label = title if title else "*** UNRESOLVED ***"
        print(f"§{s}  [{len(by_section[s])} cite(s)]  {label}")
        if not args.summary:
            for p, n, t in by_section[s]:
                try:
                    rel = p.relative_to(root.parent) if root.parent != Path(".") else p
                except ValueError:
                    rel = p
                print(f"    {rel}:{n}: {t[:150]}")
            print()
    # `cli.main` calls `args.func(args)` and DISCARDS the return value —
    # every command signals failure via `sys.exit`, not by returning. A
    # bare `return 1` here would be silently dropped, making `--strict`
    # a no-op. Follow the established convention.
    if args.strict and unresolved:
        sys.exit(1)
    return 0


def register(sub) -> None:
    """Attach the `cite-audit` subparser (called from cli.build_parser)."""
    ca = sub.add_parser(
        "cite-audit",
        help="repo citation inventory: every §number cited + title + sites",
        description=(
            "Scan a source tree for §<number> citations and resolve each "
            "against the spec. Emits the complete candidate set WITH the "
            "context needed to classify each cite by concept — so a "
            "citation sweep's discovery step is a detector, not a "
            "hand-authored pattern list."
        ),
    )
    ca.add_argument("spec", help="spec shortname (e.g. html, dom, ecma262)")
    ca.add_argument("--root", default=_DEFAULT_ROOT,
                    help=f"tree to scan (default: {_DEFAULT_ROOT})")
    ca.add_argument("--glob", default=_DEFAULT_GLOB,
                    help=f"file glob (default: {_DEFAULT_GLOB})")
    ca.add_argument("--prefix", default=None,
                    help="restrict to sections under this dotted prefix "
                         "(e.g. 4.10)")
    ca.add_argument("--summary", action="store_true",
                    help="section lines only, omit per-cite detail")
    ca.add_argument("--format", choices=("text", "json"), default="text",
                    help="output format (default: text)")
    ca.add_argument("--strict", action="store_true",
                    help="exit 1 if any cited section fails to resolve")
    ca.set_defaults(func=cmd_cite_audit)
